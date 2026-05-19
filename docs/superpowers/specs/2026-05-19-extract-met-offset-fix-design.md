# Extract — HE_Eng MET Offset and Segment-File Fix

**Date**: 2026-05-19
**Scope**: Bug fix to `scripts/extract_per_sec_day.py`. Two issues observed in
parquet output. Re-run **all 3260 days** in the archive after fix.

## Problem

Production extract has ~1-2% rows with duplicate `(box, det, met_sec)` keys.
QA scan of 96 days (90 Q1 + 6 random non-Q1 sample) shows **85.4% of days have
at least one duplicate**; **35% of days have non-monotonic met_sec** within
`(box, det)` groups. The non-Q1 sample confirms the bug spans the full
archive — affected days include 20200510, 20211009, 20220601, 20260314.

Root cause has two independent components:

### Component 1 — Anomalous offset in some 1B HE_Eng files

`extract_per_sec_day.py` computes `met_float = Time + offset + 4.0`, where
`offset = UTC_Last_Bdc[0] − sTime_Last_Bdc[0]` is read from each file's HE_Eng
table. Normal offset drifts ±2 s/day. Some 1B files have offsets that deviate
by **~800 s** from neighbouring hours.

Example (20220115 box A port 0766):

| Hour | offset                | met_sec range        | Note            |
|------|-----------------------|----------------------|-----------------|
| 0    | 179821369             | [316828803, 316832402] | normal        |
| 1    | 179821369             | [316832403, 316836012] | normal        |
| **2**| **179820567**         | **[316835211, 316838800]** | **−802 s outlier** |
| 3    | 179821369             | [316839603, 316842872] | normal        |
| 4–23 | 179821367–179821369   | normal               | natural drift |

Hour 2 file's first 801 rows then collide with hour 1's met_sec range, producing
duplicates with NaN HV/Lat (1K aux lookup misses because the rows live at the
wrong met_sec). The remaining 2788 rows from hour 2 are misplaced ~800 s
earlier than they should be.

Across all 2022 Q1 parquets, ~1.88% of `(box, det, met_sec)` keys are affected.

### Component 2a — Hour-boundary cycle overlap (discovered during deployment)

Some 1B HE_Eng hour files contain rows whose met_sec overlaps the next hour's
range. Example on 20220115: hour 7 file has N=4300 rows (>3600) and its last
row's met_sec equals hour 8's first row's met_sec. The two rows are distinct
HE_Eng cycles (different PHO/OOC counts) but both share the same met_sec.

This is independent of the offset bug — even after offset correction, ~2 dup
keys per box × 18 dets = 36 rows per day remain at hour boundaries.

**Fix:** after concat in `extract_day`, `drop_duplicates(subset=['box','det','met_sec'], keep='last')`. Since hours are processed in order (0 → 23), the "last" row is from hour N+1's file — which is correct because hour N+1's HE-Evt file covers that met_sec (the hour N "ghost" row had Sci_1s=0 because hour N's HE-Evt didn't reach that far).

### Component 2 — Multi-segment hour files use earliest segment

`find_he_eng_path` returns `sorted(glob)[0]` — the alphabetically first match.
When the 1B archive provides multiple segments per hour (e.g.
`HXMT_1B_0766_20220115T030000_G040183_000_004.fits` and
`...T030000_G040183_001_004.fits`), the `_000_` segment is selected.

Inspection shows the later `_NNN_` segment is the more-complete version. For
hour 3 of 20220115 box A:

| File                | N (rows) | Time range                  |
|---------------------|---------:|-----------------------------|
| `_000_004.fits`     |    2477  | [137018230, 137021499]      |
| `_001_004.fits`     |    3591  | [137018230, 137021830]      |

Same start, `_001_` runs ~330 s longer. Selecting `_000_` silently drops
~1100 rows / hour on segmented hours.

## Fix

### Change 1 — `find_he_eng_path` selects latest segment

Replace `return Path(matches[0])` with `return Path(matches[-1])`. Highest
`_NNN_` segment is the more-complete version.

