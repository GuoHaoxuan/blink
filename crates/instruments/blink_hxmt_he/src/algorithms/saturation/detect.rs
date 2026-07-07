use super::rec_sci_data::reconstruct_with_wrap_tracking;
use crate::io::level_1b::SciFile;

/// 饱和类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaturationType {
    /// 整包丢失：FIFOAFullReset 触发，整个 FIFO A 被清空。
    /// 表现为重建时间序列中出现远大于正常包间隔的空洞。
    FifoReset,
}

/// 单个饱和区间
#[derive(Debug, Clone)]
pub struct SaturationInterval {
    /// 空洞起始 MET（前一个包的最晚事件时间）
    pub start_met: f64,
    /// 空洞结束 MET（后一个包的最早事件时间）
    pub stop_met: f64,
    /// 空洞持续时间（秒）
    pub gap_seconds: f64,
    /// 空洞前一个包的索引（排序后）
    pub prev_pkt_idx: usize,
    /// 空洞后一个包的索引（排序后）
    pub next_pkt_idx: usize,
    /// 饱和类型
    pub saturation_type: SaturationType,
}

/// 每个 CCSDS 包的时间摘要
struct PacketTimeSummary {
    /// 原始包索引（在 SciFile.ccsds 中的位置）
    pkt_idx: usize,
    /// 包内最早事件的重建 MET
    min_met: f64,
    /// 包内最晚事件的重建 MET
    max_met: f64,
    /// 包内有效事件数
    n_events: usize,
}

const GAP_FACTOR: f64 = 100.0;

/// FIFO Reset gap 的最大持续时间（秒）。超过此值的 gap 不认为是 FIFO 复位，
/// 而是数据传输中断、SAA 等其他原因。正常 FIFO reset gap 在 8ms~100ms 量级。
const MAX_FIFO_RESET_GAP: f64 = 1.0;

/// MCU 读取速率下限 (events/s)。
/// MCU 以固定速率从 FIFO A 读取：109 events / ~7ms ≈ 15,600 evt/s。
/// 只有当物理事件率超过此值时，FIFO 才可能溢出触发 FIFOAFullReset。
/// 设为 15000 略低于理论值，留一点余量。
const MCU_READ_RATE_FLOOR: f64 = 15000.0;

/// 从单个包的时间跨度和事件数估算平均事件间隔 (秒/事件)。
/// 如果包内时间跨度过小（<1μs）或事件数不足，返回 None。
fn mean_event_interval(summary: &PacketTimeSummary) -> Option<f64> {
    let span = summary.max_met - summary.min_met;
    if span < 1e-6 || summary.n_events < 2 {
        return None;
    }
    Some(span / summary.n_events as f64)
}

/// 从单个包估算事件率 (events/s)。
/// 如果包内时间跨度过小（<1μs）或事件数不足，返回 None。
fn event_rate(summary: &PacketTimeSummary) -> Option<f64> {
    let span = summary.max_met - summary.min_met;
    if span < 1e-6 || summary.n_events < 2 {
        return None;
    }
    Some(summary.n_events as f64 / span)
}

