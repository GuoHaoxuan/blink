use super::crc_check;
use crate::io::level_1b::SciFile;
use crate::types::HxmtHe;
use blink_core::types::MissionElapsedTime;

enum Pack {
    Event {
        ptime: u64,
        channel: u8,
        raw_bytes: [u8; 8],
    },
    Second {
        stime: u64,
        ptime: u64,
    },
    Error,
}

/// 每个 CCSDS 包的时间范围
struct PackInfo {
    min_time: f64,
    max_time: f64,
}



const PTIME_MOD: u64 = 1 << 19; // 524288
const _HALF_MOD: u64 = PTIME_MOD / 2;
const WRAP_PERIOD: f64 = PTIME_MOD as f64 * 2e-6; // 1.048576s

/// 1B→1K 经验时间校正 (秒)。
/// 通过 GRB 200415A 和 GRB 221009A 交叉验证确定。
const MET_CORRECTION: f64 = 4.0;

/// Floor-based per-event wrap computation.
///
/// 独立计算每个事例的 ptime 回绕次数，不依赖事例顺序。
/// 利用 utc_tail（包级 UTC 计数器）作为粗时间参考：
///   raw_delta = ptime - anchor_ptime（可为负）
///   raw_delta_seconds = raw_delta × 2μs
///   N = max(0, floor((utc_tail - anchor - raw_delta_seconds) / 1.048576))
///   total_ticks = N × PTIME_MOD + raw_delta
///   MET = anchor + total_ticks × 2μs + MET_CORRECTION
///
/// 解决 FIFO 溢出后事例时序混乱导致的错误回绕检测。
#[inline]
fn compute_met(ptime: u64, anchor_ptime: u64, anchor: f64, utc_tail: f64) -> f64 {
    let raw_delta = ptime as i64 - anchor_ptime as i64;
    let raw_delta_seconds = raw_delta as f64 * 2e-6;
    let n_wraps = ((utc_tail - anchor - WRAP_PERIOD - raw_delta_seconds) / WRAP_PERIOD)
        .floor()
        .max(0.0) as i64;
    let total_ticks = n_wraps * PTIME_MOD as i64 + raw_delta;
    let met = anchor + total_ticks as f64 * 2e-6 + MET_CORRECTION;
    if (met - 446726278.0).abs() < 0.005 {
        eprintln!("DEBUG_MET: utc_tail={}, anchor={:.6}, raw_delta_s={:.6}, n_wraps={}, final_met={:.6}", utc_tail, anchor, raw_delta_seconds, n_wraps, met);
    }
    met
}

/// 基于高水位线进行单向滞后补偿的 MET 计算。
/// 如果 utc_tail 发生滞后，原本稳定的 floor 计算会向下错退一个周期（1.048576s）。
/// 此时算出的 t_val 将极不合理地早于最近验证过的真实时间水位线。
/// 只有在这种“时空倒退”的铁证发生时（>0.8s），我们才赋予强制补偿，以消除污染。
#[inline]
fn compute_met_corrected(
    ptime: u64,
    anchor_ptime: u64,
    anchor: f64,
    utc_tail: f64,
    max_met_seen: &mut f64,
) -> f64 {
    let mut t_val = compute_met(ptime, anchor_ptime, anchor, utc_tail);
    if *max_met_seen > 0.0 && (*max_met_seen - t_val) > 0.8 {
        t_val += 1.048576; // 修正滞后的 -1 周期错误
    }
    if t_val > *max_met_seen {
        *max_met_seen = t_val;
    }
    t_val
}

/// 解析单个 CCSDS 包中所有事例
fn parse_events(ccsds: &[u8]) -> Vec<Pack> {
    let payload = &ccsds[6..878];
    let mut events = Vec::with_capacity(109);

    for chunk in payload.chunks_exact(8) {
        let mut row = [0u64; 8];
        let mut raw_bytes = [0u8; 8];
        for (i, byte) in chunk.iter().enumerate() {
            row[i] = *byte as u64;
            raw_bytes[i] = *byte;
        }

        let pack = if crc_check(&row) == row[7] & 0x0F {
            let ptime =
                ((row[4] & 1) << 18) + (row[5] << 10) + (row[6] << 2) + ((row[7] & 0xC0) >> 6);
            let channel = raw_bytes[0];
            match row[7] & 0x30 {
                0x00 | 0x20 => Pack::Event {
                    ptime,
                    channel,
                    raw_bytes,
                },
                0x10 => {
                    let stime = (row[0] << 24) + (row[1] << 16) + (row[2] << 8) + row[3];
                    Pack::Second { stime, ptime }
                }
                _ => Pack::Error,
            }
        } else {
            Pack::Error
        };
        events.push(pack);
    }
    events
}

