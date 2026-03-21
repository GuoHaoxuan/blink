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

// ─────────────────────────────────────────────────────────────────────────────
// Two-pass reconstruction structures
// ─────────────────────────────────────────────────────────────────────────────

/// SEC anchor found during pass 1.
struct AnchorInfo {
    pkt_idx: usize,
    #[allow(dead_code)]
    evt_idx: usize,
    met: f64,
    ptime: u64,
}

/// Pass 1 output: anchors, confidence, rough METs.
struct Pass1Result {
    anchors: Vec<AnchorInfo>,
    anchor_is_clean: Vec<bool>,
    confident: Vec<Vec<bool>>,
    rough_mets: Vec<Vec<f64>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Pass 1 helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Detect disruption at packet boundary.
/// Returns true if a FIFO reset (utc_tail jump > 3s) is detected.
#[inline]
fn detect_disruption(prev_utc_tail: f64, cur_utc_tail: f64) -> bool {
    prev_utc_tail > 0.0 && cur_utc_tail - prev_utc_tail > 3.0
}

/// Estimate wrap count across a disruption using utc_tail best-of-3.
/// Given an anchor (met, ptime) and a target event ptime + utc_tail,
/// find the wrap count that best matches the utc_tail reference.
fn estimate_wrap_count(
    anc_met: f64,
    anc_ptime: u64,
    target_ptime: u64,
    target_utc_tail: f64,
) -> i64 {
    let raw_delta = target_ptime as i64 - anc_ptime as i64;
    let elapsed = target_utc_tail + 0.5 - anc_met;
    let n_est = ((elapsed - raw_delta as f64 * 2e-6) / WRAP_PERIOD).round() as i64;

    let target_met = target_utc_tail + MET_CORRECTION + 0.5;
    let mut best_n = n_est;
    let mut best_err = f64::MAX;
    for n in [n_est - 1, n_est, n_est + 1] {
        let met = anc_met + (raw_delta + n * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        let err = (met - target_met).abs();
        if err < best_err {
            best_err = err;
            best_n = n;
        }
    }
    best_n
}

// ─────────────────────────────────────────────────────────────────────────────
// Pass 1: Forward scan + aggressive saturation detection
// ─────────────────────────────────────────────────────────────────────────────

fn pass1_scan(sci_data: &SciFile, offset: f64, debug: bool) -> Pass1Result {
    let n_packets = sci_data.ccsds.len();
    let mut anchors: Vec<AnchorInfo> = Vec::new();
    let mut anchor_is_clean: Vec<bool> = Vec::new();
    let mut confident: Vec<Vec<bool>> = Vec::with_capacity(n_packets);
    let mut rough_mets: Vec<Vec<f64>> = Vec::with_capacity(n_packets);

    // Streaming state
    let mut wrap_count: i64 = 0;
    let mut prev_ptime: Option<u64> = None;
    let mut pre_wrap_ptime: Option<u64> = None; // ptime before a tentative wrap
    let mut pending_wrap: bool = false; // wrap detected but not yet confirmed
    let mut anc_met: Option<f64> = None;
    let mut anc_ptime: u64 = 0;
    let mut is_confident = false; // no anchor yet → not confident
    let mut prev_utc_tail: f64 = 0.0;
    let mut last_accepted_stime: Option<u64> = None;

    // For inter-packet interval check (aggressive saturation detection)
    let mut prev_pkt_met_max: Option<f64> = None;
    let mut baseline_interval: Option<f64> = None;

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);

        // ── Count n_error ──
        let mut n_error: usize = 0;
        for e in &events {
            if matches!(e, Pack::Error) {
                n_error += 1;
            }
        }

        // ── Disruption detection ──
        let is_disruption = detect_disruption(prev_utc_tail, utc_tail);
        if is_disruption {
            // Reset wrap tracking across disruption
            if let Some(am) = anc_met {
                // Find first valid ptime in this packet for wrap estimation
                let first_valid_ptime = events.iter().find_map(|e| match e {
                    Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => Some(*ptime),
                    _ => None,
                });
                if let Some(fp) = first_valid_ptime {
                    wrap_count = estimate_wrap_count(am, anc_ptime, fp, utc_tail);
                    // After disruption, prev_ptime chain is broken
                    prev_ptime = None;
                }
            }
            is_confident = false;
            if debug {
                eprintln!(
                    "PASS1 disruption (FIFO reset) at pkt={} utc_jump={:.1}",
                    pkt_idx, utc_tail - prev_utc_tail
                );
            }
        }

        // ── Process events: wrap tracking + SEC extraction + rough MET ──
        let mut pkt_confident: Vec<bool> = Vec::with_capacity(events.len());
        let mut pkt_mets: Vec<f64> = Vec::with_capacity(events.len());

        // Track ptime gaps within this packet for silent-drop detection
        let mut pkt_ptime_gaps: Vec<i64> = Vec::new();

        for (evt_idx, event) in events.iter().enumerate() {
            match event {
                Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => {
                    // ── Stream wrap detection with ghost rejection ──
                    // A "ghost event" is a CRC collision with random ptime.
                    // When a wrap is detected (ptime drops > MOD/2), we defer
                    // the decision: if the NEXT event's ptime is close to the
                    // pre-wrap value (within MOD/3), the drop was caused by a
                    // ghost and the wrap is rejected.
                    if pending_wrap {
                        // Confirm or reject the pending wrap
                        if let Some(pre) = pre_wrap_ptime {
                            let back_to_pre = (*ptime as i64 - pre as i64).unsigned_abs();
                            if back_to_pre < PTIME_MOD / 3 {
                                // Current ptime is close to the pre-wrap value →
                                // the "wrap" was caused by a ghost. Undo it.
                                wrap_count -= 1;
                                if debug {
                                    eprintln!(
                                        "PASS1 ghost_reject pkt={} pre_wrap={} cur={} (undid wrap)",
                                        pkt_idx, pre, *ptime
                                    );
                                }
                            }
                        }
                        pending_wrap = false;
                        pre_wrap_ptime = None;
                    }

                    if let Some(prev) = prev_ptime {
                        let diff = *ptime as i64 - prev as i64;
                        if diff < -(PTIME_MOD as i64 / 2) {
                            // Tentative wrap — commit immediately but mark
                            // for possible rollback at the next event.
                            pre_wrap_ptime = Some(prev);
                            wrap_count += 1;
                            pending_wrap = true;
                        }
                        pkt_ptime_gaps.push(diff);
                    }

                    // ── SEC anchor check ──
                    if let Pack::Second { stime, ptime: sec_ptime } = event {
                        let met = *stime as f64 + offset;
                        let normal_accept = (met - utc_tail).abs() < 2.0;
                        let continuity_accept = last_accepted_stime.map_or(false, |prev_st| {
                            *stime > prev_st && *stime <= prev_st + 60
                        });
                        if normal_accept || continuity_accept {
                            // Re-anchor: reset wrap_count
                            let old_anc_met = anc_met;
                            let old_anc_ptime = anc_ptime;
                            anc_met = Some(met);
                            anc_ptime = *sec_ptime;
                            let old_wrap = wrap_count;
                            wrap_count = 0;
                            last_accepted_stime = Some(*stime);
                            pending_wrap = false;

                            // Events in this packet before the SEC that were
                            // computed with a different wrap_count will be fixed
                            // by the SEC bracket post-correction at the end of pass1.

                            // A SEC is clean if its packet is not corrupted (CRC ok).
                            // SEC provides absolute time reference via stime,
                            // so it doesn't need prior confidence to be trustworthy.
                            let crc_clean = (n_error as f64) < 109.0 * 0.5;

                            anchors.push(AnchorInfo {
                                pkt_idx,
                                evt_idx,
                                met,
                                ptime: *sec_ptime,
                            });
                            anchor_is_clean.push(crc_clean);

                            // Any valid SEC in a non-corrupted packet restores confidence
                            if crc_clean {
                                is_confident = true;
                            }

                            if debug && continuity_accept && !normal_accept {
                                eprintln!(
                                    "PASS1 SEC_CONT pkt={} stime={} accepted via continuity",
                                    pkt_idx, stime
                                );
                            }
                        }
                    }

                    // ── Compute rough MET ──
                    if let Some(am) = anc_met {
                        let raw_delta = *ptime as i64 - anc_ptime as i64;
                        let total = raw_delta + wrap_count * PTIME_MOD as i64;
                        let met = am + total as f64 * 2e-6 + MET_CORRECTION;
                        pkt_mets.push(met);
                    } else {
                        pkt_mets.push(f64::NAN);
                    }

                    pkt_confident.push(is_confident);
                    prev_ptime = Some(*ptime);
                }
                Pack::Error => {
                    // CRC errors don't contribute to wrap tracking
                }
            }
        }

        // ── Aggressive saturation detection (packet-level) ──
        let mut is_saturated = false;

        // 1. CRC error rate > 50%
        if n_error as f64 > 109.0 * 0.5 {
            is_saturated = true;
            if debug {
                eprintln!(
                    "PASS1 saturated (CRC) pkt={} n_error={}/109",
                    pkt_idx, n_error
                );
            }
        }

        // 2. Inter-packet interval anomaly
        if !is_saturated && !pkt_mets.is_empty() {
            let pkt_met_min = pkt_mets.iter().copied().filter(|m| !m.is_nan()).fold(f64::INFINITY, f64::min);
            let pkt_met_max_cur = pkt_mets.iter().copied().filter(|m| !m.is_nan()).fold(f64::NEG_INFINITY, f64::max);

            if let Some(prev_max) = prev_pkt_met_max {
                let gap = pkt_met_min - prev_max;
                if gap > 0.0 {
                    // Update baseline as running average of normal intervals
                    if let Some(bl) = baseline_interval {
                        if gap < bl * 100.0 {
                            // Normal gap → update baseline (exponential moving average)
                            baseline_interval = Some(bl * 0.9 + gap * 0.1);
                        } else {
                            // Anomalous gap → saturation
                            is_saturated = true;
                            if debug {
                                eprintln!(
                                    "PASS1 saturated (gap) pkt={} gap={:.6} baseline={:.6}",
                                    pkt_idx, gap, bl
                                );
                            }
                        }
                    } else {
                        baseline_interval = Some(gap);
                    }
                }
            }

            if pkt_met_max_cur.is_finite() {
                prev_pkt_met_max = Some(pkt_met_max_cur);
            }
        }

        // 3. Poisson anomaly: large ptime gap within packet
        if !is_saturated && pkt_ptime_gaps.len() >= 2 {
            // Use ptime gaps (in ticks) to check for anomalous intervals
            let positive_gaps: Vec<f64> = pkt_ptime_gaps.iter()
                .map(|&g| {
                    // Normalize: if gap is negative (wrap), add PTIME_MOD
                    let normalized = if g < 0 { g + PTIME_MOD as i64 } else { g };
                    normalized as f64 * 2e-6 // convert to seconds
                })
                .filter(|&g| g > 0.0 && g < WRAP_PERIOD)
                .collect();

            if positive_gaps.len() >= 2 {
                // Estimate lambda from filtered gaps (exclude outliers)
                let mut sorted_gaps = positive_gaps.clone();
                sorted_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let median_gap = sorted_gaps[sorted_gaps.len() / 2];
                let lambda = 1.0 / median_gap;

                for &dt in &positive_gaps {
                    let log_p = -lambda * dt / std::f64::consts::LN_10;
                    if log_p < -10.0 {
                        is_saturated = true;
                        if debug {
                            eprintln!(
                                "PASS1 saturated (Poisson) pkt={} dt={:.6} log10p={:.1}",
                                pkt_idx, dt, log_p
                            );
                        }
                        break;
                    }
                }
            }
        }

        // If saturated, mark all subsequent events as not confident
        if is_saturated {
            is_confident = false;
            // Also mark events in THIS packet as not confident
            for c in pkt_confident.iter_mut() {
                *c = false;
            }
        }

        confident.push(pkt_confident);
        rough_mets.push(pkt_mets);
        prev_utc_tail = utc_tail;
    }

