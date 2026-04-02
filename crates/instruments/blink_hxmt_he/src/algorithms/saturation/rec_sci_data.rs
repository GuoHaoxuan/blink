use super::crc_check;
use crate::io::level_1b::SciFile;
use crate::types::HxmtHe;
use blink_core::types::MissionElapsedTime;

// ─────────────────────────────────────────────────────────────────────────────
// 常量
// ─────────────────────────────────────────────────────────────────────────────

const PTIME_MOD: u64 = 1 << 19; // 524288
const WRAP_PERIOD: f64 = PTIME_MOD as f64 * 2e-6; // 1.048576s

/// 1B→1K 经验时间校正 (秒)。
/// 通过 GRB 200415A 和 GRB 221009A 交叉验证确定。
const MET_CORRECTION: f64 = 4.0;

// ─────────────────────────────────────────────────────────────────────────────
// CCSDS 包解析
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// 时间重建（待重新设计，见 DESIGN.md）
// ─────────────────────────────────────────────────────────────────────────────

/// 时间重建主函数：输入 CCSDS 包序列，输出每包每事件的 MET。
/// NaN 表示无法确定的事件。
pub fn reconstruct_with_wrap_tracking(sci_data: &SciFile, offset: f64) -> Vec<Vec<f64>> {
    reconstruct_with_wrap_tracking_labeled(sci_data, offset, "")
}

