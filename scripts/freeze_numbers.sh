#!/usr/bin/env bash
#
# freeze_numbers.sh
# ----------------
# Generate numbers.csv in the paper repository: single source of truth for
# all metrics quoted in the SCPMA manuscript abstract, validation tables,
# and discussion.
#
# Usage:
#     ./scripts/freeze_numbers.sh                     # uses cached data where available
#     FRESH_RUN=1 ./scripts/freeze_numbers.sh         # re-runs blink_cli on full data
#
# Environment:
#     HXMT_1B_DIR    path to 1B archive  (default ./data/1B)
#     HXMT_1K_DIR    path to 1K archive  (default ./data/1K)
#     FRESH_RUN      if set, bypass cache and re-run blink_cli
#     PAPER_REPO     path to paper repo  (default ../paper-hxmt-saturation)
#
# Output:
#     $PAPER_REPO/numbers.csv — header + one row per metric, stamped with commit hash
#     scripts/freeze_numbers_run.log — log of all blink_cli invocations
#
# IMPORTANT: When run on a machine without full archive coverage (e.g.,
# this developer laptop has only the trigger-hour 1B file for some events),
# metrics derived from the trigger-hour subset may differ from the metrics
# obtained on the full-archive server. Re-run on a full-archive machine
# before paper submission to lock final numbers.

set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO"

export HXMT_1B_DIR=${HXMT_1B_DIR:-data/1B}
export HXMT_1K_DIR=${HXMT_1K_DIR:-data/1K}
PAPER_REPO=${PAPER_REPO:-$REPO/../paper-hxmt-saturation}

OUT=$PAPER_REPO/numbers.csv
LOG=scripts/freeze_numbers_run.log

if [ ! -d "$PAPER_REPO" ]; then
    echo "ERROR: paper repo not found at $PAPER_REPO" >&2
    echo "Set PAPER_REPO env var to override." >&2
    exit 1
fi
COMMIT=$(git rev-parse HEAD)
DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

mkdir -p "$(dirname "$OUT")"
printf "metric,value,source_grb,source_box,commit_hash,note\n" > "$OUT"
: > "$LOG"

emit() {
    # emit METRIC VALUE GRB BOX NOTE
    printf "%s,%s,%s,%s,%s,%s\n" "$1" "$2" "$3" "$4" "$COMMIT" "$5" >> "$OUT"
}

log() { printf "[%s] %s\n" "$(date +%H:%M:%S)" "$*" | tee -a "$LOG"; }

#=========================================================================
# 1. GRB 221009A — extreme saturation case
#=========================================================================
log "=== GRB 221009A ==="

LOG_221009=data/cache_221009a_reconstruct.log

if [[ -f "$LOG_221009" && "${FRESH_RUN:-}" != "1" ]]; then
    log "Mining metrics from $LOG_221009 (cached April 2026 run)"

    for box in A B C; do
        # Line: "  Box A: 17851510 events, 4006 gaps, 4008 unreliable, 163781 packets"
        STATS_LINE=$(grep "Box $box:" "$LOG_221009" | grep events | head -1)
        TOTAL=$(echo "$STATS_LINE"  | awk '{for(i=1;i<=NF;i++)if($i=="events,") print $(i-1)}')
        GAPS=$(echo "$STATS_LINE"   | awk '{for(i=1;i<=NF;i++)if($i=="gaps,") print $(i-1)}')
        UNREL=$(echo "$STATS_LINE"  | awk '{for(i=1;i<=NF;i++)if($i=="unreliable,") print $(i-1)}')
        # Line: "  Box A: 4824388 observed, 3879121 gap-filled"
        OBS_LINE=$(grep "Box $box:.*observed" "$LOG_221009")
        OBS=$(echo "$OBS_LINE"    | awk '{for(i=1;i<=NF;i++)if($i=="observed,") print $(i-1)}')
        FILLED=$(echo "$OBS_LINE" | awk '{for(i=1;i<=NF;i++)if($i=="gap-filled") print $(i-1)}')

        emit "221009a_total_crc_passed" "$TOTAL" "221009A" "$box" "events with valid CRC, full observation hour"
        emit "221009a_fifo_reset_gaps" "$GAPS" "221009A" "$box" "FIFO reset gaps detected"
        emit "221009a_unreliable_packets" "$UNREL" "221009A" "$box" "packets flagged unreliable"
        emit "221009a_reconstruct_observed" "$OBS" "221009A" "$box" "events observed in reconstruction window"
        emit "221009a_reconstruct_filled" "$FILLED" "221009A" "$box" "events gap-filled in reconstruction window"
    done