    if debug {
        let n_clean = anchor_is_clean.iter().filter(|&&c| c).count();
        eprintln!(
            "PASS1 complete: {} anchors ({} clean), {} packets",
            anchors.len(), n_clean, n_packets
        );
    }

    // ── SEC intra-packet post-correction ──
    // When a ptime wrap occurs BEFORE a SEC in the same packet, events
    // between the wrap and the SEC are computed with wrap_count=N (from
    // the old anchor), but they should be close to the SEC's time.
    // For each SEC, check events in the SAME PACKET: if any event's MET
    // is ~1 wrap above the SEC's MET, subtract one WRAP_PERIOD.
    // Also serves as safety net for ghost-induced errors near SECs.
    {
        let mut n_corrections = 0u32;
        for anc in &anchors {
            let sec_met = anc.met + MET_CORRECTION;
            for met in rough_mets[anc.pkt_idx].iter_mut() {
                if met.is_nan() {
                    continue;
                }
                let diff = *met - sec_met;
                // Event should be within ±0.5 wrap of the SEC (same packet ≈ same time).
                // If it's ~1 wrap too high, correct it.
                if diff > WRAP_PERIOD * 0.5 {
                    let n = (diff / WRAP_PERIOD).round() as i64;
                    if n > 0 && n <= 2 {
                        *met -= n as f64 * WRAP_PERIOD;
                        n_corrections += 1;
                    }
                } else if diff < -WRAP_PERIOD * 0.5 {
                    let n = (-diff / WRAP_PERIOD).round() as i64;
                    if n > 0 && n <= 2 {
                        *met += n as f64 * WRAP_PERIOD;
                        n_corrections += 1;
                    }
                }
            }
        }
        if debug && n_corrections > 0 {
            eprintln!(
                "PASS1 sec_intra_pkt: {} event corrections applied",
                n_corrections
            );
        }
    }