/// 检测整包丢失（FIFO reset）造成的饱和区间。
///
/// 算法：
/// 1. 对每个 CCSDS 包重建所有事件的 MET 时间，提取 (min_met, max_met, n_events)
/// 2. 按 min_met 排序
/// 3. 对每对相邻包：
///    - baseline = 紧邻两包中平均事件间隔较小的那个（事件率较高的包）
///    - local_max_rate = ±5 包窗口内（包含紧邻 2 包，共最多 12 包）的最大事件率
///    - 若 local_max_rate < MCU_READ_RATE_FLOOR → 跳过（源率不到饱和阈值）
///    - 若 gap > baseline × GAP_FACTOR → 标记为 FifoReset
pub fn detect_fifo_reset_intervals(sci_data: &SciFile, offset: f64) -> Vec<SaturationInterval> {
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);

    let mut summaries: Vec<PacketTimeSummary> = Vec::new();
    for (pkt_idx, times) in packet_times.iter().enumerate() {
        let valid: Vec<f64> = times.iter().copied().filter(|t| !t.is_nan()).collect();
        if valid.is_empty() {
            continue;
        }
        let min_met = valid.iter().cloned().reduce(f64::min).unwrap();
        let max_met = valid.iter().cloned().reduce(f64::max).unwrap();
        summaries.push(PacketTimeSummary {
            pkt_idx,
            min_met,
            max_met,
            n_events: valid.len(),
        });
    }

    summaries.sort_by(|a, b| a.min_met.partial_cmp(&b.min_met).unwrap());

    let mut intervals = Vec::new();
    for (wi, window) in summaries.windows(2).enumerate() {
        let gap = window[1].min_met - window[0].max_met;
        if gap <= 0.0 {
            continue;
        }

        let iv_prev = mean_event_interval(&window[0]);
        let iv_next = mean_event_interval(&window[1]);
        let baseline = match (iv_prev, iv_next) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => continue,
        };

        // 用 ±5 包窗口（含紧邻 2 包）的最大事件率作为本地源率估计：
        // 单包率有涨落，扩展到 12 包窗口取最大值更稳健。
        let lo = wi.saturating_sub(5);
        let hi = (wi + 6).min(summaries.len() - 1);
        let mut local_max_rate = 0.0_f64;
        let mut found = false;
        for k in lo..=hi {
            if let Some(r) = event_rate(&summaries[k]) {
                local_max_rate = local_max_rate.max(r);
                found = true;
            }
        }
        if !found || local_max_rate < MCU_READ_RATE_FLOOR {
            continue;
        }

        if gap > baseline * GAP_FACTOR && gap <= MAX_FIFO_RESET_GAP {
            intervals.push(SaturationInterval {
                start_met: window[0].max_met,
                stop_met: window[1].min_met,
                gap_seconds: gap,
                prev_pkt_idx: window[0].pkt_idx,
                next_pkt_idx: window[1].pkt_idx,
                saturation_type: SaturationType::FifoReset,
            });
        }
    }

    intervals
}

/// 每个包的时间摘要（公开版本，用于重建）
#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub pkt_idx: usize,
    pub min_met: f64,
    pub max_met: f64,
    pub n_events: usize,
}

impl PacketInfo {
    pub fn span(&self) -> f64 {
        self.max_met - self.min_met
    }
}

/// 从 SciFile 提取包时间摘要列表（按 min_met 排序）
pub fn extract_packet_infos(sci_data: &SciFile, offset: f64) -> Vec<PacketInfo> {
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);
    let mut infos: Vec<PacketInfo> = packet_times
        .iter()
        .enumerate()
        .filter_map(|(pkt_idx, times)| {
            let valid: Vec<f64> = times.iter().copied().filter(|t| !t.is_nan()).collect();
            if valid.is_empty() {
                return None;
            }
            let min_met = valid.iter().cloned().reduce(f64::min).unwrap();
            let max_met = valid.iter().cloned().reduce(f64::max).unwrap();
            Some(PacketInfo {
                pkt_idx,
                min_met,
                max_met,
                n_events: valid.len(),
            })
        })
        .collect();
    infos.sort_by(|a, b| a.min_met.partial_cmp(&b.min_met).unwrap());
    infos
}

/// 不可信时间区间（FIFO reset gap 或拥塞宽包的时间覆盖）
#[derive(Debug, Clone)]
pub struct UnreliableInterval {
    pub start: f64,
    pub stop: f64,
}

/// 单个 box 的饱和重建数据
#[derive(Debug)]
pub struct BoxReconstructionData {
    /// 原始事件 MET 时间（已排序）
    pub events: Vec<f64>,
    /// 与 events 一一对应的 wrapped channel（SEC 槽 = CHANNEL_SEC）
    pub channels: Vec<u16>,
    /// 与 events 一一对应的脉宽 pulinfo（NaI/CsI 甄别；SEC 槽 = 0）
    pub pulse_widths: Vec<u8>,
    /// FIFO reset 区间
    pub gaps: Vec<SaturationInterval>,
    /// 包信息
    pub packets: Vec<PacketInfo>,
    /// 每个包内的事件时间（索引 = 原始包号，内部已排序）
    pub packet_events: Vec<Vec<f64>>,
    /// 不可信区间（FIFO reset gap），用于交叉参考时排除
    pub unreliable: Vec<UnreliableInterval>,
}