pub fn reconstruct_with_wrap_tracking_labeled(
    sci_data: &SciFile,
    offset: f64,
    _label: &str,
) -> Vec<Vec<f64>> {
    let debug = std::env::var("DEBUG_WRAP").is_ok();
    let n_packets = sci_data.ccsds.len();

    if n_packets == 0 {
        return Vec::new();
    }

    // =====================================================================
    // Step 1: 解析所有包，CRC 过滤
    // =====================================================================
    // 每个 CCSDS 包 109 个 slot，通过 CRC 的为 EVT 或 SEC，不通过的为 Error。
    // 只保留通过 CRC 的事件，记录其 (pkt_idx, evt_idx, ptime, 类型)。

    let mut n_evt_total = 0u64;
    let mut n_sec_total = 0u64;
    let mut n_err_total = 0u64;

    // parsed[pkt_idx] = Vec of (evt_idx, ptime, is_second, channel, raw_bytes)
    // 只包含通过 CRC 的事件
    struct ParsedEvent {
        ptime: u64,
        is_second: bool,
        stime: Option<u64>,  // SEC 才有
        channel: u8,
        raw_bytes: [u8; 8],
    }

    let mut parsed: Vec<Vec<ParsedEvent>> = Vec::with_capacity(n_packets);

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let events = parse_events(ccsds);
        let mut pkt_events: Vec<ParsedEvent> = Vec::new();

        for event in &events {
            match event {
                Pack::Event { ptime, channel, raw_bytes } => {
                    pkt_events.push(ParsedEvent {
                        ptime: *ptime,
                        is_second: false,
                        stime: None,
                        channel: *channel,
                        raw_bytes: *raw_bytes,
                    });
                    n_evt_total += 1;
                }
                Pack::Second { stime, ptime } => {
                    pkt_events.push(ParsedEvent {
                        ptime: *ptime,
                        is_second: true,
                        stime: Some(*stime),
                        channel: 0,
                        raw_bytes: [0; 8],
                    });
                    n_sec_total += 1;
                }
                Pack::Error => {
                    n_err_total += 1;
                }
            }
        }

        parsed.push(pkt_events);
    }

    if debug {
        eprintln!(
            "STEP1 CRC filter: {} EVT + {} SEC = {} pass, {} error ({:.1}%)",
            n_evt_total, n_sec_total, n_evt_total + n_sec_total, n_err_total,
            100.0 * n_err_total as f64 / (n_evt_total + n_sec_total + n_err_total) as f64
        );
    }

    // =====================================================================
    // Step 2: 找出所有有效 SEC，过滤幽灵 SEC
    // =====================================================================
    // SEC 提供绝对时间锚点 (stime, ptime)，但 CRC 碰撞会产生幽灵 SEC。
    //
    // 有效 SEC 满足：(ptime - stime × 500000) mod 524288 ≈ 常数（硬件相位）
    // 幽灵 SEC 的相位随机分布在 0~524287。
    //
    // 算法：
    //   Phase 1: 排序+滑动窗口找最大相位簇
    //   Phase 2: 逐对验证 stime-ptime 一致性，踢掉混入的 ghost

    const TICKS_PER_SEC: i64 = 500000; // 1s / 2μs
    const PHASE_TOLERANCE: i64 = 200;  // ±200 ticks (±0.4ms)，覆盖硬件抖动

    struct SecEvent {
        pkt_idx: usize,
        evt_idx: usize,  // 在 parsed[pkt_idx] 中的下标
        stime: u64,
        ptime: u64,
        met: f64,
    }

    // 收集所有 SEC 候选
    let mut all_secs: Vec<SecEvent> = Vec::new();
    for (pkt_idx, pkt) in parsed.iter().enumerate() {
        for (local_idx, evt) in pkt.iter().enumerate() {
            if let Some(stime) = evt.stime {
                all_secs.push(SecEvent {
                    pkt_idx,
                    evt_idx: local_idx,
                    stime,
                    ptime: evt.ptime,
                    met: stime as f64 + offset,
                });
            }
        }
    }

    // Phase 1: 排序+滑动窗口找最大相位簇
    // 计算每个 SEC 的相位
    let phases: Vec<i64> = all_secs.iter()
        .map(|s| ((s.ptime as i64 - s.stime as i64 * TICKS_PER_SEC) % PTIME_MOD as i64 + PTIME_MOD as i64) % PTIME_MOD as i64)
        .collect();

    // 按 phase 排序的下标
    let mut sorted_idx: Vec<usize> = (0..phases.len()).collect();
    sorted_idx.sort_by_key(|&i| phases[i]);

    // 滑动窗口：找包含最多点的窗口（宽度 = 2 × PHASE_TOLERANCE）
    let window_width = 2 * PHASE_TOLERANCE;
    let mut best_start = 0usize;
    let mut best_count = 0usize;
    let mut left = 0usize;

    for right in 0..sorted_idx.len() {
        // 收缩左边界，保持窗口宽度
        while phases[sorted_idx[right]] - phases[sorted_idx[left]] > window_width {
            left += 1;
        }
        let count = right - left + 1;
        if count > best_count {
            best_count = count;
            best_start = left;
        }
    }

    // 标记簇内的 SEC
    let mut in_cluster = vec![false; all_secs.len()];
    for i in best_start..(best_start + best_count) {
        in_cluster[sorted_idx[i]] = true;
    }

    if debug {
        let cluster_phases: Vec<i64> = (best_start..(best_start + best_count))
            .map(|i| phases[sorted_idx[i]])
            .collect();
        eprintln!(
            "STEP2 phase cluster: {} SECs, phase range {}~{} (span={})",
            best_count,
            cluster_phases.first().unwrap_or(&0),
            cluster_phases.last().unwrap_or(&0),
            cluster_phases.last().unwrap_or(&0) - cluster_phases.first().unwrap_or(&0),
        );
    }

    // Phase 2: 逐对验证 stime-ptime 一致性
    // 按打包顺序（pkt_idx, evt_idx）排序簇内的 SEC
    // 这是事件通过 FIFO 的物理顺序
    let mut cluster_indices: Vec<usize> = (0..all_secs.len())
        .filter(|&i| in_cluster[i])
        .collect();
    cluster_indices.sort_by_key(|&i| (all_secs[i].pkt_idx, all_secs[i].evt_idx));

    // Phase 2: stime 升序检查（LIS）
    // FIFO 保序 → 打包顺序中 stime 必须严格递增。
    // Ghost SEC 的 stime 是垃圾值，会打破升序。
    // 用 LIS 找主序列，不在 LIS 中的是 ghost。
    let mut is_valid = vec![false; all_secs.len()];

    if cluster_indices.len() > 1 {
        let vals: Vec<u64> = cluster_indices.iter()
            .map(|&i| all_secs[i].stime)
            .collect();

        let n = vals.len();
        let mut tails: Vec<u64> = Vec::new();
        let mut tail_pos: Vec<usize> = Vec::new();
        let mut parent: Vec<Option<usize>> = vec![None; n];

        for i in 0..n {
            let pos = tails.partition_point(|&t| t < vals[i]);
            if pos == tails.len() {
                tails.push(vals[i]);
                tail_pos.push(i);
            } else {
                tails[pos] = vals[i];
                tail_pos[pos] = i;
            }
            parent[i] = if pos > 0 { Some(tail_pos[pos - 1]) } else { None };
        }

        let mut in_lis = vec![false; n];
        let mut idx = *tail_pos.last().unwrap();
        loop {
            in_lis[idx] = true;
            match parent[idx] {
                Some(p) => idx = p,
                None => break,
            }
        }

        for (k, &ci) in cluster_indices.iter().enumerate() {
            if in_lis[k] {
                is_valid[ci] = true;
            }
        }
    } else if cluster_indices.len() == 1 {
        is_valid[cluster_indices[0]] = true;
    }

    // 收集有效 SEC
    let valid_secs: Vec<&SecEvent> = all_secs.iter()
        .enumerate()
        .filter(|&(i, _)| is_valid[i])
        .map(|(_, s)| s)
        .collect();

    let n_valid_sec = valid_secs.len();
    let n_ghost_sec = all_secs.len() - n_valid_sec;

    if debug {
        eprintln!(
            "STEP2 SEC validated: {} valid, {} ghost (total {})",
            n_valid_sec, n_ghost_sec, all_secs.len()
        );

        if let (Some(first), Some(last)) = (valid_secs.first(), valid_secs.last()) {
            eprintln!(
                "  stime range: {} ~ {} ({} seconds)",
                first.stime, last.stime, last.stime - first.stime
            );
            eprintln!(
                "  pkt range: {} ~ {}",
                valid_secs.iter().map(|s| s.pkt_idx).min().unwrap(),
                valid_secs.iter().map(|s| s.pkt_idx).max().unwrap(),
            );

            // stime gap 分布
            let mut gap1 = 0u32;
            let mut gap2 = 0u32;
            let mut gap_other = Vec::new();
            for w in valid_secs.windows(2) {
                let gap = w[1].stime as i64 - w[0].stime as i64;
                match gap {
                    1 => gap1 += 1,
                    2 => gap2 += 1,
                    _ => gap_other.push((w[0].stime, w[1].stime, gap, w[0].pkt_idx, w[1].pkt_idx)),
                }
            }
            eprintln!("  stime gaps: {}×1s, {}×2s, {}×other", gap1, gap2, gap_other.len());
            for (s1, s2, gap, p1, p2) in &gap_other {
                let t_rel = *s1 as f64 + offset - 339945422.0;
                eprintln!("    stime {}→{} (gap={}s) pkt {}→{} T+{:.0}", s1, s2, gap, p1, p2, t_rel);
            }
        }
    }

    // =====================================================================
    // Step 3: 对所有相邻 SEC 对，解算中间事件的 MET
    // =====================================================================
    // Δstime=1: elapsed_fwd 唯一 (k=0)，直接对 elapsed_fwd 求 LIS
    // Δstime>1: 保持贪心+LIS（待改进）

    let mut result: Vec<Vec<f64>> = parsed.iter().map(|pkt| vec![f64::NAN; pkt.len()]).collect();
    let mut n_resolved = 0u64;
    let mut n_ghost_deadzone = 0u64;
    let mut n_ghost_order = 0u64;
    let mut n_sec_pairs = 0u64;

    // 有效 SEC 按打包顺序排列的索引
    let mut valid_indices: Vec<usize> = (0..all_secs.len())
        .filter(|&i| is_valid[i])
        .collect();
    valid_indices.sort_by_key(|&i| (all_secs[i].pkt_idx, all_secs[i].evt_idx));

    // 给 SEC 事件本身赋值 MET
    for &vi in &valid_indices {
        let sec = &all_secs[vi];
        result[sec.pkt_idx][sec.evt_idx] = sec.met + MET_CORRECTION;
    }

    // 对每对相邻有效 SEC
    for w in valid_indices.windows(2) {
        let sec1 = &all_secs[w[0]];
        let sec2 = &all_secs[w[1]];
        let ds = sec2.stime as i64 - sec1.stime as i64;

        if ds <= 0 {
            continue;
        }

        // 环境变量控制最大 gap：MAX_SEC_GAP=1 只处理 1s 对
        let max_gap: i64 = std::env::var("MAX_SEC_GAP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(i64::MAX);
        if ds > max_gap {
            continue;
        }
        n_sec_pairs += 1;

        let met1 = sec1.met + MET_CORRECTION;
        let met2 = sec2.met + MET_CORRECTION;
        let pt1 = sec1.ptime as i64;

        // 两个 SEC 之间 ptime 的总前进量
        let total_ticks = ds * TICKS_PER_SEC;  // ds 秒 × 500000 ticks/s

        // 收集两个 SEC 之间的所有事件
        let pkt_a = sec1.pkt_idx;
        let evt_a = sec1.evt_idx;
        let pkt_b = sec2.pkt_idx;
        let evt_b = sec2.evt_idx;

        struct Candidate {
            pkt_idx: usize,
            local_idx: usize,
            elapsed_fwd: i64,  // mod PTIME_MOD
            utc_max_elapsed: i64,  // UTC tail 约束：elapsed 上界
        }

        let mut candidates: Vec<Candidate> = Vec::new();
        let mut last_pkt_idx: usize = usize::MAX;
        let mut cached_utc_max: i64 = total_ticks;
        for pkt_idx in pkt_a..=pkt_b {
            // 每个包计算一次 UTC tail 约束
            if pkt_idx != last_pkt_idx {
                let utc_tail = get_utc_tail(&sci_data.ccsds[pkt_idx]);
                // elapsed ≤ (utc_tail + 1 - sec1.met) / 2μs
                // +1 因为 UTC tail 是整秒截断
                let utc_ticks = ((utc_tail + 1.0 - sec1.met) / 2e-6) as i64;
                cached_utc_max = utc_ticks.clamp(0, total_ticks);
                last_pkt_idx = pkt_idx;
            }
            let start = if pkt_idx == pkt_a { evt_a + 1 } else { 0 };
            let end = if pkt_idx == pkt_b { evt_b } else { parsed[pkt_idx].len() };
            for local_idx in start..end {
                let pt = parsed[pkt_idx][local_idx].ptime as i64;
                let elapsed_fwd = (pt - pt1).rem_euclid(PTIME_MOD as i64);
                candidates.push(Candidate { pkt_idx, local_idx, elapsed_fwd, utc_max_elapsed: cached_utc_max });
            }
        }

        let pmod = PTIME_MOD as i64;
        let mut alive = vec![false; candidates.len()];
        let mut actual_elapsed = vec![0i64; candidates.len()];

        if ds == 1 {
            // ─── Δstime=1: 直接 LIS ───
            // elapsed_fwd 唯一（k=0），过滤 dead zone 后对 ef 求 LIS
            // LIS 自动排除幽灵事件（随机 ptime 打破升序），不会级联丢失

            // 收集有效候选（非 dead zone）的下标
            let mut valid_idx: Vec<usize> = Vec::new();
            for (i, c) in candidates.iter().enumerate() {
                if c.elapsed_fwd <= total_ticks {
                    actual_elapsed[i] = c.elapsed_fwd;
                    valid_idx.push(i);
                } else {
                    n_ghost_deadzone += 1;
                }
            }

            // 对有效候选的 elapsed_fwd 求 LIS
            if valid_idx.len() > 1 {
                let vals: Vec<i64> = valid_idx.iter()
                    .map(|&i| actual_elapsed[i])
                    .collect();

                let n = vals.len();
                let mut tails: Vec<i64> = Vec::new();
                let mut tail_pos: Vec<usize> = Vec::new();
                let mut parent: Vec<Option<usize>> = vec![None; n];

                for i in 0..n {
                    let pos = tails.partition_point(|&t| t < vals[i]);
                    if pos == tails.len() {
                        tails.push(vals[i]);
                        tail_pos.push(i);
                    } else {
                        tails[pos] = vals[i];
                        tail_pos[pos] = i;
                    }
                    parent[i] = if pos > 0 { Some(tail_pos[pos - 1]) } else { None };
                }

                // 回溯 LIS
                let mut in_lis = vec![false; n];
                let mut idx = *tail_pos.last().unwrap();
                loop {
                    in_lis[idx] = true;
                    match parent[idx] {
                        Some(p) => idx = p,
                        None => break,
                    }
                }

                for (k, &vi) in valid_idx.iter().enumerate() {
                    if in_lis[k] {
                        alive[vi] = true;
                    } else {
                        n_ghost_order += 1;
                    }
                }
            } else if valid_idx.len() == 1 {
                alive[valid_idx[0]] = true;
            }
        } else {
            // ─── Δstime>1: 分组 LIS ───
            // 每个事件有 ds 个候选 elapsed = elapsed_fwd + w × PTIME_MOD, w ∈ [0, ds)
            // 全局求解：每个事件最多选一个候选，使选出的 elapsed 严格递增
            // 同一事件的候选按降序处理，避免同组候选互相"抬轿"

            let mut entries: Vec<(usize, i64)> = Vec::new(); // (event_idx, elapsed)
            let mut tails: Vec<i64> = Vec::new();
            let mut tail_entry: Vec<usize> = Vec::new();
            let mut lis_parent: Vec<Option<usize>> = Vec::new();

            for (event_idx, c) in candidates.iter().enumerate() {
                let mut cands: Vec<i64> = (0..ds)
                    .map(|w| c.elapsed_fwd + w * pmod)
                    .filter(|&e| e >= 0 && e <= total_ticks && e <= c.utc_max_elapsed)
                    .collect();
                cands.sort_unstable_by(|a, b| b.cmp(a)); // 降序

                for elapsed in cands {
                    let eidx = entries.len();
                    entries.push((event_idx, elapsed));

                    let pos = tails.partition_point(|&t| t < elapsed);
                    lis_parent.push(if pos > 0 { Some(tail_entry[pos - 1]) } else { None });

                    if pos == tails.len() {
                        tails.push(elapsed);
                        tail_entry.push(eidx);
                    } else {
                        tails[pos] = elapsed;
                        tail_entry[pos] = eidx;
                    }
                }
            }

            // 回溯标记 LIS 成员
            if !tails.is_empty() {
                let mut idx = *tail_entry.last().unwrap();
                loop {
                    let (ev, el) = entries[idx];
                    alive[ev] = true;
                    actual_elapsed[ev] = el;
                    match lis_parent[idx] {
                        Some(p) => idx = p,
                        None => break,
                    }
                }
            }

            // 统计未选中事件
            for (i, c) in candidates.iter().enumerate() {
                if !alive[i] {
                    let has_valid = (0..ds).any(|w| {
                        let e = c.elapsed_fwd + w * pmod;
                        e >= 0 && e <= total_ticks && e <= c.utc_max_elapsed
                    });
                    if has_valid {
                        n_ghost_order += 1;
                    } else {
                        n_ghost_deadzone += 1;
                    }
                }
            }
        }

        // 赋值 MET
        for (i, c) in candidates.iter().enumerate() {
            if alive[i] {
                let met_fwd = met1 + actual_elapsed[i] as f64 * 2e-6;
                let remaining = total_ticks - actual_elapsed[i];
                let met_bwd = met2 - remaining as f64 * 2e-6;
                result[c.pkt_idx][c.local_idx] = (met_fwd + met_bwd) / 2.0;
                n_resolved += 1;
            }
        }
    }

    if debug {
        let n_total_events: usize = parsed.iter().map(|p| p.len()).sum();
        let n_nan = result.iter().flat_map(|p| p.iter()).filter(|m| m.is_nan()).count();
        eprintln!(
            "STEP3 1s-SEC pairs: {} pairs, {} events resolved",
            n_sec_pairs, n_resolved
        );
        eprintln!(
            "  ghosts: {} dead-zone + {} order-violation = {} total",
            n_ghost_deadzone, n_ghost_order, n_ghost_deadzone + n_ghost_order
        );
        eprintln!(
            "  coverage: {}/{} events have MET ({:.1}%), {} NaN",
            n_total_events - n_nan, n_total_events,
            100.0 * (n_total_events - n_nan) as f64 / n_total_events as f64,
            n_nan
        );
    }

    result
}