fn get_utc_tail(ccsds: &[u8]) -> f64 {
    ccsds[878] as f64
        + (ccsds[879] as f64) * 256.0
        + (ccsds[880] as f64) * 65536.0
        + (ccsds[881] as f64) * 16777216.0
}

/// 本函数用单遍扫描进行安全重建。
pub fn reconstruct_with_wrap_tracking(sci_data: &SciFile, offset: f64) -> Vec<Vec<f64>> {
    let mut result = Vec::with_capacity(sci_data.ccsds.len());
    let mut met_anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;
    let mut max_met_seen: f64 = 0.0;

    for ccsds in sci_data.ccsds.iter() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        let mut times = Vec::with_capacity(events.len());

        for event in &events {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    // 仅当秒信号与包尾 UTC 时差在合理范围内（2秒数据窗口）才认为是有效锚点
                    if (met - utc_tail).abs() < 2.0 {
                        met_anchor = Some(met);
                        anchor_ptime = *ptime;
                        if met > max_met_seen {
                            max_met_seen = met;
                        }
                    }

                    if let Some(anchor) = met_anchor {
                        times.push(compute_met_corrected(
                            *ptime,
                            anchor_ptime,
                            anchor,
                            utc_tail,
                            &mut max_met_seen,
                        ));
                    }
                }
                Pack::Event { ptime, .. } => {
                    if let Some(anchor) = met_anchor {
                        times.push(compute_met_corrected(
                            *ptime,
                            anchor_ptime,
                            anchor,
                            utc_tail,
                            &mut max_met_seen,
                        ));
                    }
                }
                Pack::Error => {}
            }
        }
        result.push(times);
    }
    result
}

/// 提取所有秒事例（Second event）的重建 MET 时间。
pub fn extract_second_event_times(sci_data: &SciFile, offset: f64) -> Vec<f64> {
    let mut second_times: Vec<f64> = Vec::new();
    let mut met_anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;
    let mut prev_ptime: u64 = 0;
    let mut wrap_count: i64 = 0;

    for ccsds in sci_data.ccsds.iter() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        for event in &events {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    if (met - utc_tail).abs() < 2.0 {
                        met_anchor = Some(met);
                        anchor_ptime = *ptime;
                        prev_ptime = *ptime;
                        wrap_count = 0;
                    }
                    if let Some(anchor) = met_anchor {
                        if *ptime < prev_ptime && (prev_ptime - *ptime) > 262144 {
                            wrap_count += 1;
                        }
                        prev_ptime = *ptime;
                        let total_ticks =
                            wrap_count * PTIME_MOD as i64 + *ptime as i64 - anchor_ptime as i64;
                        second_times.push(anchor + total_ticks as f64 * 2e-6 + 4.0);
                    }
                }
                Pack::Event { ptime, .. } => {
                    if let Some(_anchor) = met_anchor {
                        if *ptime < prev_ptime && (prev_ptime - *ptime) > 262144 {
                            wrap_count += 1;
                        }
                        prev_ptime = *ptime;
                    }
                }
                Pack::Error => {}
            }
        }
    }
    second_times
}

/// 重建所有事例的 MET 时间（扁平化）。
pub fn reconstruct_met_times(sci_data: &SciFile, offset: f64) -> Vec<f64> {
    reconstruct_with_wrap_tracking(sci_data, offset)
        .into_iter()
        .flatten()
        .collect()
}