/// 检测不可信时间区间：仅 FIFO reset gap。
///
/// 拥塞宽包和包内泊松异常检测已移除：
/// - 拥塞宽包：实际触发多为 SAA 开关机导致的包跨时异常，非真正拥塞
/// - 包内异常：与静默丢数检测同一判据（泊松 log₁₀(p) < -10），
///   因 λ 在单包时间跨度内不稳定导致大量误报
pub fn detect_unreliable_intervals(
    gaps: &[SaturationInterval],
    _packets: &[PacketInfo],
    _packet_events: &[Vec<f64>],
) -> Vec<UnreliableInterval> {
    let mut intervals: Vec<UnreliableInterval> = Vec::new();

    for g in gaps {
        intervals.push(UnreliableInterval {
            start: g.start_met,
            stop: g.stop_met,
        });
    }

    // 按 start 排序
    intervals.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    intervals
}

fn is_in_unreliable(t: f64, intervals: &[UnreliableInterval]) -> bool {
    intervals.iter().any(|iv| t >= iv.start && t <= iv.stop)
}

/// 重建后的补全事件
#[derive(Debug, Clone)]
pub struct ReconstructedGap {
    /// 对应的 gap 索引
    pub gap_idx: usize,
    /// 补全的事件 MET 时间
    pub filled_events: Vec<f64>,
    /// N_lost
    pub n_lost: usize,
    /// 是否使用了交叉参考
    pub has_cross_ref: bool,
}

const EVENTS_PER_PKT: f64 = 109.0;
const SHAPE_BIN_WIDTH: f64 = 0.001; // 1ms

