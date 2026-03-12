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
const WRAP_PERIOD: f64 = PTIME_MOD as f64 * 2e-6; // 1.048576s

/// 1B→1K 经验时间校正 (秒)。
/// 通过 GRB 200415A 和 GRB 221009A 交叉验证确定。
const MET_CORRECTION: f64 = 4.0;

/// SEC-anchored MET computation with multi-wrap support.
///
/// Uses SEC anchor's (met, ptime) as precise reference. For fresh anchors
/// (< 1.5 WRAP_PERIOD old), determines wrap by threshold on ptime delta.
/// For stale anchors (after FIFO reset gaps), uses utc_tail to estimate
/// the number of complete wraps, then picks the best candidate from
/// n_est-1, n_est, n_est+1 by comparing to utc_tail.
///
/// Immune to multi-detector ptime non-monotonicity (per-event independent).
const WRAP_THRESHOLD: i64 = 10000; // ~20ms in ptime ticks
#[inline]
fn compute_met_anchored(ptime: u64, anchor_ptime: u64, anchor: f64, utc_tail: f64) -> f64 {
    let raw_delta = ptime as i64 - anchor_ptime as i64;
    let elapsed = utc_tail - anchor;

    if elapsed < WRAP_PERIOD * 1.5 && elapsed > -0.5 {
        // Normal case: anchor is fresh, at most 1 wrap
        let adjusted_delta = if raw_delta < -WRAP_THRESHOLD {
            raw_delta + PTIME_MOD as i64
        } else if raw_delta > (PTIME_MOD as i64 - WRAP_THRESHOLD) {
            raw_delta - PTIME_MOD as i64
        } else {
            raw_delta
        };
        anchor + adjusted_delta as f64 * 2e-6 + MET_CORRECTION
    } else {
        // Stale anchor (gap from FIFO reset): estimate wraps using utc_tail
        let n_est = ((elapsed - raw_delta as f64 * 2e-6) / WRAP_PERIOD)
            .round()
            .max(0.0) as i64;

        let mut best_met = f64::NAN;
        let mut best_err = f64::MAX;
        for n in [n_est - 1, n_est, n_est + 1] {
            if n < 0 {
                continue;
            }
            let met = anchor + (raw_delta + n * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
            let err = (met - MET_CORRECTION - utc_tail).abs();
            if err < best_err {
                best_err = err;
                best_met = met;
            }
        }
        best_met
    }
}

/// Floor-based per-event wrap computation (retained for diagnostics only).
#[inline]
fn compute_met(ptime: u64, anchor_ptime: u64, anchor: f64, utc_tail: f64) -> f64 {
    let raw_delta = ptime as i64 - anchor_ptime as i64;
    let raw_delta_seconds = raw_delta as f64 * 2e-6;
    let n_wraps = ((utc_tail - anchor - WRAP_PERIOD - raw_delta_seconds) / WRAP_PERIOD)
        .floor()
        .max(0.0) as i64;
    let total_ticks = n_wraps * PTIME_MOD as i64 + raw_delta;
    anchor + total_ticks as f64 * 2e-6 + MET_CORRECTION
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

/// SEC-anchored 时间重建。
///
/// 每个事件用最近 SEC 的 (met, ptime) 作为锚点，通过 ptime 差值阈值判定
/// 是否发生了 wrap。不使用 utc_tail，不依赖 ptime 全局单调性。
///
/// 前提条件：SEC 事件每秒出现一次，间距 < WRAP_PERIOD (1.048576s)，
/// 因此相邻两个 SEC 锚点之间至多发生 1 次 wrap。
pub fn reconstruct_with_wrap_tracking(sci_data: &SciFile, offset: f64) -> Vec<Vec<f64>> {
    let mut result = Vec::with_capacity(sci_data.ccsds.len());
    let mut anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;
    let debug = std::env::var("DEBUG_WRAP").is_ok();

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        let mut times = Vec::with_capacity(events.len());

        for event in &events {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;
                    if (met - utc_tail).abs() < 2.0 {
                        anchor = Some(met);
                        anchor_ptime = *ptime;
                    }
                    if let Some(anc) = anchor {
                        times.push(compute_met_anchored(
                            *ptime,
                            anchor_ptime,
                            anc,
                            utc_tail,
                        ));
                    }
                }
                Pack::Event { ptime, .. } => {
                    if let Some(anc) = anchor {
                        let met = compute_met_anchored(
                            *ptime,
                            anchor_ptime,
                            anc,
                            utc_tail,
                        );
                        times.push(met);
                    }
                }
                Pack::Error => {}
            }
        }
        if debug && !times.is_empty() {
            let t_min = times.iter().cloned().fold(f64::INFINITY, f64::min);
            let t_max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let elapsed = utc_tail - anchor.unwrap_or(0.0);
            eprintln!(
                "PKT pkt={} ut={:.0} anc={:.3} anc_pt={} el={:.3} n={} tmin={:.6} tmax={:.6} span={:.4}",
                pkt_idx, utc_tail, anchor.unwrap_or(0.0), anchor_ptime,
                elapsed, times.len(), t_min, t_max, t_max - t_min
            );
        }
        result.push(times);
    }

    // ── Pass 2: Fix FIFO-reset time reversals ──
    // After FIFO resets, some packets get reordered in the file. A batch of
    // packets may be placed at time T+X+WRAP_PERIOD when they should be at T+X.
    // Signature: a batch at time A is followed by packets at time B where
    // B < A - 0.5 (time reversal ≈ -WRAP_PERIOD). The batch before the reversal
    // should be shifted by -WRAP_PERIOD.
    fix_wrap_reversals(&mut result);

    // ── Pass 3: Fix wrap-boundary dips ──
    // Near ptime wrap boundaries, some events/packets get assigned one wrap too
    // low. This manifests as: ...HIGH packets... LOW batch... HIGH packets...
    // where the LOW batch is ~WRAP_PERIOD below neighbors. Shift them up.
    // Also fix mixed-wrap packets (span ≈ WRAP_PERIOD) by aligning the minority
    // cluster with neighbors.
    fix_wrap_boundary_dips(&mut result);

    // ── Pass 4: Global sort for complex reordering ──
    // In extreme saturation cases (e.g., GRB 221009A), FIFO resets cause complex
    // multi-level packet reordering that Pass 2 cannot fully resolve. Sort all
    // events globally by time to ensure monotonicity.
    let needs_global_sort = result.iter().any(|pkt| {
        if pkt.len() < 2 {
            return false;
        }
        // Check if packet has any time reversals
        pkt.windows(2).any(|w| w[0] > w[1])
    });

    if needs_global_sort {
        if debug {
            eprintln!("Global sort: detected time reversals, sorting all events");
        }
        for pkt in result.iter_mut() {
            if pkt.len() > 1 {
                pkt.sort_by(|a, b| a.partial_cmp(b).unwrap());
            }
        }
    }

    result
}