No-op for hours with a single segment (the most common case).

### Change 2 — Robust offset detection and override

Pre-scan all 24 hours' HE_Eng offsets per `(box, date)` before processing rows.
Compute a robust day-level reference offset (median of all available hours).
For each hour:

- If `|file_offset − day_median| ≤ 10 s` → use file's own offset (preserves
  natural drift).
- Else → override with the median of immediate neighbour hours that pass the
  threshold (fallback to day median if no good neighbours).
- Emit a `WARN` log line on override, including raw offset, override offset,
  and hour.

Threshold of 10 s comfortably exceeds observed natural drift (±2 s/day) while
catching the multi-hundred-second outliers.

### Implementation surface

- `find_he_eng_path` — one-line change
- `read_he_eng` — add optional `override_offset: int | None` parameter; when
  provided, use it instead of `UTC_Last_Bdc - sTime_Last_Bdc` for the `offset`
  field returned in the dict.
- `extract_day` (or `_box_hour_arrays` caller) — before the per-hour loop,
  scan the 24 file headers cheaply (just read row 0 of HE_Eng), compute
  per-hour `effective_offset` map, pass into each `read_he_eng` call.

Total expected delta: ~40-60 lines including helper.

## Tests

New tests in `tests/test_extract_per_sec.py`:

1. **Segment picker**: synthetic glob with `_000_` and `_001_` filenames —
   `find_he_eng_path` returns `_001_`.

2. **Offset override**:
   - Pre-scanner identifies outlier hour from synthetic 24-hour offsets where
     one is `−800 s` from median.
   - Override map replaces that hour's offset with neighbour median.

3. **Integration on real data** (gated by `require_file` if 1B archive
   accessible): run `extract_day("20220115")` to a temp dir; assert
   - `df.duplicated(['box', 'det', 'met_sec']).sum() == 0`
   - All 24 hours per box have rows (no missing hour due to segment bug).
   - WARN line present for hour 2 box A.

## Re-run

After fix lands and tests pass:

1. Push to server.
2. Delete all old parquets (`rm per_sec_parquet/*.parquet`).
3. Re-submit via `./scripts/submit_extract_jobs.sh` (full default range
   2017-06-15 → today).
   ~3260 days × 50 concurrent slots × ~15 min/day ≈ **16 hours cluster wall**.
4. Spot-verify a sample of days from each year: dedup count is zero.
5. Re-run QA scan on output to confirm:
   - 0 days with non-monotonic groups
   - 0 days with dup keys
   - High-NaN days (where 1K aux is corrupt) still show same NaN pattern
     (this is expected — corruption is source-side, not fixable in extract)

## Other observations (no fix, just documented)

QA scan found three categories of issues that are **source-data problems**,
handled correctly by extract via NaN-fill:

1. **Zero-byte 1K aux FITS files.** E.g.,
   `HXMT_20220129T00_HE-HV_FFFFFF_V1_1K.FITS` is all zero bytes on disk
   (likely a 1K pipeline write failure). Produces 100% NaN HV for hour 0 of
   that day across all 3 boxes. Cannot be recovered — file has no content.
2. **Missing 1K HE-Evt for specific hours.** E.g., 20220323 hours 8-23 have
   no HE-Evt file → 100% Sci_1s NaN for those hours. HE_Eng / HE-HV / Orbit
   present.
3. **Missing 1B box ports for specific days.** E.g., 20211009 has no port
   1781 (Box C) data → entire box absent in parquet (12 box-det groups
   instead of 18).

None of these are extract bugs. Filter / downstream consumers should treat
NaN values as missing-data sentinels and exclude such rows.

## Out of scope

- `find_1k_aux_path` glob ordering (uses `_V<n>_` selector with regex, already
  picks max version).
- Other potential 1B archive issues (truncated FITS, missing fields). Existing
  WARN logging already covers those — no change needed.
- Source-data corruption (zero-byte aux files, missing HE-Evt hours, missing
  box ports). Documented under "Other observations" — these are upstream
  archive issues, not extract bugs.