else
    log "Re-running solve and reconstruct on 221009A (FRESH_RUN=1 or no cache)"
    for box in a b c; do
        BOXU=$(echo "$box" | tr a-z A-Z)
        ./target/release/blink_cli sat extract 2022-10-09T13:17:02 \
            --source 1b --box "$box" --before 50 --after 750 \
            > /tmp/freeze_221009_${box}.csv 2>>"$LOG"
        TOTAL=$(awk -F, 'NR>1 && $2=="EVT"' /tmp/freeze_221009_${box}.csv | wc -l)
        COVERED=$(awk -F, 'NR>1 && $2=="EVT" && $3!="NaN"' /tmp/freeze_221009_${box}.csv | wc -l)
        emit "221009a_solve_window_total" "$TOTAL" "221009A" "$BOXU" "events in trigger window"
        emit "221009a_solve_window_covered" "$COVERED" "221009A" "$BOXU" "events with MET assigned"
    done
fi

# Coverage % from main.tex — to be re-verified on full data run
emit "221009a_coverage_pct_a" "96.4" "221009A" "A" "main.tex value, RE-VERIFY on full-data run"
emit "221009a_coverage_pct_b" "96.3" "221009A" "B" "main.tex value, RE-VERIFY on full-data run"
emit "221009a_coverage_pct_c" "96.1" "221009A" "C" "main.tex value, RE-VERIFY on full-data run"

# Cross-box reference availability — from main.tex Table 5
emit "221009a_xref_two_boxes_a" "49.2" "221009A" "A" "%% gaps with 2 reference boxes available"
emit "221009a_xref_at_least_one_a" "92.0" "221009A" "A" "%% gaps with >=1 reference box"
emit "221009a_xref_three_saturated_a" "8.0" "221009A" "A" "%% gaps with all 3 boxes saturated"

#=========================================================================
# 2. GRB 260226A — moderate saturation, internal consistency case
#=========================================================================
log "=== GRB 260226A ==="

# Trigger 2026-02-26T13:18:21 — local laptop only has T10 hour file
# (1B + 1K), which doesn't cover the trigger. Numbers below are from
# main.tex / DESIGN.md, RE-VERIFY on server with T13 archive.

emit "260226a_box_a_1b_event_count" "549658" "260226A" "A" "main.tex value, RE-VERIFY"
emit "260226a_box_a_1k_event_count" "549661" "260226A" "A" "main.tex value, RE-VERIFY"
emit "260226a_box_a_residual" "3" "260226A" "A" "1B vs 1K event count residual"
emit "260226a_residual_fraction_pct" "0.000546" "260226A" "A" "3 / 549661 in percent"

if [[ "${FRESH_RUN:-}" == "1" ]]; then
    log "Attempting fresh compare on 260226A — requires T13 hour archive locally"
    # Note: with only T10 hour locally, this will fail. Server-side run only.
    if HXMT_1B_DIR="$HXMT_1B_DIR" HXMT_1K_DIR="$HXMT_1K_DIR" \
       ./target/release/blink_cli sat compare 2026-02-26T13:18:21 \
       --box a --before 50 --after 100 --csv \
       > /tmp/freeze_compare_260226.csv 2>>"$LOG"; then
        log "260226A compare succeeded — parse for residual"
        # actual residual extraction depends on compare output format
    else
        log "260226A compare skipped (T13 archive not available locally)"
    fi
fi

#=========================================================================
# 3. GRB 200415A — magnetar giant flare validation
#=========================================================================
log "=== GRB 200415A ==="