/// 对单个 box 的 FIFO reset gap 进行光变曲线重建。
///
/// 算法：
/// 1. 用参考 box 的事件分布构建校准后的形状函数（1ms bin）
/// 2. N_lost = shape 总和（校准后的参考计数直接给出丢失事件数）
/// 3. 按形状分配事件到各 bin
///
/// 当参考 box 不可用（所有 box 同时饱和）时，退化为 post-reset 包率估算 + 均匀分配。
pub fn reconstruct_gaps(
    target: &BoxReconstructionData,
    references: &[&BoxReconstructionData],
) -> Vec<ReconstructedGap> {
    let mut results = Vec::new();

    for (gap_idx, gap) in target.gaps.iter().enumerate() {
        let gap_start = gap.start_met;
        let gap_stop = gap.stop_met;
        let gap_dur = gap_stop - gap_start;
        if gap_dur <= 0.0 {
            continue;
        }

        // 步骤一：构建形状 bin
        let n_sbins = ((gap_dur / SHAPE_BIN_WIDTH).ceil() as usize).max(1);
        let actual_sbin = gap_dur / n_sbins as f64;
        let mut shape = vec![0.0f64; n_sbins];

        // 汇总所有参考 box 的事件，统一构建形状函数
        let mut has_ref = false;
        for (si, s) in shape.iter_mut().enumerate() {
            let bin_lo = gap_start + si as f64 * actual_sbin;
            let bin_hi = bin_lo + actual_sbin;
            let bin_mid = (bin_lo + bin_hi) / 2.0;

            let mut total_ref_count = 0.0;
            let mut n_valid_refs = 0;

            for ref_data in references {
                if is_in_unreliable(bin_mid, &ref_data.unreliable) {
                    continue;
                }

                let lo_idx = ref_data.events.partition_point(|&t| t < bin_lo);
                let hi_idx = ref_data.events.partition_point(|&t| t < bin_hi);
                let count = (hi_idx - lo_idx) as f64;

                if count > 0.0 {
                    let k = calibrate_ratio_sorted(
                        &target.events, &ref_data.events,
                        &target.unreliable, &ref_data.unreliable,
                        gap_start, gap_stop, 0.5,
                    );
                    total_ref_count += count * k;
                    n_valid_refs += 1;
                }
            }

            if n_valid_refs > 0 {
                *s = total_ref_count / n_valid_refs as f64;
                has_ref = true;
            }
        }

        // ── diagnostic: calibration ratio breakdown ──
        for (ri, ref_data) in references.iter().enumerate() {
            let dw = [(gap_start - 0.5, gap_start), (gap_stop, gap_stop + 0.5)];
            let (mut tc, mut rc) = (0usize, 0usize);
            let (mut te, mut re) = (0.0f64, 0.0f64);
            for &(wl, wh) in &dw {
                let t = effective_duration(wl, wh, &target.unreliable);
                if t > 1e-6 {
                    let a = target.events.partition_point(|&x| x < wl);
                    let b = target.events.partition_point(|&x| x < wh);
                    tc += target.events[a..b].iter()
                        .filter(|&&x| !is_in_unreliable(x, &target.unreliable)).count();
                    te += t;
                }
                let r = effective_duration(wl, wh, &ref_data.unreliable);
                if r > 1e-6 {
                    let a = ref_data.events.partition_point(|&x| x < wl);
                    let b = ref_data.events.partition_point(|&x| x < wh);
                    rc += ref_data.events[a..b].iter()
                        .filter(|&&x| !is_in_unreliable(x, &ref_data.unreliable)).count();
                    re += r;
                }
            }
            if re > 1e-6 && rc > 10 && te > 1e-6 {
                let tr = tc as f64 / te;
                let rr = rc as f64 / re;
                eprintln!("  gap[{gap_idx}] ref[{ri}]: k={:.4}  tgt={tc}/{te:.4}s={tr:.0}/s  ref={rc}/{re:.4}s={rr:.0}/s", tr/rr);
            } else {
                eprintln!("  gap[{gap_idx}] ref[{ri}]: k=1.0(default)  tgt={tc}/{te:.4}s  ref={rc}/{re:.4}s");
            }
        }

        // 步骤二：确定 N_lost
        let n_lost;
        if has_ref {
            let n_filled = shape.iter().filter(|&&v| v > 0.0).count();
            if n_filled * 100 / n_sbins >= 30 {
                // 参考覆盖充分：用 shape 总和作为 N_lost
                interpolate_empty_bins(&mut shape);
                n_lost = shape.iter().sum::<f64>().round() as usize;
                eprintln!("gap[{gap_idx}]: {gap_dur:.4}s  n_lost={n_lost}  cross-ref  cov={n_filled}/{n_sbins}");
            } else {
                // 参考覆盖不足：退化为 pre/post 率线性插值
                fill_shape_fallback(&mut shape, gap, &target.packets);
                n_lost = (shape.iter().sum::<f64>() * actual_sbin).round() as usize;
                eprintln!("gap[{gap_idx}]: {gap_dur:.4}s  n_lost={n_lost}  FALLBACK  cov={n_filled}/{n_sbins}");
            }
        } else {
            // 无参考：pre/post 率线性插值
            fill_shape_fallback(&mut shape, gap, &target.packets);
            n_lost = (shape.iter().sum::<f64>() * actual_sbin).round() as usize;
            eprintln!("gap[{gap_idx}]: {gap_dur:.4}s  n_lost={n_lost}  NO-REF");
        }

        if n_lost == 0 {
            continue;
        }

        // 步骤三：按形状分配事件
        let total: f64 = shape.iter().sum();
        if total <= 0.0 {
            continue;
        }

        let mut filled_events = Vec::with_capacity(n_lost);
        for (si, &s) in shape.iter().enumerate() {
            let n_in_bin = (s / total * n_lost as f64).round() as usize;
            if n_in_bin > 0 {
                let bin_lo = gap_start + si as f64 * actual_sbin;
                let bin_hi = bin_lo + actual_sbin;
                let step = (bin_hi - bin_lo) / n_in_bin as f64;
                for j in 0..n_in_bin {
                    filled_events.push(bin_lo + (j as f64 + 0.5) * step);
                }
            }
        }

        results.push(ReconstructedGap {
            gap_idx,
            filled_events,
            n_lost,
            has_cross_ref: has_ref,
        });
    }

    results
}

/// 从包的 span 估算事件率。
fn packet_rate(packets: &[PacketInfo], pkt_idx: usize) -> Option<f64> {
    packets.iter().find(|p| p.pkt_idx == pkt_idx).and_then(|info| {
        let span = info.span();
        if span > 1e-9 { Some(EVENTS_PER_PKT / span) } else { None }
    })
}

/// 无参考时的 fallback：用 pre/post-reset 包的率线性插值构建 shape。
fn fill_shape_fallback(
    shape: &mut [f64],
    gap: &SaturationInterval,
    packets: &[PacketInfo],
) {
    let r_pre = packet_rate(packets, gap.prev_pkt_idx);
    let r_post = packet_rate(packets, gap.next_pkt_idx);
    let n = shape.len();

    match (r_pre, r_post) {
        (Some(rp), Some(rn)) => {
            for (i, s) in shape.iter_mut().enumerate() {
                let t = (i as f64 + 0.5) / n as f64;
                *s = rp * (1.0 - t) + rn * t;
            }
        }
        (Some(r), None) | (None, Some(r)) => {
            shape.iter_mut().for_each(|v| *v = r);
        }
        (None, None) => {
            shape.iter_mut().for_each(|v| *v = MCU_READ_RATE_FLOOR);
        }
    }
}

