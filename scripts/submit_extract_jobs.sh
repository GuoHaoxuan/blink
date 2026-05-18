#!/usr/bin/env bash
# Submit one hep_sub job per missing date in the per_sec_parquet/ output dir.
#
# Usage:
#   ./submit_extract_jobs.sh [START_DATE] [END_DATE] [--dry-run]
#
# Defaults:
#   START_DATE = 20170615
#   END_DATE   = $(date -u +%Y%m%d)
#
# Idempotent: skips dates whose output already exists.
# Re-run any number of times to backfill.

set -euo pipefail

START="${1:-20170615}"
END="${2:-$(date -u +%Y%m%d)}"
DRY_RUN=""
for arg in "$@"; do
    [ "$arg" = "--dry-run" ] && DRY_RUN=1
done

# Edit these for your server environment:
SCRIPT_PATH="$(realpath "$(dirname "$0")/extract_per_sec_day.py")"
PYTHON="python3"
OUTPUT_DIR="per_sec_parquet"
LOG_DIR="logs/extract"
HEPSUB_GROUP=""    # ← fill in your hep_sub group name (or pass via env)
HEPSUB_MEM="2GB"
HEPSUB_TIME="1h"

mkdir -p "$LOG_DIR" "$OUTPUT_DIR"

# Iterate dates from START to END inclusive.
cur="$START"
n_submitted=0
n_skipped=0
while [ "$cur" -le "$END" ]; do
    out="$OUTPUT_DIR/${cur}.parquet"
    if [ -s "$out" ]; then
        n_skipped=$((n_skipped + 1))
    else
        cmd="$PYTHON $SCRIPT_PATH $cur --output-dir $OUTPUT_DIR"
        if [ -n "$DRY_RUN" ]; then
            echo "[DRY] hep_sub -g $HEPSUB_GROUP -mem $HEPSUB_MEM -wt $HEPSUB_TIME \\"
            echo "      -o $LOG_DIR/${cur}.out -e $LOG_DIR/${cur}.err \"$cmd\""
        else
            hep_sub -g "$HEPSUB_GROUP" -mem "$HEPSUB_MEM" -wt "$HEPSUB_TIME" \
                    -o "$LOG_DIR/${cur}.out" -e "$LOG_DIR/${cur}.err" \
                    "$cmd"
        fi
        n_submitted=$((n_submitted + 1))
    fi
    # Advance by one day. Use a portable date arithmetic via Python.
    cur=$(python3 -c "
from datetime import date, timedelta
d = date(int('$cur'[:4]), int('$cur'[4:6]), int('$cur'[6:8])) + timedelta(days=1)
print(d.strftime('%Y%m%d'))")
done

echo "submitted=$n_submitted skipped=$n_skipped (range: $START..$END)"