/// Detect and fix time reversals caused by FIFO-reset packet reordering.
///
/// When a batch of consecutive packets has times ~WRAP_PERIOD ahead of the
/// following batch, shift the earlier batch by -WRAP_PERIOD. This fixes the
/// case where stale anchors + biased utc_tail cause events to be placed
/// one wrap period too late.
fn fix_wrap_reversals(packets: &mut [Vec<f64>]) {
    let debug = std::env::var("DEBUG_WRAP").is_ok();

    if debug {
        eprintln!("fix_wrap_reversals: called with {} packets", packets.len());
    }

    // Strategy: detect backward jumps ≈ -WRAP_PERIOD between clean packets,
    // then walk backward to find the misplaced batch. Distinguish real
    // misplacement from file reordering by checking whether the batch is
    // preceded by a TIME GAP (FIFO reset → misplacement) or by smooth
    // continuation (file reorder → no shift needed).

    struct PktStat {
        idx: usize,
        mn: f64,
        mx: f64,
    }

    let clean: Vec<PktStat> = (0..packets.len())
        .filter_map(|i| {
            if packets[i].is_empty() {
                return None;
            }
            let mn = packets[i].iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = packets[i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let span = mx - mn;
            if span < 0.3 {
                Some(PktStat { idx: i, mn, mx })
            } else {
                None
            }
        })
        .collect();

    let mut fixed = vec![false; packets.len()];

    for ci in 0..clean.len().saturating_sub(1) {
        if fixed[clean[ci].idx] {
            continue;
        }
        let reversal = clean[ci].mx - clean[ci + 1].mn;
        if reversal < WRAP_PERIOD * 0.8 || reversal > WRAP_PERIOD * 1.2 {
            continue;
        }

        // Found backward jump ≈ -WRAP between clean[ci] and clean[ci+1].
        // Walk backward from clean[ci]: find all packets with
        // mn > clean[ci+1].mn + 0.5*WRAP (i.e., at the elevated level).
        let ref_mn = clean[ci + 1].mn;
        let threshold = ref_mn + WRAP_PERIOD * 0.5;

        let mut first_shifted = clean[ci].idx;
        let mut prev_pkt_max = f64::NAN; // max of the packet just before the batch

        for j in (0..clean[ci].idx).rev() {
            if packets[j].is_empty() || fixed[j] {
                continue;
            }
            let j_mn = packets[j].iter().cloned().fold(f64::INFINITY, f64::min);
            let j_mx = packets[j].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let j_span = j_mx - j_mn;

            if j_mn > threshold && j_span < WRAP_PERIOD * 0.5 {
                first_shifted = j;
            } else {
                prev_pkt_max = j_mx;
                break;
            }
        }

        // Check: is there a gap between the previous packet and the batch?
        // A misplaced batch follows a FIFO reset gap (> 0.5s).
        // File-reordered packets have smooth continuation (gap < 0.1s).
        let batch_mn = packets[first_shifted]
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let gap = batch_mn - prev_pkt_max;

        if debug {
            eprintln!(
                "fix_wrap_reversals: reversal={:.4}s at pkt {}->pkt {}, batch {}..={}, gap_before={:.4}s",
                reversal,
                clean[ci].idx,
                clean[ci + 1].idx,
                first_shifted,
                clean[ci].idx,
                gap
            );
        }

        // Decision criteria:
        // 1. Large gap (> 0.3s) → definitely FIFO reset → shift
        // 2. Small gap but reversal ≈ WRAP_PERIOD → likely FIFO reset in high-rate region → shift
        // 3. Small gap and small reversal → file reorder → skip
        let should_shift = gap > 0.3
            || prev_pkt_max.is_nan()
            || (reversal > WRAP_PERIOD * 0.8 && reversal < WRAP_PERIOD * 1.2);

        if should_shift {
            // Misplaced batch after FIFO reset → shift down
            if debug {
                let reason = if gap > 0.3 || prev_pkt_max.is_nan() {
                    format!("large gap={:.4}s", gap)
                } else {
                    format!("reversal≈WRAP={:.4}s, gap={:.4}s", reversal, gap)
                };
                eprintln!(
                    "fix_wrap_reversals:   → SHIFTING pkts {}..={} by -{:.6} ({})",
                    first_shifted,
                    clean[ci].idx,
                    WRAP_PERIOD,
                    reason
                );
            }
            for idx in first_shifted..=clean[ci].idx {
                if !packets[idx].is_empty() {
                    for t in packets[idx].iter_mut() {
                        *t -= WRAP_PERIOD;
                    }
                    fixed[idx] = true;
                }
            }
        } else if debug {
            eprintln!(
                "fix_wrap_reversals:   → SKIPPING (file reorder: reversal={:.4}s, gap={:.4}s)",
                reversal, gap
            );
        }
    }
}

/// Fix wrap-boundary dips: short batches at the wrong wrap level.
///
/// After FIFO resets, some packets near the ptime wrap boundary get assigned
/// one wrap period too low. Pattern: HIGH...HIGH, LOW_batch, HIGH...HIGH
/// where LOW_batch is ~WRAP_PERIOD below both neighbors. Also fixes mixed-wrap
/// packets (span ≈ WRAP_PERIOD) by shifting the minority cluster.
fn fix_wrap_boundary_dips(packets: &mut [Vec<f64>]) {
    let debug = std::env::var("DEBUG_WRAP").is_ok();

    if debug {
        eprintln!("fix_wrap_boundary_dips: called with {} packets", packets.len());
    }

    // Step 1: Fix sandwiched "dip" batches FIRST.
    // Find forward jumps ≈ WRAP_PERIOD between clean packets, then check
    // if the low batch was preceded by a backward jump from the high level.
    // Also shift LOW events within adjacent mixed-wrap packets.

    struct PktStat {
        idx: usize,
        mn: f64,
        mx: f64,
    }

    let clean: Vec<PktStat> = (0..packets.len())
        .filter_map(|i| {
            if packets[i].is_empty() {
                return None;
            }
            let mn = packets[i].iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = packets[i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let span = mx - mn;
            if span < 0.3 {
                Some(PktStat { idx: i, mn, mx })
            } else {
                None
            }
        })
        .collect();

    let mut shifted = vec![false; packets.len()];

    for ci in 0..clean.len().saturating_sub(1) {
        if shifted[clean[ci].idx] {
            continue;
        }

        // Look for forward jump ≈ WRAP_PERIOD: clean[ci] is LOW, clean[ci+1] is HIGH
        let forward_jump = clean[ci + 1].mn - clean[ci].mx;
        if forward_jump < WRAP_PERIOD * 0.8 || forward_jump > WRAP_PERIOD * 1.2 {
            continue;
        }

        // Walk backward from clean[ci] to find the extent of the LOW batch
        let high_level = clean[ci + 1].mn;
        let low_threshold = high_level - WRAP_PERIOD * 0.5;

        let mut first_low = clean[ci].idx;
        let mut prev_high_max = f64::NAN;
        let mut mixed_pkts_in_range: Vec<usize> = Vec::new();

        for j in (0..clean[ci].idx).rev() {
            if packets[j].is_empty() || shifted[j] {
                continue;
            }
            let j_mn = packets[j].iter().cloned().fold(f64::INFINITY, f64::min);
            let j_mx = packets[j].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let j_span = j_mx - j_mn;

            // Skip mixed-wrap packets but remember them for later processing
            if j_span > WRAP_PERIOD * 0.5 {
                mixed_pkts_in_range.push(j);
                continue;
            }

            if j_mx < low_threshold {
                first_low = j;
            } else if j_mn >= low_threshold {
                // Found a HIGH packet before the batch → sandwiched!
                prev_high_max = j_mx;
                break;
            } else {
                break;
            }
        }

        // Verify: the batch is preceded by HIGH level (backward jump from HIGH to LOW)
        if prev_high_max.is_nan() {
            continue;
        }

        let batch_mn = packets[first_low]
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let backward_jump = prev_high_max - batch_mn;

        if backward_jump < WRAP_PERIOD * 0.5 {
            continue; // not a genuine wrap dip
        }

        if debug {
            eprintln!(
                "fix_wrap_boundary_dips: dip batch pkts {}..={}, backward_jump={:.4}s, forward_jump={:.4}s",
                first_low, clean[ci].idx, backward_jump, forward_jump
            );
            eprintln!(
                "fix_wrap_boundary_dips:   → SHIFTING pkts {}..={} UP by {:.6}",
                first_low, clean[ci].idx, WRAP_PERIOD
            );
        }

        // Shift all clean LOW packets in the batch
        for idx in first_low..=clean[ci].idx {
            if !packets[idx].is_empty() && !shifted[idx] {
                for t in packets[idx].iter_mut() {
                    *t += WRAP_PERIOD;
                }
                shifted[idx] = true;
            }
        }

        // Also shift LOW events within adjacent mixed-wrap packets
        for &mix_idx in &mixed_pkts_in_range {
            if shifted[mix_idx] || packets[mix_idx].is_empty() {
                continue;
            }
            let mix_mn = packets[mix_idx].iter().cloned().fold(f64::INFINITY, f64::min);
            let midpoint = mix_mn + WRAP_PERIOD * 0.5;
            if debug {
                let n_low = packets[mix_idx].iter().filter(|&&t| t < midpoint).count();
                eprintln!(
                    "fix_wrap_boundary_dips:   → also shifting {} LOW events in mixed pkt {} UP",
                    n_low, mix_idx
                );
            }
            for t in packets[mix_idx].iter_mut() {
                if *t < midpoint {
                    *t += WRAP_PERIOD;
                }
            }
            shifted[mix_idx] = true;
        }
    }

    // Step 2: Fix remaining mixed-wrap packets (span ≈ WRAP_PERIOD).
    // Split events into two clusters, shift the minority to match majority
    // based on neighboring packet times.
    for i in 0..packets.len() {
        if shifted[i] || packets[i].len() < 2 {
            continue;
        }
        let mn = packets[i].iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = packets[i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let span = mx - mn;
        if span < WRAP_PERIOD * 0.8 || span > WRAP_PERIOD * 1.2 {
            continue;
        }

        // Split into low and high clusters
        let midpoint = mn + WRAP_PERIOD * 0.5;
        let n_low = packets[i].iter().filter(|&&t| t < midpoint).count();
        let n_high = packets[i].iter().filter(|&&t| t >= midpoint).count();

        if n_low == 0 || n_high == 0 {
            continue;
        }

        // Determine which cluster to shift by looking at neighboring packets
        let mut neighbor_high_count = 0usize;
        let mut neighbor_low_count = 0usize;

        // Check up to 5 neighbors on each side (skip mixed packets)
        for &dir in &[-1i64, 1i64] {
            for step in 1..=5usize {
                let j = i as i64 + dir * step as i64;
                if j < 0 || j >= packets.len() as i64 {
                    break;
                }
                let j = j as usize;
                if packets[j].is_empty() {
                    continue;
                }
                let j_mn = packets[j].iter().cloned().fold(f64::INFINITY, f64::min);
                let j_mx = packets[j].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let j_span = j_mx - j_mn;
                if j_span > WRAP_PERIOD * 0.5 {
                    continue; // skip other mixed packets
                }
                if j_mn >= midpoint {
                    neighbor_high_count += 1;
                } else {
                    neighbor_low_count += 1;
                }
                break; // only count nearest non-mixed neighbor per direction
            }
        }

        if debug {
            eprintln!(
                "fix_wrap_boundary_dips: mixed pkt {}: n_low={}, n_high={}, neighbors: high={}, low={}",
                i, n_low, n_high, neighbor_high_count, neighbor_low_count
            );
        }

        // If neighbors are predominantly at the HIGH level, shift LOW cluster UP
        if neighbor_high_count > neighbor_low_count {
            if debug {
                eprintln!(
                    "fix_wrap_boundary_dips:   → shifting {} LOW events UP by {:.6}",
                    n_low, WRAP_PERIOD
                );
            }
            for t in packets[i].iter_mut() {
                if *t < midpoint {
                    *t += WRAP_PERIOD;
                }
            }
        } else if neighbor_low_count > neighbor_high_count {
            if debug {
                eprintln!(
                    "fix_wrap_boundary_dips:   → shifting {} HIGH events DOWN by {:.6}",
                    n_high, WRAP_PERIOD
                );
            }
            for t in packets[i].iter_mut() {
                if *t >= midpoint {
                    *t -= WRAP_PERIOD;
                }
            }
        }
    }
}

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
                    second_times.push(met + MET_CORRECTION);
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
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);
    let mut result = Vec::new();

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let events = parse_events(ccsds);
        let times = &packet_times[pkt_idx];

        // reconstruct_with_wrap_tracking 已经过滤了 Error，times 和非 Error 事件一一对应
        let mut time_idx = 0;
        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Second { .. } => {
                    if time_idx < times.len() {
                        let computed_met = times[time_idx];
                        time_idx += 1;
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
                    channel,
                    raw_bytes,
                    ..
                } => {
                    if time_idx < times.len() {
                        let computed_met = times[time_idx];
                        time_idx += 1;
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

/// 诊断：对每个 CCSDS 包尝试 0~7 字节偏移，输出各偏移下的 CRC 通过数。
/// 用于验证高错误率包是否因字节错位导致。
pub fn check_byte_offsets(sci_data: &SciFile, pkt_min: usize, pkt_max: usize) {
    println!("pkt,utc_tail,off0,off1,off2,off3,off4,off5,off6,off7");
    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        if pkt_idx < pkt_min || pkt_idx > pkt_max {
            continue;
        }
        let utc_tail = get_utc_tail(ccsds);
        let mut pass_counts = [0u32; 8];

        for offset in 0..8usize {
            let start = 6 + offset;
            let end = 878; // payload ends at 878
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
                    pass_counts[offset] += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchored_basic() {
        let anchor = 292.0;
        let anchor_ptime = 500000u64;
        let utc_tail = 292.0; // fresh anchor

        let met = compute_met_anchored(500100, anchor_ptime, anchor, utc_tail);
        let expected = 292.0 + 100.0 * 2e-6 + MET_CORRECTION;
        assert!((met - expected).abs() < 1e-9);
    }

    #[test]
    fn test_anchored_wrap() {
        let anchor = 292.0;
        let anchor_ptime = 400000u64;
        let utc_tail = 293.0; // fresh, ~1s later

        let met = compute_met_anchored(3000, anchor_ptime, anchor, utc_tail);
        let expected = 292.0 + (3000i64 - 400000 + PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        assert!((met - expected).abs() < 1e-9, "expected {expected}, got {met}");
    }

    #[test]
    fn test_anchored_multi_detector() {
        let anchor = 292.0;
        let anchor_ptime = 100000u64;
        let utc_tail = 292.0;

        let met = compute_met_anchored(99950, anchor_ptime, anchor, utc_tail);
        let expected = 292.0 + (-50.0) * 2e-6 + MET_CORRECTION;
        assert!((met - expected).abs() < 1e-9);

        let met2 = compute_met_anchored(99000, anchor_ptime, anchor, utc_tail);
        let expected2 = 292.0 + (-1000.0) * 2e-6 + MET_CORRECTION;
        assert!((met2 - expected2).abs() < 1e-9);
    }

    #[test]
    fn test_anchored_consistency() {
        let anchor = 292.0;
        let anchor_ptime = 100u64;
        let utc_tail = 292.0;

        let met1 = compute_met_anchored(200, anchor_ptime, anchor, utc_tail);
        let met2 = compute_met_anchored(300, anchor_ptime, anchor, utc_tail);
        let met3 = compute_met_anchored(400, anchor_ptime, anchor, utc_tail);
        assert!(met2 > met1);
        assert!(met3 > met2);
        assert!((met2 - met1 - 100.0 * 2e-6).abs() < 1e-9);
    }

    #[test]
    fn test_anchored_stale_multi_wrap() {
        // FIFO reset gap: anchor is 2+ wraps old
        let anchor = 292.0;
        let anchor_ptime = 200000u64;
        // Event 2.2s later → 2 complete wraps
        // ptime = (200000 + 1100000) % 524288 = 251424 (after 2 full wraps)
        let utc_tail = 294.0; // ~2s after anchor

        let met = compute_met_anchored(251424, anchor_ptime, anchor, utc_tail);
        // Expected: anchor + 2.2s worth of ticks * 2μs + correction
        let expected = 292.0 + (251424i64 - 200000 + 2 * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        assert!(
            (met - expected).abs() < 1e-6,
            "stale anchor multi-wrap: expected {expected}, got {met}"
        );
    }
}