// ─────────────────────────────────────────────────────────────────────────────
// 公共接口（供 CLI 和其他模块调用）
// ─────────────────────────────────────────────────────────────────────────────

/// 提取所有秒事例（Second event）的重建 MET 时间。
pub fn extract_second_event_times(sci_data: &SciFile, offset: f64) -> Vec<f64> {
    let mut second_times: Vec<f64> = Vec::new();

    for ccsds in sci_data.ccsds.iter() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        for event in &events {
            if let Pack::Second { stime, .. } = event {
                let met = *stime as f64 + offset;
                if (met - utc_tail).abs() < 2.0 {
                    second_times.push(met);
                }
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
    solve_events(sci_data, offset, Some(met_min), Some(met_max))
}

/// 时间解算：返回所有事件的详细信息。
/// met_min/met_max 为 None 时不做时间窗口过滤。
pub fn solve_events(
    sci_data: &SciFile,
    offset: f64,
    met_min: Option<f64>,
    met_max: Option<f64>,
) -> Vec<EventDetail> {
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);
    let mut result = Vec::new();
    let lo = met_min.unwrap_or(f64::NEG_INFINITY);
    let hi = met_max.unwrap_or(f64::INFINITY);

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let events = parse_events(ccsds);
        let times = &packet_times[pkt_idx];

        let mut time_idx = 0;
        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Second { .. } => {
                    if time_idx < times.len() {
                        let computed_met = times[time_idx];
                        time_idx += 1;
                        if computed_met >= lo && computed_met <= hi {
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
                    channel,
                    raw_bytes,
                    ..
                } => {
                    if time_idx < times.len() {
                        let computed_met = times[time_idx];
                        time_idx += 1;
                        if computed_met >= lo && computed_met <= hi {
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
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);
    let mut result = Vec::new();
    let mut has_anchor = false;

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        let times = &packet_times[pkt_idx];

        let mut n_event = 0usize;
        let mut n_second = 0usize;
        let mut n_error = 0usize;
        let mut n_second_valid = 0usize;

        for event in &events {
            match event {
                Pack::Second { stime, .. } => {
                    n_second += 1;
                    let met = *stime as f64 + offset;
                    if (met - utc_tail).abs() < 2.0 {
                        n_second_valid += 1;
                        has_anchor = true;
                    }
                }
                Pack::Event { .. } => {
                    n_event += 1;
                }
                Pack::Error => {
                    n_error += 1;
                }
            }
        }

        let n_output = times.len();
        let n_dropped = (n_event + n_second).saturating_sub(n_output);

        let pkt_met_min = times.iter().copied().reduce(f64::min);
        let pkt_met_max = times.iter().copied().reduce(f64::max);

        result.push(PacketDiag {
            pkt_index: pkt_idx,
            n_event,
            n_second,
            n_error,
            n_second_valid,
            n_output,
            n_dropped,
            has_anchor,
            utc_tail,
            met_min: pkt_met_min,
            met_max: pkt_met_max,
        });
    }

    result
}

/// 扫描饱和区间，返回 MissionElapsedTime 类型。
pub fn scan_saturation_intervals(
    sci_data: &SciFile,
    offset: f64,
) -> Vec<(MissionElapsedTime<HxmtHe>, MissionElapsedTime<HxmtHe>)> {
    // 依赖时间重建，暂时返回空
    Vec::new()
}

/// 扫描饱和区间，直接返回原始 MET 秒数。
pub fn scan_saturation_intervals_raw(sci_data: &SciFile, offset: f64) -> Vec<(f64, f64)> {
    Vec::new()
}

/// 诊断：打印包信息。
pub fn print_diagnose_packets(sci_data: &SciFile, offset: f64, pkt_min: usize, pkt_max: usize) {
    let diags = diagnose_packets(sci_data, offset);
    for d in &diags {
        if d.pkt_index >= pkt_min && d.pkt_index <= pkt_max {
            println!(
                "pkt={} evt={} sec={} err={} sec_valid={} out={} drop={} anchor={} utc={:.0} met=[{}, {}]",
                d.pkt_index, d.n_event, d.n_second, d.n_error, d.n_second_valid,
                d.n_output, d.n_dropped, d.has_anchor, d.utc_tail,
                d.met_min.map_or("?".to_string(), |v| format!("{:.6}", v)),
                d.met_max.map_or("?".to_string(), |v| format!("{:.6}", v)),
            );
        }
    }
}

/// 打印 ptime/utc 诊断信息。
pub fn dump_ptime_utc(sci_data: &SciFile, offset: f64, pkt_min: usize, pkt_max: usize) {
    println!("pkt,evt_idx,type,ptime,stime,utc_tail,met");
    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        if pkt_idx < pkt_min || pkt_idx > pkt_max {
            continue;
        }
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Event { ptime, .. } => {
                    println!("{},{},EVT,{},,,{:.0}", pkt_idx, evt_idx, ptime, utc_tail);
                }
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    println!("{},{},SEC,{},{},{:.0},{:.6}", pkt_idx, evt_idx, ptime, stime, utc_tail, met);
                }
                Pack::Error => {
                    println!("{},{},ERR,,,,", pkt_idx, evt_idx);
                }
            }
        }
    }
}

