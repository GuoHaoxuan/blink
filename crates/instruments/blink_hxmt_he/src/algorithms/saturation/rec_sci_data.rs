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
/// When `force_normal` is true, the normal (threshold) path is used even if
/// `elapsed` is large. This is used during saturation when a SEC event was
/// accepted via stime continuity — the anchor is fresh relative to the FIFO
/// events even though `utc_tail` has advanced far beyond.
///
/// Immune to multi-detector ptime non-monotonicity (per-event independent).
const WRAP_THRESHOLD: i64 = 10000; // ~20ms in ptime ticks

/// Estimate the base wrap count for a packet using its median ptime.
///
/// The stale path's per-event best-of-3 is ambiguous when events are near
/// (k + 0.5) × WRAP_PERIOD from utc_tail. The median ptime is typically
/// far from any wrap boundary, making the estimation robust.
///
/// Returns the base wrap count `n_base` such that:
///   met ≈ anchor + (raw_delta + n_base * PTIME_MOD) * 2e-6 + MET_CORRECTION
/// for events with ptime near the packet's median.
#[inline]
fn estimate_packet_wraps(
    median_ptime: u64,
    anchor_ptime: u64,
    anchor: f64,
    utc_tail: f64,
) -> i64 {
    let elapsed = utc_tail - anchor;
    let raw_delta = median_ptime as i64 - anchor_ptime as i64;
    let n_est = ((elapsed - raw_delta as f64 * 2e-6) / WRAP_PERIOD)
        .round()
        .max(0.0) as i64;

    let mut best_n = n_est;
    let mut best_err = f64::MAX;
    for n in [n_est - 1, n_est, n_est + 1] {
        if n < 0 {
            continue;
        }
        let met = anchor + (raw_delta + n * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        let err = (met - MET_CORRECTION - utc_tail).abs();
        if err < best_err {
            best_err = err;
            best_n = n;
        }
    }
    best_n
}

/// Like `estimate_packet_wraps`, but allows negative wrap counts.
///
/// Used for backward SEC correction where events happen BEFORE the anchor
/// (fresh SEC), so the wrap count relative to the anchor is negative.
/// Also returns the confidence margin: how close the best candidate is
/// to the decision boundary (0.0 = right on boundary, 0.5 = maximum
/// confidence). Values below ~0.1 indicate ambiguous wrap count.
#[inline]
fn estimate_packet_wraps_signed(
    median_ptime: u64,
    anchor_ptime: u64,
    anchor: f64,
    utc_tail: f64,
) -> (i64, f64) {
    // Use +0.5 bias: utc_tail = floor(onboard_time), so midpoint of the
    // bin (utc_tail + 0.5) is a better estimate than the floor.
    let elapsed = utc_tail + 0.5 - anchor;
    let raw_delta = median_ptime as i64 - anchor_ptime as i64;
    let n_est = ((elapsed - raw_delta as f64 * 2e-6) / WRAP_PERIOD)
        .round() as i64; // No .max(0.0) — allow negative

    let target = utc_tail + MET_CORRECTION + 0.5;
    let mut best_n = n_est;
    let mut best_err = f64::MAX;
    let mut second_err = f64::MAX;
    for n in [n_est - 1, n_est, n_est + 1] {
        let met = anchor + (raw_delta + n * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        let err = (met - target).abs();
        if err < best_err {
            second_err = best_err;
            best_err = err;
            best_n = n;
        } else if err < second_err {
            second_err = err;
        }
    }
    // Confidence: how much better the best is vs second-best, normalized
    // to [0, 0.5]. Near 0 = ambiguous, near 0.5 = certain.
    let margin = (second_err - best_err) / (2.0 * WRAP_PERIOD);
    (best_n, margin.min(0.5))
}

/// SEC-anchored MET computation with multi-wrap support.
///
/// When `force_normal` is true, uses the threshold path (handles ±1 wrap).
/// When false and elapsed is small (< 1.5 WRAP), also uses threshold path.
/// When false and elapsed is large, uses per-event stale path (best-of-3).
///
/// For consistent intra-packet wrap assignment during FIFO congestion,
/// prefer calling `estimate_packet_wraps` once for the packet's median
/// ptime, then use `compute_met_with_base_wraps` for each event.
#[inline]
fn compute_met_anchored(
    ptime: u64,
    anchor_ptime: u64,
    anchor: f64,
    utc_tail: f64,
    force_normal: bool,
) -> f64 {
    let raw_delta = ptime as i64 - anchor_ptime as i64;
    let elapsed = utc_tail - anchor;

    if force_normal || (elapsed < WRAP_PERIOD * 1.5 && elapsed > -0.5) {
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

/// Compute MET using a pre-determined base wrap count from the packet's
/// median ptime. Handles the single possible intra-packet wrap via threshold.
#[inline]
fn compute_met_with_base_wraps(
    ptime: u64,
    anchor_ptime: u64,
    anchor: f64,
    n_base: i64,
    median_ptime: u64,
) -> f64 {
    // raw_delta relative to anchor, same as stale path
    let raw_delta = ptime as i64 - anchor_ptime as i64;
    // delta relative to median ptime — determines if event is in the same
    // wrap as the median or ±1 wrap
    let delta_from_median = ptime as i64 - median_ptime as i64;
    let wrap_adjust = if delta_from_median < -(PTIME_MOD as i64 / 2) {
        1 // event ptime wrapped forward relative to median
    } else if delta_from_median > (PTIME_MOD as i64 / 2) {
        -1 // event ptime wrapped backward relative to median
    } else {
        0 // same wrap as median
    };
    let total = raw_delta + (n_base + wrap_adjust) * PTIME_MOD as i64;
    anchor + total as f64 * 2e-6 + MET_CORRECTION
}

/// Compute MET using a "backward" (future) SEC anchor.
///
/// After a FIFO reset, the first SEC may be stale (from FIFO contents
/// before the reset). The second SEC is fresh and correct. Events between
/// the two SECs are recomputed using the second SEC as anchor.
///
/// `approx_met` is the approximate MET from biased estimation, used to
/// determine the correct wrap count when events span multiple wraps.
#[inline]
fn compute_met_backward(
    ptime: u64,
    future_anchor_ptime: u64,
    future_anchor_met: f64,
    approx_met: f64,
) -> f64 {
    let raw_delta = ptime as i64 - future_anchor_ptime as i64;
    let raw_met = future_anchor_met + raw_delta as f64 * 2e-6 + MET_CORRECTION;
    // Determine the correct wrap offset using the approximate MET
    let k = ((raw_met - approx_met) / WRAP_PERIOD).round() as i64;
    raw_met - k as f64 * WRAP_PERIOD
}

/// Compute MET using packet-level estimation with +0.5 utc_tail bias.
///
/// Fallback for post-FIFO-reset events when no future SEC is available
/// (e.g., end of data). The +0.5 bias compensates for utc_tail being
/// floor(event_time), making the target the midpoint of the 1-second
/// utc_tail bin.
#[inline]
fn estimate_packet_wraps_biased(
    median_ptime: u64,
    anchor_ptime: u64,
    anchor: f64,
    utc_tail: f64,
) -> i64 {
    let elapsed = utc_tail + 0.5 - anchor;
    let raw_delta = median_ptime as i64 - anchor_ptime as i64;
    let n_est = ((elapsed - raw_delta as f64 * 2e-6) / WRAP_PERIOD)
        .round()
        .max(0.0) as i64;

    let target = utc_tail + MET_CORRECTION + 0.5;
    let mut best_n = n_est;
    let mut best_err = f64::MAX;
    for n in [n_est - 1, n_est, n_est + 1] {
        if n < 0 {
            continue;
        }
        let met = anchor + (raw_delta + n * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        let err = (met - target).abs();
        if err < best_err {
            best_err = err;
            best_n = n;
        }
    }
    best_n
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
    reconstruct_with_wrap_tracking_labeled(sci_data, offset, "")
}

pub fn reconstruct_with_wrap_tracking_labeled(sci_data: &SciFile, offset: f64, label: &str) -> Vec<Vec<f64>> {
    let mut result: Vec<Vec<f64>> = Vec::with_capacity(sci_data.ccsds.len());
    let mut anchor: Option<f64> = None;
    let mut anchor_ptime: u64 = 0;
    let mut anchor_pkt: usize = 0;
    let mut last_accepted_stime: Option<u64> = None;
    let debug = std::env::var("DEBUG_WRAP").is_ok();

    // Maximum number of packets since last anchor update to still consider
    // it "recent" (i.e., use the normal path). ~35 packets ≈ 1 second of
    // FIFO data at saturation event rates.
    const ANCHOR_RECENT_PKT_LIMIT: usize = 35;

    // ── FIFO congestion: median-ptime wrap tracking ──
    //
    // During FIFO congestion, utc_tail tracks real time while events are
    // delayed. The stale path's utc_tail-based n_wraps estimation is
    // biased (over-estimates by FIFO_delay / WRAP_PERIOD wraps).
    //
    // Fix: track ptime wraps between consecutive packets using each
    // packet's median ptime. Within a congestion period, the FIFO is read
    // sequentially, so consecutive packets have monotonically advancing
    // ptimes. When the median ptime wraps (large negative delta), we
    // increment a wrap counter.
    //
    // This gives the correct n_wraps without using utc_tail, eliminating
    // the FIFO delay bias entirely.
    //
    // Reset on FIFO reset (utc_tail jump > 3s): after reset, events are
    // fresh and the stale path's raw utc_tail is correct again.
    //
    // IMPORTANT: We always accumulate wrap counts when stale, but only
    // USE them (replace stale path) when utc_tail has diverged far from
    // the anchor (indicating extreme FIFO congestion). This prevents
    // wrap tracking from interfering with moderate saturation (e.g.,
    // 260226A, elapsed ~1-5s) where the stale path's utc_tail-based
    // estimation is accurate enough. For extreme saturation (e.g.,
    // 221009A, elapsed 100s+), utc_tail is far from event time and
    // wrap tracking is essential.
    //
    // Elapsed threshold: 0.0 means always use wrap tracking when stale.
    // This works because compute_met_with_base_wraps handles intra-packet
    // wrap boundaries correctly, and the initialization from the anchor
    // packet's median ptime ensures accurate inter-packet tracking.
    const WRAP_TRACKING_ELAPSED_THRESHOLD: f64 = 0.0;
    let mut congestion_wrap_count: i64 = 0;
    let mut prev_median_ptime: Option<u64> = None;
    let mut wrap_tracking_active = false;
    let mut prev_utc_tail: f64 = 0.0;
    // Phase correction: median-based wrap tracking counts rollovers of the
    // median ptime, but the time computation uses anchor_ptime (SEC's ptime).
    // When anchor_ptime and anchor_median are far apart (e.g., SEC ptime near
    // 0 but median near PTIME_MOD), median rollovers overcount by 1.
    let mut anchor_median_ptime: Option<u64> = None;
    // After a FIFO reset (UTC_JUMP), events are fresh (no FIFO delay).
    // The stale path's utc_tail estimation is correct for them.
    // Prevent wrap tracking from re-activating until a new SEC anchor
    // is established, which properly resets congestion_wrap_count.
    let mut fifo_reset_no_wt = false;
    // After FIFO reset, the first accepted SEC establishes an anchor that
    // may be stale (the SEC event was in the FIFO before the reset). Events
    // computed with this anchor may be ~1 WRAP_PERIOD off. We buffer these
    // events and recompute them when a fresh SEC arrives (backward approach).
    // In the meantime, we use +0.5 biased estimation as a reasonable
    // approximation. The backward SEC replaces it once available.
    let mut anchor_post_fifo_reset = false;
    // Buffer of (pkt_idx, evt_idx, ptime, utc_tail) for events awaiting
    // backward correction from a fresh SEC.
    struct PendingEvent {
        pkt_idx: usize,
        evt_idx: usize,
        ptime: u64,
        utc_tail: f64,
    }
    let mut fifo_reset_pending: Vec<PendingEvent> = Vec::new();
    // When wrap tracking activates during FIFO reset recovery, record
    // the starting packet. The backward flush will later compute the
    // correct total wraps from the two SEC anchors (stale + fresh)
    // and retroactively fix wrap tracking events.
    let mut wrap_tracking_fifo_reset_start: Option<usize> = None;

    // When a second UTC_JUMP occurs before the backward flush for the
    // first has fired, we save the first cycle's state here instead
    // of dropping it. The backward flush processes all stale batches
    // when a fresh SEC finally arrives.
    struct StaleBatch {
        pending_events: Vec<PendingEvent>,
        wt_start: Option<usize>,
        wt_end_pkt: usize,
        cwc_at_end: i64,
        anchor_met: f64,
        anchor_ptime: u64,
    }
    let mut stale_batches: Vec<StaleBatch> = Vec::new();
    // Track packets corrected by backward flush so the global sort
    // doesn't undo their corrections.
    let mut backward_flushed: Vec<bool> = vec![false; sci_data.ccsds.len()];

    for (pkt_idx, ccsds) in sci_data.ccsds.iter().enumerate() {
        let utc_tail = get_utc_tail(ccsds);
        let events = parse_events(ccsds);
        let mut times = Vec::with_capacity(events.len());

        // Anchor is "recent" if set within the last few packets.
        let mut anchor_is_recent =
            anchor.is_some() && pkt_idx.saturating_sub(anchor_pkt) < ANCHOR_RECENT_PKT_LIMIT;

        // Detect FIFO reset: utc_tail jumps forward significantly.
        // After reset, events are fresh (no FIFO delay), so wrap tracking
        // from the congestion period is invalid. Fall back to stale path.
        let utc_jumped = prev_utc_tail > 0.0 && utc_tail - prev_utc_tail > 3.0;
        if utc_jumped {
            // Save current FIFO reset cycle's state before clearing,
            // so the backward flush can correct it when a fresh SEC
            // finally arrives.
            if !fifo_reset_pending.is_empty() || wrap_tracking_fifo_reset_start.is_some() {
                if let Some(anc) = anchor {
                    stale_batches.push(StaleBatch {
                        pending_events: std::mem::take(&mut fifo_reset_pending),
                        wt_start: wrap_tracking_fifo_reset_start.take(),
                        wt_end_pkt: pkt_idx,
                        cwc_at_end: congestion_wrap_count,
                        anchor_met: anc,
                        anchor_ptime,
                    });
                    if debug {
                        let sb = stale_batches.last().unwrap();
                        eprintln!(
                            "UTC_JUMP pkt={}: saved stale batch #{}: {} pending, wt_start={:?}, wt_end={}, cwc={}, anc={:.3} anc_pt={}",
                            pkt_idx, stale_batches.len(), sb.pending_events.len(),
                            sb.wt_start, sb.wt_end_pkt, sb.cwc_at_end, sb.anchor_met, sb.anchor_ptime
                        );
                    }
                } else {
                    // No anchor — can't save; just clear.
                    fifo_reset_pending.clear();
                    wrap_tracking_fifo_reset_start = None;
                }
            }
            wrap_tracking_active = false;
            prev_median_ptime = None;
            congestion_wrap_count = 0;
            fifo_reset_no_wt = true;
            anchor_post_fifo_reset = false;
            if debug {
                eprintln!(
                    "UTC_JUMP pkt={} prev_ut={:.0} ut={:.0} jump={:.0} → wrap tracking reset, fifo_reset_no_wt=true",
                    pkt_idx, prev_utc_tail, utc_tail, utc_tail - prev_utc_tail
                );
            }
        }

        // Compute median ptime for wrap tracking.
        // Only trust the median if enough events pass CRC. Corrupted packets
        // may have a few random CRC-passing events whose ptimes are garbage;
        // using such a median would falsely trigger WRAP_INC.
        const MIN_EVENTS_FOR_MEDIAN: usize = 50;
        let (median_pt, n_valid_events) = if anchor.is_some() {
            let mut ptimes: Vec<u64> = events
                .iter()
                .filter_map(|e| match e {
                    Pack::Event { ptime, .. } | Pack::Second { ptime, .. } => Some(*ptime),
                    _ => None,
                })
                .collect();
            let n = ptimes.len();
            if ptimes.is_empty() {
                (None, 0)
            } else {
                ptimes.sort_unstable();
                (Some(ptimes[ptimes.len() / 2]), n)
            }
        } else {
            (None, 0)
        };

        // Activate wrap tracking (accumulation) when anchor goes stale.
        // Activate wrap tracking (accumulation) when anchor goes stale.
        // congestion_wrap_count was reset to 0 when the anchor was set.
        // prev_median_ptime was set to the anchor packet's median, so the
        // first median comparison (current packet vs anchor packet) spans
        // ALL recent packets and will detect any wraps that occurred.
        if !wrap_tracking_active && !anchor_is_recent && prev_median_ptime.is_some() && !fifo_reset_no_wt {
            wrap_tracking_active = true;
            // If wrap tracking activates while backward correction is pending,
            // stop buffering new events (wrap tracking computes correct METs
            // directly). Keep the existing pending buffer so the backward
            // flush can still correct POST_RESET_BIAS events when a fresh
            // SEC arrives.
            if anchor_post_fifo_reset {
                wrap_tracking_fifo_reset_start = Some(pkt_idx);
                if debug {
                    eprintln!(
                        "WRAP_INIT pkt={} wrap tracking active, stop buffering ({} pending events kept for backward flush)",
                        pkt_idx, fifo_reset_pending.len()
                    );
                }
                anchor_post_fifo_reset = false;
            }
            if debug {
                eprintln!(
                    "WRAP_INIT pkt={} congestion_wrap_count={} prev_med={:?}",
                    pkt_idx, congestion_wrap_count, prev_median_ptime
                );
            }
        }

        // Always accumulate wrap counts when stale (even before USE threshold).
        // Only detect forward wraps (WRAP_INC): during FIFO congestion, ptime
        // advances monotonically (sequential FIFO reads), so backward wraps
        // (large positive diff) never occur. A large positive diff at activation
        // time is normal ptime advancement during the recent period when anchor
        // ptime was near 0 — NOT a backward wrap.
        // Skip corrupted packets (< MIN_EVENTS_FOR_MEDIAN valid events) to
        // avoid false WRAP_INC from unreliable median ptimes.
        if wrap_tracking_active && !anchor_is_recent && n_valid_events >= MIN_EVENTS_FOR_MEDIAN {
            if let (Some(med), Some(prev_med)) = (median_pt, prev_median_ptime) {
                let diff = med as i64 - prev_med as i64;
                if diff < -(PTIME_MOD as i64 / 2) {
                    congestion_wrap_count += 1;
                    if debug {
                        eprintln!(
                            "WRAP_INC pkt={} med={} prev_med={} diff={} → wraps={}",
                            pkt_idx, med, prev_med, diff, congestion_wrap_count
                        );
                    }
                }
            }
        }

        // Only USE wrap tracking for time computation when utc_tail has
        // diverged far enough from anchor (extreme FIFO congestion).
        // For moderate saturation, the stale path is more reliable.
        let elapsed_from_anchor = anchor.map_or(0.0, |a| utc_tail - a);
        let use_wrap_tracking = wrap_tracking_active
            && !anchor_is_recent
            && elapsed_from_anchor > WRAP_TRACKING_ELAPSED_THRESHOLD;
        // Only update prev_median_ptime when wrap tracking is active (to
        // track inter-packet wraps) or when it hasn't been set yet.
        // During the "recent" period, we KEEP the anchor packet's median
        // so that the first wrap comparison at activation spans all recent
        // packets and catches any wraps that occurred.
        // Skip corrupted packets to avoid poisoning the median reference.
        if (wrap_tracking_active || prev_median_ptime.is_none()) && n_valid_events >= MIN_EVENTS_FOR_MEDIAN {
            if let Some(med) = median_pt {
                prev_median_ptime = Some(med);
            }
        }

        // Pre-compute packet-level base wraps for the stale path.
        // When the anchor is stale (not recent) and we're not using wrap
        // tracking, determine n_base from the median ptime to avoid
        // per-event wrap ambiguity.
        let use_stale_path = anchor.is_some()
            && !anchor_is_recent
            && !use_wrap_tracking
            && elapsed_from_anchor >= WRAP_PERIOD * 1.5;
        let mut packet_base_wraps = if use_stale_path {
            median_pt.map(|med| {
                estimate_packet_wraps(med, anchor_ptime, anchor.unwrap(), utc_tail)
            })
        } else if anchor_post_fifo_reset && anchor.is_some() && !use_wrap_tracking {
            // Post-FIFO-reset: anchor may be stale by ~1s. The normal
            // threshold path would give wrong n_wraps. Use +0.5 biased
            // estimation which targets the midpoint of the utc_tail bin,
            // compensating for utc_tail being floor(event_time).
            median_pt.map(|med| {
                let n = estimate_packet_wraps_biased(med, anchor_ptime, anchor.unwrap(), utc_tail);
                if debug {
                    eprintln!(
                        "POST_RESET_BIAS pkt={} med={} anc_pt={} anc={:.3} ut={:.0} → n_base={}",
                        pkt_idx, med, anchor_ptime, anchor.unwrap(), utc_tail, n
                    );
                }
                n
            })
        } else {
            None
        };

        // Phase correction for wrap tracking: congestion_wrap_count tracks
        // median-ptime rollovers relative to anchor_median. But the time
        // computation uses anchor_ptime. When these differ significantly
        // (e.g., SEC ptime near 0, median near PTIME_MOD), the median
        // rollover count overcounts (or undercounts).
        //
        // Correction: count how many extra wraps go from anchor_ptime to
        // anchor_median vs. direct anchor_ptime to current_median.
        let phase_correction: i64 = if use_wrap_tracking {
            if let Some(anc_med) = anchor_median_ptime {
                let delta = anc_med as i64 - anchor_ptime as i64;
                // Use 70% threshold (367002) instead of 50% (262144).
                // At 50%, Box C falsely triggers (delta=269502=51.4%)
                // when anchor and median are just slightly over half
                // apart but no actual wrap boundary crossing occurred.
                let phase_thresh = PTIME_MOD as i64 * 7 / 10;
                if delta > phase_thresh {
                    -1
                } else if delta < -phase_thresh {
                    1
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };
        let corrected_wrap_count = congestion_wrap_count + phase_correction;

        for event in &events {
            match event {
                Pack::Second { stime, ptime } => {
                    let met = *stime as f64 + offset;

                    let normal_accept = (met - utc_tail).abs() < 2.0;
                    let continuity_accept = last_accepted_stime.map_or(false, |prev| {
                        *stime > prev && *stime <= prev + 60
                    });

                    if normal_accept || continuity_accept {
                        if debug && continuity_accept && !normal_accept {
                            eprintln!(
                                "SEC_CONT pkt={} stime={} prev={} ut={:.0} anchor updated via continuity",
                                pkt_idx, stime,
                                last_accepted_stime.unwrap_or(0),
                                utc_tail
                            );
                        }

                        // When a SEC arrives after the anchor has gone stale
                        // (non-recent), the SEC is from new data, not from
                        // the pre-reset FIFO buffer. Flush pending events
                        // using this fresh SEC as backward anchor, then clear
                        // the post-reset flag. Stale SECs arrive in the first
                        // 1-2 packets (still "recent"), so this only triggers
                        // for fresh SECs.
                        if (!fifo_reset_pending.is_empty() || !stale_batches.is_empty()) && !anchor_is_recent {
                            let fresh_met = met;
                            let fresh_ptime = *ptime;

                            // ── Process stale batches from earlier FIFO resets ──
                            for sb in stale_batches.drain(..) {
                                // Phase S1-S3: recompute pending events using fresh SEC
                                if !sb.pending_events.is_empty() {
                                    let mut sb_pkt_groups: std::collections::BTreeMap<usize, Vec<usize>> =
                                        std::collections::BTreeMap::new();
                                    for (i, pe) in sb.pending_events.iter().enumerate() {
                                        sb_pkt_groups.entry(pe.pkt_idx).or_default().push(i);
                                    }
                                    let mut sb_pkt_results: std::collections::BTreeMap<usize, (i64, u64)> =
                                        std::collections::BTreeMap::new();
                                    for (&pi, indices) in &sb_pkt_groups {
                                        let mut ptimes_v: Vec<u64> = indices.iter()
                                            .map(|&i| sb.pending_events[i].ptime).collect();
                                        ptimes_v.sort_unstable();
                                        let med = ptimes_v[ptimes_v.len() / 2];
                                        let ut = sb.pending_events[indices[0]].utc_tail;
                                        let (n_base, _margin) = estimate_packet_wraps_signed(
                                            med, fresh_ptime, fresh_met, ut,
                                        );
                                        sb_pkt_results.insert(pi, (n_base, med));
                                    }
                                    if debug {
                                        eprintln!(
                                            "STALE_BATCH_FLUSH pkt={}: {} pending events in {} pkts, anc={:.3} anc_pt={}",
                                            pkt_idx, sb.pending_events.len(), sb_pkt_groups.len(),
                                            sb.anchor_met, sb.anchor_ptime,
                                        );
                                    }
                                    // Apply corrections
                                    for (&pi, &(n_base, med)) in &sb_pkt_results {
                                        let indices = &sb_pkt_groups[&pi];
                                        for &idx in indices {
                                            let pe = &sb.pending_events[idx];
                                            let corrected = compute_met_with_base_wraps(
                                                pe.ptime, fresh_ptime, fresh_met, n_base, med,
                                            );
                                            result[pe.pkt_idx][pe.evt_idx] = corrected;
                                        }
                                    }
                                }
                                // Phase S4: correct wrap tracking events from stale batch
                                if let Some(wt_start) = sb.wt_start {
                                    let total_wraps = ((fresh_met - sb.anchor_met) / 2e-6
                                        - (fresh_ptime as i64 - sb.anchor_ptime as i64) as f64)
                                        / PTIME_MOD as f64;
                                    let total_wraps = total_wraps.round() as i64;
                                    let delta = total_wraps - sb.cwc_at_end;
                                    if delta != 0 {
                                        let shift = delta as f64 * WRAP_PERIOD;
                                        if debug {
                                            eprintln!(
                                                "STALE_BATCH wt correction: wt_start={} → wt_end={}, total_wraps={} cwc={} delta={} shift={:.6}",
                                                wt_start, sb.wt_end_pkt, total_wraps, sb.cwc_at_end, delta, shift
                                            );
                                        }
                                        for pi in wt_start..sb.wt_end_pkt {
                                            for t in result[pi].iter_mut() {
                                                *t += shift;
                                            }
                                        }
                                    }
                                    // Phase S5: boundary continuity between pending and wt
                                    if !sb.pending_events.is_empty() && !result[wt_start].is_empty() {
                                        let first_wt_met = result[wt_start][0];
                                        // Find last pending packet
                                        let last_pend_pkt = sb.pending_events.iter()
                                            .map(|pe| pe.pkt_idx).max().unwrap();
                                        if !result[last_pend_pkt].is_empty() {
                                            let last_pend_met = *result[last_pend_pkt].last().unwrap();
                                            let gap = first_wt_met - last_pend_met;
                                            let n_shift = (gap / WRAP_PERIOD).round() as i64;
                                            let residual = (gap - n_shift as f64 * WRAP_PERIOD).abs();
                                            if n_shift.abs() == 1 && residual < 0.3 {
                                                let corr = n_shift as f64 * WRAP_PERIOD;
                                                if debug {
                                                    eprintln!(
                                                        "STALE_BATCH boundary fix: last_pend={:.3} first_wt={:.3} gap={:.3} n_shift={} corr={:.6}",
                                                        last_pend_met, first_wt_met, gap, n_shift, corr,
                                                    );
                                                }
                                                for pe in &sb.pending_events {
                                                    result[pe.pkt_idx][pe.evt_idx] += corr;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // ── End stale batch processing ──
                            // Group pending events by packet
                            let mut pkt_groups: std::collections::BTreeMap<usize, Vec<usize>> =
                                std::collections::BTreeMap::new();
                            for (i, pe) in fifo_reset_pending.iter().enumerate() {
                                pkt_groups.entry(pe.pkt_idx).or_default().push(i);
                            }
                            // Phase 1: compute backward MET for all packets,
                            // classify as confident or ambiguous.
                            const CONFIDENCE_THRESHOLD: f64 = 0.05;
                            struct PktResult {
                                n_base: i64,
                                n_alt: i64, // alternative wrap count
                                confident: bool,
                                median: u64,
                                representative_met: f64, // MET of first event
                            }
                            let mut pkt_results: std::collections::BTreeMap<usize, PktResult> =
                                std::collections::BTreeMap::new();
                            let mut n_confident = 0usize;
                            let mut n_ambiguous = 0usize;
                            for (&pi, indices) in &pkt_groups {
                                let mut ptimes_v: Vec<u64> = indices.iter()
                                    .map(|&i| fifo_reset_pending[i].ptime).collect();
                                ptimes_v.sort_unstable();
                                let med = ptimes_v[ptimes_v.len() / 2];
                                let ut = fifo_reset_pending[indices[0]].utc_tail;
                                let (n_base, margin) = estimate_packet_wraps_signed(
                                    med, fresh_ptime, fresh_met, ut,
                                );
                                let confident = margin > CONFIDENCE_THRESHOLD;
                                if debug && !confident {
                                    let raw_d = med as i64 - fresh_ptime as i64;
                                    let m_base = fresh_met + (raw_d + n_base * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
                                    eprintln!(
                                        "  AMBIGUOUS pi={} med={} ut={:.0} n_base={} margin={:.4} met_base={:.3}",
                                        pi, med, ut, n_base, margin, m_base
                                    );
                                }
                                // Alternative: shift by ±1 (toward the ambiguous side)
                                let n_alt = if margin <= CONFIDENCE_THRESHOLD {
                                    // Determine which direction is ambiguous
                                    let raw_delta = med as i64 - fresh_ptime as i64;
                                    let best_met = fresh_met + (raw_delta + n_base * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
                                    let target = ut;
                                    if best_met - MET_CORRECTION > target {
                                        n_base - 1 // try lower
                                    } else {
                                        n_base + 1 // try higher
                                    }
                                } else {
                                    n_base
                                };
                                // Use median ptime for representative MET (consistent with n_base)
                                let raw_delta_med = med as i64 - fresh_ptime as i64;
                                let repr_met = fresh_met + (raw_delta_med + n_base * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
                                if confident { n_confident += 1; } else { n_ambiguous += 1; }
                                pkt_results.insert(pi, PktResult {
                                    n_base, n_alt, confident, median: med,
                                    representative_met: repr_met,
                                });
                            }
                            if debug {
                                eprintln!(
                                    "BACKWARD_FLUSH pkt={} total={} confident={} ambiguous={} fresh_met={} fresh_pt={}",
                                    pkt_idx, fifo_reset_pending.len(), n_confident, n_ambiguous, fresh_met, fresh_ptime
                                );
                            }
                            // Phase 2: resolve ambiguous packets using
                            // cross-reference ("前后对照") between the biased
                            // forward estimate (stale anchor) and the backward
                            // estimate (fresh anchor). Since the two anchors
                            // have different wrap boundaries (different ptime),
                            // events ambiguous for one are often unambiguous
                            // for the other.
                            for (&pi, pr) in pkt_results.iter_mut() {
                                if pr.confident {
                                    continue;
                                }
                                let indices = &pkt_groups[&pi];
                                // Get the existing biased MET (computed from stale anchor)
                                let biased_met = if indices[0] < fifo_reset_pending.len() {
                                    let pe = &fifo_reset_pending[indices[0]];
                                    if pe.pkt_idx == pkt_idx {
                                        times[pe.evt_idx]
                                    } else {
                                        result[pe.pkt_idx][pe.evt_idx]
                                    }
                                } else {
                                    continue;
                                };
                                // Compute backward MET for both options
                                let raw_delta = pr.median as i64 - fresh_ptime as i64;
                                let met_base = fresh_met + (raw_delta + pr.n_base * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
                                let met_alt = fresh_met + (raw_delta + pr.n_alt * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
                                // Pick the backward option closest to the biased estimate
                                let err_base = (met_base - biased_met).abs();
                                let err_alt = (met_alt - biased_met).abs();
                                if err_alt < err_base {
                                    if debug {
                                        eprintln!(
                                            "  CROSSREF pi={} n_base={} → n_alt={} (biased={:.3} base={:.3} alt={:.3} err_b={:.3} err_a={:.3})",
                                            pi, pr.n_base, pr.n_alt,
                                            biased_met, met_base, met_alt,
                                            err_base, err_alt,
                                        );
                                    }
                                    pr.n_base = pr.n_alt;
                                } else if debug {
                                    eprintln!(
                                        "  CROSSREF_KEEP pi={} n_base={} (biased={:.3} base={:.3} alt={:.3} err_b={:.3} err_a={:.3})",
                                        pi, pr.n_base,
                                        biased_met, met_base, met_alt,
                                        err_base, err_alt,
                                    );
                                }
                            }
                            // Phase 3: apply corrections
                            for (&pi, pr) in &pkt_results {
                                let indices = &pkt_groups[&pi];
                                for &idx in indices {
                                    let pe = &fifo_reset_pending[idx];
                                    let corrected = compute_met_with_base_wraps(
                                        pe.ptime, fresh_ptime, fresh_met, pr.n_base, pr.median,
                                    );
                                    if pe.pkt_idx == pkt_idx {
                                        times[pe.evt_idx] = corrected;
                                    } else {
                                        result[pe.pkt_idx][pe.evt_idx] = corrected;
                                    }
                                }
                            }
                            // Mark backward-flushed packets so the global
                            // sort won't undo the correction.
                            for &pi in pkt_results.keys() {
                                if pi < backward_flushed.len() {
                                    backward_flushed[pi] = true;
                                }
                            }
                            if pkt_idx < backward_flushed.len() {
                                backward_flushed[pkt_idx] = true;
                            }
                            // Phase 4: retroactive wrap tracking correction.
                            // When wrap tracking activated during FIFO reset
                            // recovery, it used congestion_wrap_count=0. The
                            // backward flush knows the correct n_base for the
                            // last pending packet. Convert to stale-anchor
                            // equivalent and adjust wrap tracking events.
                            if let Some(wt_start) = wrap_tracking_fifo_reset_start {
                                // Compute total wraps between the two SEC anchors
                                // directly, avoiding the error-prone utc_tail-based
                                // estimates. This is exact because both SECs are
                                // ground truth.
                                let anc = anchor.unwrap();
                                let total_wraps = ((fresh_met - anc) / 2e-6
                                    - (fresh_ptime as i64 - anchor_ptime as i64) as f64)
                                    / PTIME_MOD as f64;
                                let total_wraps = total_wraps.round() as i64;
                                // congestion_wrap_count tracks rollovers detected
                                // since the stale SEC reset it to 0. The correct
                                // total should match total_wraps.
                                let delta = total_wraps - congestion_wrap_count;
                                if delta != 0 {
                                    let shift = delta as f64 * WRAP_PERIOD;
                                    if debug {
                                        eprintln!(
                                            "BACKWARD_FLUSH retroactive wt correction: wt_start={} → pkt={}, total_wraps={} cwc={} delta={} shift={:.6}",
                                            wt_start, pkt_idx, total_wraps, congestion_wrap_count, delta, shift
                                        );
                                    }
                                    for pi in wt_start..pkt_idx {
                                        for t in result[pi].iter_mut() {
                                            *t += shift;
                                        }
                                    }
                                    // Also fix events already pushed for the current packet
                                    // (events before the SEC in this packet used wrap tracking)
                                    for t in times.iter_mut() {
                                        *t += shift;
                                    }
                                    congestion_wrap_count += delta;
                                }
                                // Phase 5: boundary continuity check.
                                // The backward flush may assign wrong n_base
                                // to pending packets (off by ±1) because
                                // utc_tail is ~1s ahead of actual event time
                                // after FIFO reset. Check if the last pending
                                // packet's MET is consistent with the first
                                // wrap tracking packet's (already corrected)
                                // MET. If there's a ~WRAP_PERIOD gap, shift
                                // all pending events.
                                // Get the first wrap tracking event MET.
                                // If wt_start == pkt_idx, the current packet
                                // hasn't been pushed to result yet; use times.
                                let first_wt_times = if wt_start < pkt_idx {
                                    result[wt_start].as_slice()
                                } else {
                                    times.as_slice()
                                };
                                if !first_wt_times.is_empty() {
                                    let first_wt_met = first_wt_times[0];
                                    // Find last pending packet's representative MET
                                    if let Some((&last_pi, _)) = pkt_results.iter().next_back() {
                                        if !result[last_pi].is_empty() {
                                            let last_pend_met = *result[last_pi].last().unwrap();
                                            let gap = first_wt_met - last_pend_met;
                                            // Generalized: find integer N such that
                                            // gap - N*WRAP ≈ 0 (pending N wraps off)
                                            let n_shift = (gap / WRAP_PERIOD).round() as i64;
                                            let residual = (gap - n_shift as f64 * WRAP_PERIOD).abs();
                                            // Only correct ±1 WRAP; larger shifts are
                                            // likely genuine FIFO reset gaps, not errors.
                                            if n_shift.abs() == 1 && residual < 0.3 {
                                                let corr = n_shift as f64 * WRAP_PERIOD;
                                                if debug {
                                                    eprintln!(
                                                        "BACKWARD_FLUSH boundary fix: last_pend={:.3} first_wt={:.3} gap={:.3} n_shift={} corr={:.6}",
                                                        last_pend_met, first_wt_met, gap, n_shift, corr,
                                                    );
                                                }
                                                for (&pi, _) in &pkt_results {
                                                    for t in result[pi].iter_mut() {
                                                        *t += corr;
                                                    }
                                                }
                                                // Also fix pending events in the current packet
                                                // (pkt_idx == the SEC packet)
                                                if pkt_results.contains_key(&pkt_idx) {
                                                    for t in times.iter_mut() {
                                                        *t += corr;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                wrap_tracking_fifo_reset_start = None;
                            }
                            fifo_reset_pending.clear();
                            anchor_post_fifo_reset = false;
                        } else if anchor_post_fifo_reset && !anchor_is_recent {
                            // Fresh SEC but no pending events — just clear
                            anchor_post_fifo_reset = false;
                        }

                        anchor = Some(met);
                        anchor_ptime = *ptime;
                        anchor_pkt = pkt_idx;
                        last_accepted_stime = Some(*stime);
                        anchor_is_recent = true;
                        congestion_wrap_count = 0;
                        wrap_tracking_active = false;
                        // After FIFO reset, mark anchor as potentially stale.
                        // The flag stays set until a non-recent SEC arrives
                        // (which indicates a fresh SEC after the stale ones).
                        if fifo_reset_no_wt {
                            anchor_post_fifo_reset = true;
                        }
                        fifo_reset_no_wt = false;
                        if let Some(med) = median_pt {
                            prev_median_ptime = Some(med);
                            anchor_median_ptime = Some(med);
                        }
                        packet_base_wraps = None;
                    }

                    if let Some(anc) = anchor {
                        let evt_idx = times.len();
                        if use_wrap_tracking && !anchor_is_recent {
                            if let Some(med) = median_pt {
                                times.push(compute_met_with_base_wraps(
                                    *ptime, anchor_ptime, anc, corrected_wrap_count, med,
                                ));
                            } else {
                                let raw_delta = *ptime as i64 - anchor_ptime as i64;
                                let total = raw_delta + corrected_wrap_count * PTIME_MOD as i64;
                                times.push(anc + total as f64 * 2e-6 + MET_CORRECTION);
                            }
                        } else if let (Some(n_base), Some(med)) = (packet_base_wraps, median_pt) {
                            times.push(compute_met_with_base_wraps(
                                *ptime, anchor_ptime, anc, n_base, med,
                            ));
                        } else {
                            times.push(compute_met_anchored(
                                *ptime,
                                anchor_ptime,
                                anc,
                                utc_tail,
                                anchor_is_recent,
                            ));
                        }
                        // Buffer for backward correction from future fresh SEC
                        if anchor_post_fifo_reset {
                            fifo_reset_pending.push(PendingEvent {
                                pkt_idx, evt_idx, ptime: *ptime, utc_tail,
                            });
                        }
                    }
                }
                Pack::Event { ptime, .. } => {
                    if let Some(anc) = anchor {
                        let evt_idx = times.len();
                        if use_wrap_tracking && !anchor_is_recent {
                            if let Some(med) = median_pt {
                                times.push(compute_met_with_base_wraps(
                                    *ptime, anchor_ptime, anc, corrected_wrap_count, med,
                                ));
                            } else {
                                let raw_delta = *ptime as i64 - anchor_ptime as i64;
                                let total = raw_delta + corrected_wrap_count * PTIME_MOD as i64;
                                times.push(anc + total as f64 * 2e-6 + MET_CORRECTION);
                            }
                        } else if let (Some(n_base), Some(med)) = (packet_base_wraps, median_pt) {
                            times.push(compute_met_with_base_wraps(
                                *ptime, anchor_ptime, anc, n_base, med,
                            ));
                        } else {
                            let met = compute_met_anchored(
                                *ptime,
                                anchor_ptime,
                                anc,
                                utc_tail,
                                anchor_is_recent,
                            );
                            times.push(met);
                        }
                        // Buffer for backward correction from future fresh SEC
                        if anchor_post_fifo_reset {
                            fifo_reset_pending.push(PendingEvent {
                                pkt_idx, evt_idx, ptime: *ptime, utc_tail,
                            });
                        }
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
                "PKT {} pkt={} ut={:.0} anc={:.3} anc_pt={} el={:.3} n={} tmin={:.6} tmax={:.6} span={:.4} recent={} wt={} wraps={} phase={} eff={}",
                label, pkt_idx, utc_tail, anchor.unwrap_or(0.0), anchor_ptime,
                elapsed, times.len(), t_min, t_max, t_max - t_min, anchor_is_recent, use_wrap_tracking, congestion_wrap_count, phase_correction, corrected_wrap_count
            );
        }

        prev_utc_tail = utc_tail;
        result.push(times);
    }

    // Remaining pending events have no future fresh SEC — keep their
    // biased estimation times (the best we can do without a backward anchor).
    if !fifo_reset_pending.is_empty() {
        if debug {
            eprintln!(
                "POST_RESET_FALLBACK: {} events have no backward SEC, keeping biased estimation",
                fifo_reset_pending.len()
            );
        }
        fifo_reset_pending.clear();
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
    // multi-level packet reordering that Pass 2 cannot fully resolve. Flatten all
    // events and sort globally by time to ensure monotonicity.
    let needs_global_sort = result.iter().enumerate().any(|(i, pkt)| {
        if i == 0 || pkt.is_empty() {
            return false;
        }
        // Check if this packet's first event is before previous packet's last event
        if let Some(prev_pkt) = result.get(i - 1) {
            if !prev_pkt.is_empty() && !pkt.is_empty() {
                if pkt[0] < prev_pkt[prev_pkt.len() - 1] {
                    // Skip reversals at backward-flushed boundaries:
                    // pending (fresh SEC) vs wrap tracking (stale anchor)
                    // reversals are expected and should not trigger a sort.
                    let bf_prev = i > 0 && backward_flushed.get(i - 1).copied().unwrap_or(false);
                    let bf_curr = backward_flushed.get(i).copied().unwrap_or(false);
                    if bf_prev || bf_curr {
                        return false;
                    }
                    return true;
                }
            }
        }
        // Intra-packet reversals are handled by Pass 3 (mixed packet
        // logic) and should not trigger a global sort.
        false
    });

    if needs_global_sort {
        if debug {
            eprintln!("Global sort: sorting non-backward-flushed segments");
        }
        // Sort each contiguous segment of non-backward-flushed packets
        // independently, preserving backward-flushed packet METs.
        let n = result.len();
        let mut seg_start = 0;
        while seg_start < n {
            if seg_start < backward_flushed.len() && backward_flushed[seg_start] {
                seg_start += 1;
                continue;
            }
            let mut seg_end = seg_start + 1;
            while seg_end < n && !(seg_end < backward_flushed.len() && backward_flushed[seg_end]) {
                seg_end += 1;
            }
            let mut seg_events: Vec<f64> = result[seg_start..seg_end]
                .iter().flatten().copied().collect();
            seg_events.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut event_idx = 0;
            for pkt in result[seg_start..seg_end].iter_mut() {
                let pkt_len = pkt.len();
                if pkt_len > 0 && event_idx < seg_events.len() {
                    let end_idx = (event_idx + pkt_len).min(seg_events.len());
                    pkt.clear();
                    pkt.extend_from_slice(&seg_events[event_idx..end_idx]);
                    event_idx = end_idx;
                }
            }
            seg_start = seg_end;
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
        // 1. Large gap (> 0.3s) → FIFO reset → shift
        // 2. Small gap → file reorder → skip
        // Note: do NOT shift based on reversal size alone. Even when
        // reversal ≈ WRAP_PERIOD, a small gap means it's file reordering,
        // not FIFO reset. Shifting incorrectly breaks 260226A.
        let should_shift = gap > 0.3 || prev_pkt_max.is_nan();

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

        let met = compute_met_anchored(500100, anchor_ptime, anchor, utc_tail, false);
        let expected = 292.0 + 100.0 * 2e-6 + MET_CORRECTION;
        assert!((met - expected).abs() < 1e-9);
    }

    #[test]
    fn test_anchored_wrap() {
        let anchor = 292.0;
        let anchor_ptime = 400000u64;
        let utc_tail = 293.0; // fresh, ~1s later

        let met = compute_met_anchored(3000, anchor_ptime, anchor, utc_tail, false);
        let expected = 292.0 + (3000i64 - 400000 + PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        assert!((met - expected).abs() < 1e-9, "expected {expected}, got {met}");
    }

    #[test]
    fn test_anchored_multi_detector() {
        let anchor = 292.0;
        let anchor_ptime = 100000u64;
        let utc_tail = 292.0;

        let met = compute_met_anchored(99950, anchor_ptime, anchor, utc_tail, false);
        let expected = 292.0 + (-50.0) * 2e-6 + MET_CORRECTION;
        assert!((met - expected).abs() < 1e-9);

        let met2 = compute_met_anchored(99000, anchor_ptime, anchor, utc_tail, false);
        let expected2 = 292.0 + (-1000.0) * 2e-6 + MET_CORRECTION;
        assert!((met2 - expected2).abs() < 1e-9);
    }

    #[test]
    fn test_anchored_consistency() {
        let anchor = 292.0;
        let anchor_ptime = 100u64;
        let utc_tail = 292.0;

        let met1 = compute_met_anchored(200, anchor_ptime, anchor, utc_tail, false);
        let met2 = compute_met_anchored(300, anchor_ptime, anchor, utc_tail, false);
        let met3 = compute_met_anchored(400, anchor_ptime, anchor, utc_tail, false);
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

        let met = compute_met_anchored(251424, anchor_ptime, anchor, utc_tail, false);
        // Expected: anchor + 2.2s worth of ticks * 2μs + correction
        let expected = 292.0 + (251424i64 - 200000 + 2 * PTIME_MOD as i64) as f64 * 2e-6 + MET_CORRECTION;
        assert!(
            (met - expected).abs() < 1e-6,
            "stale anchor multi-wrap: expected {expected}, got {met}"
        );
    }

    #[test]
    fn test_anchored_force_normal_during_saturation() {
        // Simulates saturation: anchor was recently set via stime continuity,
        // but utc_tail is far ahead (FIFO delay). force_normal=true should
        // use the threshold path, ignoring the large elapsed.
        let anchor = 292.0;
        let anchor_ptime = 200000u64;
        let utc_tail = 305.0; // 13s ahead — FIFO delay during saturation

        // Event with ptime slightly ahead of anchor — should be placed at ~292.0
        let met = compute_met_anchored(200500, anchor_ptime, anchor, utc_tail, true);
        let expected = 292.0 + 500.0 * 2e-6 + MET_CORRECTION;
        assert!(
            (met - expected).abs() < 1e-9,
            "force_normal: expected {expected}, got {met}"
        );

        // Without force_normal, the stale path would pick a much larger n_wraps
        let met_stale = compute_met_anchored(200500, anchor_ptime, anchor, utc_tail, false);
        assert!(
            met_stale > met + 5.0,
            "stale path should give much larger MET: stale={met_stale}, normal={met}"
        );
    }
}