/// 计算 target/ref 的事件率比值，用于将参考 box 的计数换算为 target box 的计数。
///
/// 在 gap 前后各 margin 秒的窗口内统计双方的事件率（events/有效秒），
/// 排除各自的 unreliable 区间。
fn calibrate_ratio_sorted(
    target_events: &[f64],
    ref_events: &[f64],
    target_unreliable: &[UnreliableInterval],
    ref_unreliable: &[UnreliableInterval],
    gap_start: f64,
    gap_stop: f64,
    margin: f64,
) -> f64 {
    let windows = [
        (gap_start - margin, gap_start),
        (gap_stop, gap_stop + margin),
    ];

    let mut target_count = 0usize;
    let mut ref_count = 0usize;
    let mut target_effective = 0.0f64;
    let mut ref_effective = 0.0f64;

    for &(win_lo, win_hi) in &windows {
        // target 侧：排除 target 的 unreliable 区间
        let t_eff = effective_duration(win_lo, win_hi, target_unreliable);
        if t_eff > 1e-6 {
            let a = target_events.partition_point(|&t| t < win_lo);
            let b = target_events.partition_point(|&t| t < win_hi);
            // 只计落在可信时段内的事件
            let cnt = target_events[a..b]
                .iter()
                .filter(|&&t| !is_in_unreliable(t, target_unreliable))
                .count();
            target_count += cnt;
            target_effective += t_eff;
        }

        // ref 侧：排除 ref 的 unreliable 区间
        let r_eff = effective_duration(win_lo, win_hi, ref_unreliable);
        if r_eff > 1e-6 {
            let a = ref_events.partition_point(|&t| t < win_lo);
            let b = ref_events.partition_point(|&t| t < win_hi);
            let cnt = ref_events[a..b]
                .iter()
                .filter(|&&t| !is_in_unreliable(t, ref_unreliable))
                .count();
            ref_count += cnt;
            ref_effective += r_eff;
        }
    }

    // 用事件率比值（而非事件数比值），补偿双方有效时长不同
    if ref_effective > 1e-6 && ref_count > 10 && target_effective > 1e-6 {
        let target_rate = target_count as f64 / target_effective;
        let ref_rate = ref_count as f64 / ref_effective;
        target_rate / ref_rate
    } else {
        1.0
    }
}

/// 计算窗口 [lo, hi] 内排除 unreliable 区间后的有效时长。
fn effective_duration(lo: f64, hi: f64, unreliable: &[UnreliableInterval]) -> f64 {
    let mut excluded = 0.0;
    for iv in unreliable {
        let overlap_lo = iv.start.max(lo);
        let overlap_hi = iv.stop.min(hi);
        if overlap_hi > overlap_lo {
            excluded += overlap_hi - overlap_lo;
        }
    }
    (hi - lo - excluded).max(0.0)
}




/// 空 bin 插值：从最近的有值 bin 做线性插值，边缘用最近有值 bin 常数外推。
fn interpolate_empty_bins(shape: &mut [f64]) {
    let n = shape.len();
    if n == 0 {
        return;
    }

    // 预计算每个位置左边和右边最近的有值 bin 索引
    let mut left_filled: Vec<Option<usize>> = vec![None; n];
    let mut right_filled: Vec<Option<usize>> = vec![None; n];

    let mut last = None;
    for i in 0..n {
        if shape[i] > 0.0 {
            last = Some(i);
        }
        left_filled[i] = last;
    }

    last = None;
    for i in (0..n).rev() {
        if shape[i] > 0.0 {
            last = Some(i);
        }
        right_filled[i] = last;
    }

    for i in 0..n {
        if shape[i] > 0.0 {
            continue;
        }
        match (left_filled[i], right_filled[i]) {
            (Some(l), Some(r)) => {
                // 两侧都有值：线性插值
                let t = (i - l) as f64 / (r - l) as f64;
                shape[i] = shape[l] * (1.0 - t) + shape[r] * t;
            }
            (Some(l), None) => shape[i] = shape[l],   // 右侧无值：常数外推
            (None, Some(r)) => shape[i] = shape[r],   // 左侧无值：常数外推
            (None, None) => {}                          // 全空，不应到达此处
        }
    }
}