/// 诊断：对每个 CCSDS 包尝试 0~7 字节偏移，输出各偏移下的 CRC 通过数。
pub fn check_byte_offsets(sci_data: &SciFile, pkt_min: usize, pkt_max: usize) {
    println!("pkt,utc_tail,off0,off1,off2,off3,off4,off5,off6,off7");
    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        if pkt_idx < pkt_min || pkt_idx > pkt_max {
            continue;
        }
        let utc_tail = get_utc_tail(ccsds);
        let mut pass_counts = [0u32; 8];

        for byte_offset in 0..8usize {
            let start = 6 + byte_offset;
            let end = 878;
            if start >= end {
                continue;
            }
            let payload = &ccsds[start..end];
            for chunk in payload.chunks_exact(8) {
                let mut row = [0u64; 8];
                for (i, byte) in chunk.iter().enumerate() {
                    row[i] = *byte as u64;
                }
                if crc_check(&row) == row[7] & 0x0F {
                    pass_counts[byte_offset] += 1;
                }
            }
        }

        println!(
            "{},{:.0},{},{},{},{},{},{},{},{}",
            pkt_idx, utc_tail,
            pass_counts[0], pass_counts[1], pass_counts[2], pass_counts[3],
            pass_counts[4], pass_counts[5], pass_counts[6], pass_counts[7],
        );
    }
}