    Pass1Result {
        anchors,
        anchor_is_clean,
        confident,
        rough_mets,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pass 2: Bidirectional anchor reconstruction
// ─────────────────────────────────────────────────────────────────────────────

fn pass2_reconstruct(
    sci_data: &SciFile,
    pass1: &Pass1Result,
    offset: f64,
    debug: bool,
) -> Vec<Vec<f64>> {
    let n_packets = sci_data.ccsds.len();
    let mut result: Vec<Vec<f64>> = vec![Vec::new(); n_packets];

    if pass1.anchors.is_empty() {
        return result;
    }

    // Collect clean anchor indices
    let clean_indices: Vec<usize> = pass1.anchor_is_clean.iter()
        .enumerate()
        .filter(|&(_, c)| *c)
        .map(|(i, _)| i)
        .collect();

    // If no clean anchors, fall back to all anchors (mark as suspicious)
    let working_indices = if clean_indices.is_empty() {
        if debug {
            eprintln!("PASS2 WARNING: no clean anchors, falling back to all anchors");
        }
        (0..pass1.anchors.len()).collect::<Vec<_>>()
    } else {
        clean_indices
    };

    // Pre-parse all packets (events needed for both forward and backward passes)
    let all_events: Vec<Vec<Pack>> = sci_data.ccsds.iter().map(|c| parse_events(c)).collect();

    // For each packet, find the best anchor and compute MET via directional wrap tracking.
    // Strategy: for each packet, find the nearest clean anchor on each side,
    // then use the closer one (or both for cross-validation on uncertain events).

    // Build a sorted list of (pkt_idx, anchor_index_in_working) for binary search
    let anchor_pkts: Vec<usize> = working_indices.iter().map(|&i| pass1.anchors[i].pkt_idx).collect();

    for pkt_idx in 0..n_packets {
        let n_evt = pass1.rough_mets[pkt_idx].len();
        if n_evt == 0 {
            continue;
        }

        // Pass1's streaming with ghost rejection + SEC intra-packet correction
        // is reliable for all non-NaN events. Only fall through to pass2
        // for events that have no anchor (NaN rough_mets).
        if pass1.rough_mets[pkt_idx].iter().all(|m| !m.is_nan()) {
            result[pkt_idx] = pass1.rough_mets[pkt_idx].clone();
            continue;
        }

        // Find nearest clean anchors (left and right)
        let pos = anchor_pkts.partition_point(|&p| p <= pkt_idx);
        let left_anchor = if pos > 0 { Some(working_indices[pos - 1]) } else { None };
        let right_anchor = if pos < working_indices.len() { Some(working_indices[pos]) } else { None };

        // If this packet contains an anchor, prefer it
        let self_anchor = if pos > 0 && pass1.anchors[working_indices[pos - 1]].pkt_idx == pkt_idx {
            Some(working_indices[pos - 1])
        } else if pos < working_indices.len() && pass1.anchors[working_indices[pos]].pkt_idx == pkt_idx {
            Some(working_indices[pos])
        } else {
            None
        };

        // Choose primary anchor: self > nearer side
        let primary = self_anchor.or_else(|| {
            match (left_anchor, right_anchor) {
                (Some(l), Some(r)) => {
                    let ld = pkt_idx - pass1.anchors[l].pkt_idx;
                    let rd = pass1.anchors[r].pkt_idx - pkt_idx;
                    if ld <= rd { Some(l) } else { Some(r) }
                }
                (Some(l), None) => Some(l),
                (None, Some(r)) => Some(r),
                (None, None) => None,
            }
        });

        let Some(primary_idx) = primary else {
            continue;
        };

        let anc = &pass1.anchors[primary_idx];
        let is_forward = anc.pkt_idx <= pkt_idx; // anchor is to the left → scan forward

        // Compute MET via directional wrap tracking from anchor to this packet
        let pkt_mets = if is_forward {
            forward_wrap_track(sci_data, &all_events, anc, pkt_idx, offset)
        } else {
            backward_wrap_track(sci_data, &all_events, anc, pkt_idx, offset)
        };

        // Cross-validate with secondary anchor for uncertain events
        let secondary = if is_forward { right_anchor } else { left_anchor };
        if let Some(sec_idx) = secondary {
            if sec_idx != primary_idx {
                let sec_anc = &pass1.anchors[sec_idx];
                let sec_is_forward = sec_anc.pkt_idx <= pkt_idx;
                let sec_mets = if sec_is_forward {
                    forward_wrap_track(sci_data, &all_events, sec_anc, pkt_idx, offset)
                } else {
                    backward_wrap_track(sci_data, &all_events, sec_anc, pkt_idx, offset)
                };

                // Check consistency for uncertain events
                if debug && pkt_mets.len() == sec_mets.len() {
                    for (i, (m1, m2)) in pkt_mets.iter().zip(sec_mets.iter()).enumerate() {
                        if !m1.is_nan() && !m2.is_nan() {
                            let diff = (m1 - m2).abs();
                            if diff > 10e-6 {
                                eprintln!(
                                    "PASS2 XCHECK pkt={} evt={} fwd={:.6} bwd={:.6} diff={:.6}",
                                    pkt_idx, i, m1, m2, diff
                                );
                            }
                        }
                    }
                }
            }
        }

        result[pkt_idx] = pkt_mets.into_iter().filter(|m| !m.is_nan()).collect();
    }

    result
}

/// Forward wrap tracking: from anchor to target packet, scanning left→right.
/// Re-anchors on valid SEC events and rejects ghost-induced false wraps.
fn forward_wrap_track(
    sci_data: &SciFile,
    all_events: &[Vec<Pack>],
    anchor: &AnchorInfo,
    target_pkt: usize,
    offset: f64,
) -> Vec<f64> {
    let mut wrap_count: i64 = 0;
    let mut prev_ptime: Option<u64> = None;
    let mut pending_wrap = false;
    let mut pre_wrap_ptime: Option<u64> = None;
    let mut anc_met = anchor.met;
    let mut anc_ptime = anchor.ptime;

    let start_pkt = anchor.pkt_idx;
    let mut target_mets: Vec<f64> = Vec::new();

    for pkt_idx in start_pkt..=target_pkt {
        let events = &all_events[pkt_idx];
        let utc_tail = get_utc_tail(&sci_data.ccsds[pkt_idx]);

        if pkt_idx > start_pkt {
            let prev_ut = get_utc_tail(&sci_data.ccsds[pkt_idx - 1]);
            if detect_disruption(prev_ut, utc_tail) {
                let first_ptime = events.iter().find_map(|e| match e {
                    Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => Some(*ptime),
                    _ => None,
                });
                if let Some(fp) = first_ptime {
                    wrap_count = estimate_wrap_count(anc_met, anc_ptime, fp, utc_tail);
                    prev_ptime = None;
                    pending_wrap = false;
                }
            }
        }

        let is_target = pkt_idx == target_pkt;

        for event in events {
            match event {
                Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => {
                    // Ghost rejection: if pending wrap, check next event
                    if pending_wrap {
                        if let Some(pre) = pre_wrap_ptime {
                            let back = (*ptime as i64 - pre as i64).unsigned_abs();
                            if back < PTIME_MOD / 3 {
                                wrap_count -= 1;
                            }
                        }
                        pending_wrap = false;
                        pre_wrap_ptime = None;
                    }

                    // Wrap detection
                    if let Some(prev) = prev_ptime {
                        let diff = *ptime as i64 - prev as i64;
                        if diff < -(PTIME_MOD as i64 / 2) {
                            pre_wrap_ptime = Some(prev);
                            wrap_count += 1;
                            pending_wrap = true;
                        }
                    }

                    // Re-anchor on valid SEC
                    if let Pack::Second { stime, ptime: sec_ptime } = event {
                        let met = *stime as f64 + offset;
                        if (met - utc_tail).abs() < 2.0 {
                            anc_met = met;
                            anc_ptime = *sec_ptime;
                            wrap_count = 0;
                            pending_wrap = false;
                        }
                    }

                    if is_target {
                        let raw_delta = *ptime as i64 - anc_ptime as i64;
                        let total = raw_delta + wrap_count * PTIME_MOD as i64;
                        target_mets.push(anc_met + total as f64 * 2e-6 + MET_CORRECTION);
                    }

                    prev_ptime = Some(*ptime);
                }
                Pack::Error => {}
            }
        }
    }

    target_mets
}

/// Backward wrap tracking: from anchor to target packet, scanning right→left.
/// Re-anchors on valid SEC events and rejects ghost-induced false wraps.
fn backward_wrap_track(
    sci_data: &SciFile,
    all_events: &[Vec<Pack>],
    anchor: &AnchorInfo,
    target_pkt: usize,
    offset: f64,
) -> Vec<f64> {
    let mut wrap_count: i64 = 0;
    let mut next_ptime: Option<u64> = None;
    let mut pending_unwrap = false;
    let mut pre_unwrap_ptime: Option<u64> = None;
    let mut anc_met = anchor.met;
    let mut anc_ptime = anchor.ptime;

    let start_pkt = anchor.pkt_idx;
    let mut target_mets: Vec<f64> = Vec::new();

    for pkt_idx in (target_pkt..=start_pkt).rev() {
        let events = &all_events[pkt_idx];
        let utc_tail = get_utc_tail(&sci_data.ccsds[pkt_idx]);

        if pkt_idx < start_pkt {
            let next_ut = get_utc_tail(&sci_data.ccsds[pkt_idx + 1]);
            if detect_disruption(utc_tail, next_ut) {
                let last_ptime = events.iter().rev().find_map(|e| match e {
                    Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => Some(*ptime),
                    _ => None,
                });
                if let Some(lp) = last_ptime {
                    wrap_count = estimate_wrap_count(anc_met, anc_ptime, lp, utc_tail);
                    next_ptime = None;
                    pending_unwrap = false;
                }
            }
        }

        let is_target = pkt_idx == target_pkt;

        let mut pkt_mets_rev: Vec<f64> = Vec::new();
        for event in events.iter().rev() {
            match event {
                Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => {
                    // Ghost rejection (backward)
                    if pending_unwrap {
                        if let Some(pre) = pre_unwrap_ptime {
                            let back = (*ptime as i64 - pre as i64).unsigned_abs();
                            if back < PTIME_MOD / 3 {
                                wrap_count += 1; // undo the -1
                            }
                        }
                        pending_unwrap = false;
                        pre_unwrap_ptime = None;
                    }

                    // Backward wrap detection
                    if let Some(next) = next_ptime {
                        let diff = next as i64 - *ptime as i64;
                        if diff < -(PTIME_MOD as i64 / 2) {
                            pre_unwrap_ptime = Some(next);
                            wrap_count -= 1;
                            pending_unwrap = true;
                        }
                    }

                    // Re-anchor on valid SEC
                    if let Pack::Second { stime, ptime: sec_ptime } = event {
                        let met = *stime as f64 + offset;
                        if (met - utc_tail).abs() < 2.0 {
                            anc_met = met;
                            anc_ptime = *sec_ptime;
                            wrap_count = 0;
                            pending_unwrap = false;
                        }
                    }

                    if is_target {
                        let raw_delta = *ptime as i64 - anc_ptime as i64;
                        let total = raw_delta + wrap_count * PTIME_MOD as i64;
                        pkt_mets_rev.push(anc_met + total as f64 * 2e-6 + MET_CORRECTION);
                    }

                    next_ptime = Some(*ptime);
                }
                Pack::Error => {}
            }
        }

        if is_target {
            pkt_mets_rev.reverse();
            target_mets = pkt_mets_rev;
        }
    }

    target_mets
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// SEC-anchored 时间重建。
///
/// 使用两阶段方法：先预扫描找出所有 SEC 锚点和间隔异常，
/// 然后对每段事件选可信方向的锚点 + ptime 单调性计 wrap，一次算对。
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

    // Pass 1: Forward scan — extract anchors, confidence, rough METs
    let pass1 = pass1_scan(sci_data, offset, debug);

    if pass1.anchors.is_empty() {
        return vec![Vec::new(); n_packets];
    }

    // Pass 2: Bidirectional anchor reconstruction
    let result = pass2_reconstruct(sci_data, &pass1, offset, debug);

    result
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
    fn test_streaming_wrap_tracking_no_wrap() {
        // Simulate a sequence of increasing ptimes with no wrap
        let ptimes = [100, 200, 300, 400, 500];
        let anc_met = 292.0;
        let anc_ptime = 100u64;

        let mut wrap_count: i64 = 0;
        let mut prev_ptime: Option<u64> = None;
        let mut mets = Vec::new();

        for &ptime in &ptimes {
            if let Some(prev) = prev_ptime {
                if (ptime as i64 - prev as i64) < -(PTIME_MOD as i64 / 2) {
                    wrap_count += 1;
                }
            }
            let raw_delta = ptime as i64 - anc_ptime as i64;
            let total = raw_delta + wrap_count * PTIME_MOD as i64;
            mets.push(anc_met + total as f64 * 2e-6 + MET_CORRECTION);
            prev_ptime = Some(ptime);
        }

        // No wraps, so MET should be strictly increasing
        for i in 1..mets.len() {
            assert!(mets[i] > mets[i - 1], "MET should be increasing");
        }
        assert_eq!(wrap_count, 0);

        // First event (ptime=100) is anchor itself → delta=0
        let expected_first = anc_met + MET_CORRECTION;
        assert!((mets[0] - expected_first).abs() < 1e-9);
    }

    #[test]
    fn test_streaming_wrap_tracking_with_wrap() {
        // Simulate ptime wrap: 524000 → 200 (drop > MOD/2 = 262144)
        let ptimes = [100000u64, 200000, 400000, 524000, 200];
        let anc_met = 292.0;
        let anc_ptime = 100000u64;

        let mut wrap_count: i64 = 0;
        let mut prev_ptime: Option<u64> = None;
        let mut mets = Vec::new();

        for &ptime in &ptimes {
            if let Some(prev) = prev_ptime {
                if (ptime as i64 - prev as i64) < -(PTIME_MOD as i64 / 2) {
                    wrap_count += 1;
                }
            }
            let raw_delta = ptime as i64 - anc_ptime as i64;
            let total = raw_delta + wrap_count * PTIME_MOD as i64;
            mets.push(anc_met + total as f64 * 2e-6 + MET_CORRECTION);
            prev_ptime = Some(ptime);
        }

        assert_eq!(wrap_count, 1, "should detect one wrap");
        // MET should still be monotonically increasing
        for i in 1..mets.len() {
            assert!(mets[i] > mets[i - 1], "MET should be increasing after wrap at i={}", i);
        }

        // Last event: ptime=200, wrap_count=1
        // delta = 200 - 100000 + 1*524288 = 424488
        let expected_last = anc_met + 424488.0 * 2e-6 + MET_CORRECTION;
        assert!((mets[4] - expected_last).abs() < 1e-9);
    }

    #[test]
    fn test_bidirectional_wrap_consistency() {
        // Forward: anchor at ptime=100000, events ascending then wrapping
        let ptimes = [100000u64, 200000, 400000, 524000, 200, 100000];
        let anc_met = 292.0;
        let anc_ptime = 100000u64;

        // Forward pass
        let mut fwd_wraps: i64 = 0;
        let mut prev: Option<u64> = None;
        let mut fwd_mets = Vec::new();
        for &ptime in &ptimes {
            if let Some(p) = prev {
                if (ptime as i64 - p as i64) < -(PTIME_MOD as i64 / 2) {
                    fwd_wraps += 1;
                }
            }
            let raw = ptime as i64 - anc_ptime as i64;
            fwd_mets.push(anc_met + (raw + fwd_wraps * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION);
            prev = Some(ptime);
        }

        // Backward pass from the last event (use it as reverse anchor)
        // The last event's MET from forward is known
        let last_met = fwd_mets[ptimes.len() - 1];
        let last_ptime = ptimes[ptimes.len() - 1];

        let mut bwd_wraps: i64 = 0;
        let mut next: Option<u64> = None;
        let mut bwd_mets = vec![0.0f64; ptimes.len()];
        for i in (0..ptimes.len()).rev() {
            let ptime = ptimes[i];
            if let Some(n) = next {
                if (n as i64 - ptime as i64) < -(PTIME_MOD as i64 / 2) {
                    bwd_wraps -= 1;
                }
            }
            let raw = ptime as i64 - last_ptime as i64;
            bwd_mets[i] = last_met + (raw + bwd_wraps * PTIME_MOD as i64) as f64 * 2e-6;
            next = Some(ptime);
        }

        // Forward and backward should agree within floating-point tolerance
        for i in 0..ptimes.len() {
            let diff = (fwd_mets[i] - bwd_mets[i]).abs();
            assert!(diff < 1e-9, "fwd/bwd mismatch at i={}: fwd={:.9} bwd={:.9} diff={:.9}", i, fwd_mets[i], bwd_mets[i], diff);
        }
    }

    #[test]
    fn test_estimate_wrap_count_basic() {
        let anc_met = 100.0;
        let anc_ptime = 100000u64;
        // Target is ~2 wrap periods later, same ptime → 2 wraps
        let target_ptime = 100000u64;
        // Expected MET = 100 + (0 + 2*524288)*2e-6 + 4.0 = 106.097152
        // utc_tail ≈ MET - MET_CORRECTION - 0.5
        let target_utc_tail = anc_met + 2.0 * WRAP_PERIOD - 0.5;

        let n = estimate_wrap_count(anc_met, anc_ptime, target_ptime, target_utc_tail);
        assert_eq!(n, 2, "should estimate 2 wraps");
    }

    #[test]
    fn test_detect_disruption() {
        assert!(!detect_disruption(0.0, 100.0)); // prev=0 → no check
        assert!(!detect_disruption(100.0, 101.0)); // small jump
        assert!(detect_disruption(100.0, 104.0)); // >3s jump
    }
}