/// 扫描单个机箱一小时的科学数据，返回饱和时间段列表。
fn scan_saturation_intervals_impl(sci_data: &SciFile, offset: f64) -> Vec<(f64, f64)> {
    const GAP_THRESHOLD: f64 = 6.9e-3; // 6.9ms

    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);

    let mut packs: Vec<PackInfo> = Vec::new();
    for times in &packet_times {
        if times.is_empty() {
            continue;
        }

        let min_t = *times
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_t = *times
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        packs.push(PackInfo {
            min_time: min_t,
            max_time: max_t,
        });
    }

    // 按时间排序，确保 gap 计算正确
    packs.sort_by(|a, b| a.min_time.partial_cmp(&b.min_time).unwrap());

    let mut intervals: Vec<(f64, f64)> = Vec::new();

    for window in packs.windows(2) {
        let gap = window[1].min_time - window[0].max_time;

        if gap > GAP_THRESHOLD {
            let start = window[0].max_time;
            let stop = window[1].min_time;
            intervals.push((start, stop));
        }
    }

    intervals
}

/// 扫描饱和区间，直接返回原始 MET 秒数。
pub fn scan_saturation_intervals_raw(sci_data: &SciFile, offset: f64) -> Vec<(f64, f64)> {
    scan_saturation_intervals_impl(sci_data, offset)
}

/// 以下部分为对原有未走交叉验证管道的调试辅助函数保持接口不变
pub fn print_diagnose_packets(sci_data: &SciFile, offset: f64, pkt_min: usize, pkt_max: usize) {
    let mut met_anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;

    let _ptime_mod = 524288;
    let wrap_period = 1.048576;
    let met_correction = 4.0;

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        if pkt_idx < pkt_min || pkt_idx > pkt_max {
            continue;
        }

        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);

        let mut second_count = 0;
        let mut event_count = 0;
        let mut error_count = 0;

        for event in &events {
            match event {
                Pack::Second { .. } => second_count += 1,
                Pack::Event { .. } => event_count += 1,
                Pack::Error => error_count += 1,
            }
        }

        println!("\n==========================================");
        println!(
            "Packet {}: utc_tail = {}, events = {} (SEC: {}, EVT: {}, ERR: {})",
            pkt_idx,
            utc_tail,
            events.len(),
            second_count,
            event_count,
            error_count
        );

        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    println!(
                        "  [{}] SEC: stime={}, ptime={} => met={}, anchor updated",
                        evt_idx, stime, ptime, met
                    );

                    if (met - utc_tail).abs() < 2.0 {
                        met_anchor = Some(met);
                        anchor_ptime = *ptime;
                    }
                }
                Pack::Event { channel, ptime, .. } => {
                    if let Some(anchor) = met_anchor {
                        let raw_delta = *ptime as f64 * 2e-6 - anchor_ptime as f64 * 2e-6;
                        let time_since_anchor = utc_tail - anchor - met_correction;
                        let wraps = ((time_since_anchor - raw_delta) / wrap_period).floor();

                        println!(
                            "  [{}] EVT: ch={:3}, ptime={:7} | raw_delta={:+.6}, need={:+.6} => wrap={:>2.0}, comp_met={:.6}",
                            evt_idx, channel, ptime, raw_delta, time_since_anchor, wraps,
                            compute_met(*ptime, anchor_ptime, anchor, utc_tail)
                        );
                    } else {
                        println!("  [{}] EVT: ch={:3}, ptime={:7} | NO ANCHOR", evt_idx, channel, ptime);
                    }
                }
                Pack::Error => {
                    println!("  [{}] ERR: CRC mismatch or malformed", evt_idx);
                }
            }
        }
    }
}