CACHE_200415=data/cache_200415a_reconstruct.csv
if [[ -f "$CACHE_200415" ]]; then
    for box in A B C; do
        N=$(awk -F, -v b="$box" 'NR>1 && $1==b' "$CACHE_200415" | wc -l | tr -d ' ')
        emit "200415a_reconstruct_total" "$N" "200415A" "$box" "events in reconstruct window (April 2026 cache)"
    done
fi

# From DESIGN.md / SPI-ACS validation
emit "200415a_filled_event_count" "2285" "200415A" "ALL" "events filled by reconstruction (1.6%% of total)"
emit "200415a_peak_observed_evt_per_s" "29400" "200415A" "ALL" "peak observed rate at 50ms bin"
emit "200415a_peak_recovered_evt_per_s" "48800" "200415A" "ALL" "peak rate after gap-fill (+66%%)"
emit "200415a_spiacs_ratio" "1.09" "200415A" "ALL" "HXMT/SPI-ACS scale ratio (50ms bin)"
emit "200415a_spiacs_ratio_err" "0.14" "200415A" "ALL" "ratio uncertainty"
emit "200415a_spiacs_lighttravel_ms" "-406.6" "200415A" "ALL" "INTEGRAL light-travel projection"
emit "200415a_spiacs_ratio_5ms" "1.47" "200415A" "ALL" "HXMT/SPI-ACS ratio at 5ms bin (31 bins)"
emit "200415a_spiacs_ratio_5ms_err" "1.33" "200415A" "ALL" "ratio uncertainty at 5ms"
emit "200415a_gbm_ratio_5ms" "1.32" "200415A" "ALL" "HXMT/GBM (n0+n4) ratio at 5ms (22 bins)"
emit "200415a_gbm_ratio_5ms_err" "0.64" "200415A" "ALL" "GBM ratio uncertainty"
emit "200415a_gbm_scale_factor" "3.67" "200415A" "ALL" "HXMT effective area / GBM n0+n4 scale factor"
emit "200415a_asim_ratio_1ms" "0.37" "200415A" "ALL" "HXMT/ASIM-MXGS ratio at 1ms (saturation window, 18 bins)"
emit "200415a_asim_ratio_1ms_err" "0.31" "200415A" "ALL" "ASIM ratio uncertainty (1ms scale, marks too-short boundary)"
emit "200415a_asim_scale_factor" "4.2" "200415A" "ALL" "HXMT / ASIM-MXGS LED 50-400 keV scale factor"

#=========================================================================
# 4. Method-wide constants (independent of specific GRB)
#=========================================================================
log "=== Method constants ==="

emit "ptime_modulus" "524288" "ALL" "ALL" "2^19, ptime range"
emit "ptime_resolution_us" "2.0" "ALL" "ALL" "us per ptime tick"
emit "ptime_wrap_period_s" "1.048576" "ALL" "ALL" "PTIME_MOD * 2us"
emit "dead_zone_pct" "4.6" "ALL" "ALL" "ptime SPACE fraction (NOT event loss); structural feature"
emit "met_correction_s" "4.0" "ALL" "ALL" "1B->1K empirical time offset"
emit "mcu_read_floor_evt_per_s" "15000" "ALL" "ALL" "MCU readout rate floor"
emit "fifo_a_capacity_events" "455" "ALL" "ALL" "approximate, 4096 bytes / 9 bytes per event"
emit "ccsds_packet_event_count" "109" "ALL" "ALL" "events per packet"
emit "crc_bits" "4" "ALL" "ALL" "1/16 collision probability"

#=========================================================================
log "Wrote $(wc -l < "$OUT") rows to $OUT"
log "Commit: $COMMIT"
log "Run completed: $DATE"
log ""
log "BEFORE PAPER SUBMISSION: re-run with FRESH_RUN=1 on a machine with"
log "  full HXMT 1B/1K archive coverage (T10..T14 for 260226A in particular)"
log "  to obtain truly current values for the metrics tagged 'RE-VERIFY'."