/// 打印从指定包开始的所有事例的时钟漂移详情。
pub fn dump_ptime_utc(sci_data: &SciFile, offset: f64, pkt_min: usize, pkt_max: usize) {
    let mut met_anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;

    println!("pkt_index,evt_index,type,ptime,n_wraps,anchor_ptime,utc_tail,anchor_met,met");

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        if pkt_idx < pkt_min || pkt_idx > pkt_max {
            continue;
        }

        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);

        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    if (met - utc_tail).abs() < 2.0 {
                        met_anchor = Some(met);
                        anchor_ptime = *ptime;
                        if let Some(anchor) = met_anchor {
                            let n_wraps = ((utc_tail - anchor - 4.0
                                - (*ptime as f64 * 2e-6 - anchor_ptime as f64 * 2e-6))
                                / 1.048576)
                                .floor()
                                .max(0.0) as i64;
                            println!(
                                "pkt={},evt={},SEC,ptime={},n_wraps={},anchor_pt={},utc_tail={:.0},anchor_met={:.6},met={:.6}",
                                pkt_idx,
                                evt_idx,
                                ptime,
                                n_wraps,
                                anchor_ptime,
                                utc_tail,
                                anchor,
                                compute_met(*ptime, anchor_ptime, anchor, utc_tail)
                            );
                        }
                    }
                }
                Pack::Event { ptime, .. } => {
                    if let Some(_anchor) = met_anchor {
                        if let Some(anchor) = met_anchor {
                            let n_wraps = ((utc_tail - anchor - 4.0
                                - (*ptime as f64 * 2e-6 - anchor_ptime as f64 * 2e-6))
                                / 1.048576)
                                .floor()
                                .max(0.0) as i64;
                            println!(
                                "pkt={},evt={},EVT,ptime={},n_wraps={},anchor_pt={},utc_tail={:.0},met={:.6}",
                                pkt_idx,
                                evt_idx,
                                ptime,
                                n_wraps,
                                anchor_ptime,
                                utc_tail,
                                compute_met(*ptime, anchor_ptime, anchor, utc_tail)
                            );
                        }
                    }
                }
                Pack::Error => {}
            }
        }
    }
}

/// 单个事例的详细信息（用于 dump-events 调试）。
pub struct EventDetail {
    pub pkt_index: usize,
    pub evt_index: usize,
    pub is_second: bool,
    pub channel: u8,
    pub met: f64,
    pub raw_bytes: [u8; 8],
}

/// 输出指定 MET 范围内的所有事例详情。
pub fn dump_event_details(
    sci_data: &SciFile,
    offset: f64,
    met_min: f64,
    met_max: f64,
) -> Vec<EventDetail> {
    let mut result = Vec::new();
    let mut met_anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;
    let mut max_met_seen: f64 = 0.0;

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);

        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    if (met - utc_tail).abs() < 2.0 {
                        met_anchor = Some(met);
                        anchor_ptime = *ptime;
                        if met > max_met_seen {
                            max_met_seen = met;
                        }
                    }
                    if let Some(anchor) = met_anchor {
                        let computed_met = compute_met_corrected(
                            *ptime,
                            anchor_ptime,
                            anchor,
                            utc_tail,
                            &mut max_met_seen,
                        );
                        if computed_met >= met_min && computed_met <= met_max {
                            result.push(EventDetail {
                                pkt_index: pkt_idx,
                                evt_index: evt_idx,
                                is_second: true,
                                channel: 0,
                                met: computed_met,
                                raw_bytes: [0; 8],
                            });
                        }
                    }
                }
                Pack::Event {
                    ptime,
                    channel,
                    raw_bytes,
                } => {
                    if let Some(anchor) = met_anchor {
                        let computed_met = compute_met_corrected(
                            *ptime,
                            anchor_ptime,
                            anchor,
                            utc_tail,
                            &mut max_met_seen,
                        );
                        if computed_met >= met_min && computed_met <= met_max {
                            result.push(EventDetail {
                                pkt_index: pkt_idx,
                                evt_index: evt_idx,
                                is_second: false,
                                channel: *channel,
                                met: computed_met,
                                raw_bytes: *raw_bytes,
                            });
                        }
                    }
                }
                Pack::Error => {}
            }
        }
    }

    result
}

/// 单个 CCSDS 包的诊断信息。
pub struct PacketDiag {
    pub pkt_index: usize,
    pub n_event: usize,
    pub n_second: usize,
    pub n_error: usize,
    pub n_second_valid: usize,
    pub n_output: usize,
    pub n_dropped: usize,
    pub has_anchor: bool,
    pub utc_tail: f64,
    pub met_min: Option<f64>,
    pub met_max: Option<f64>,
}

/// 对每个 CCSDS 包进行诊断，返回统计信息。
pub fn diagnose_packets(sci_data: &SciFile, offset: f64) -> Vec<PacketDiag> {
    let mut result = Vec::new();

    let mut met_anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;
    let mut max_met_seen: f64 = 0.0;

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);

        let mut n_event = 0usize;
        let mut n_second = 0usize;
        let mut n_error = 0usize;
        let mut n_second_valid = 0usize;
        let mut n_output = 0usize;
        let mut n_dropped = 0usize;
        let mut pkt_met_min: Option<f64> = None;
        let mut pkt_met_max: Option<f64> = None;
        let had_anchor_before = met_anchor.is_some();

        for event in &events {
            match event {
                Pack::Second { stime, ptime } => {
                    n_second += 1;
                    let met = *stime as f64 + offset;
                    if (met - utc_tail).abs() < 2.0 {
                        met_anchor = Some(met);
                        anchor_ptime = *ptime;
                        n_second_valid += 1;
                        if met > max_met_seen {
                            max_met_seen = met;
                        }
                    }
                    if let Some(anchor) = met_anchor {
                        let computed_met = compute_met_corrected(*ptime, anchor_ptime, anchor, utc_tail, &mut max_met_seen);
                        n_output += 1;
                        pkt_met_min =
                            Some(pkt_met_min.map_or(computed_met, |v: f64| v.min(computed_met)));
                        pkt_met_max =
                            Some(pkt_met_max.map_or(computed_met, |v: f64| v.max(computed_met)));
                    } else {
                        n_dropped += 1;
                    }
                }
                Pack::Event { ptime, .. } => {
                    n_event += 1;
                    if let Some(anchor) = met_anchor {
                        let computed_met = compute_met_corrected(*ptime, anchor_ptime, anchor, utc_tail, &mut max_met_seen);
                        n_output += 1;
                        pkt_met_min =
                            Some(pkt_met_min.map_or(computed_met, |v: f64| v.min(computed_met)));
                        pkt_met_max =
                            Some(pkt_met_max.map_or(computed_met, |v: f64| v.max(computed_met)));
                    } else {
                        n_dropped += 1;
                    }
                }
                Pack::Error => {
                    n_error += 1;
                }
            }
        }

        result.push(PacketDiag {
            pkt_index: pkt_idx,
            n_event,
            n_second,
            n_error,
            n_second_valid,
            n_output,
            n_dropped,
            has_anchor: met_anchor.is_some() || had_anchor_before,
            utc_tail,
            met_min: pkt_met_min,
            met_max: pkt_met_max,
        });
    }

    result
}

/// 扫描单个机箱一小时的科学数据，返回饱和时间段列表。
pub fn scan_saturation_intervals(
    sci_data: &SciFile,
    offset: f64,
) -> Vec<(MissionElapsedTime<HxmtHe>, MissionElapsedTime<HxmtHe>)> {
    scan_saturation_intervals_impl(sci_data, offset)
        .into_iter()
        .map(|(start, stop)| {
            (
                MissionElapsedTime::new(start),
                MissionElapsedTime::new(stop),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用户数值示例：
    /// anchor_cur:  MET=292, ptime=216822
    /// 若发生在饱和延迟期间，utc_tail 滞后，该测试将验证由于水位线机制它的周期被自动推后一周期。
    #[test]
    fn test_compute_met_corrected_user_example() {
        let anchor_ptime: u64 = 500000;
        let ptime: u64 = 10000; // 时间发生物理回绕
        let anchor_met: f64 = 292.0;

        let mut max_met_seen = 296.4; // 之前正常积累的水位线
        let utc_tail: f64 = 292.0;    // 本该是 293 却滞后的 utc_tail，会造成 floor 少一周期
        
        let result = compute_met_corrected(ptime, anchor_ptime, anchor_met, utc_tail, &mut max_met_seen);

        // 如果没有修正，floor 算出 n=0，导致算出的 MET 为 292 - 0.98 + 4 = 295.02
        // 但此时水位其实已经在 296.4。差别 > 0.8 ! 触发修正。
        let expected = compute_met(ptime, anchor_ptime, anchor_met, 292.0) + 1.048576;
        
        assert!(
            (result - expected).abs() < 1e-6,
            "expected {expected}, got {result}"
        );
        
        // 验证水位线更新正确
        assert!(
            max_met_seen >= result,
            "max_met_seen should be updated"
        );
    }
}
