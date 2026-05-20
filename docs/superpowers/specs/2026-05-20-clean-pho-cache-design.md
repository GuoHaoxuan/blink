# Clean PHO-Verification Cache (2020-H1)

**Date**: 2026-05-20
**Scope**: Build a single cleaned parquet cache from `per_sec_parquet/` covering 2020-01-01 to 2020-06-30, suitable for verifying simpler-coefficient PHO reconstruction models. Old `cache_training.py` / `partition_cache.py` / `train_cache.parquet` / `perdet_npz/` are discarded — this design replaces them.

## Motivation

The previous PHO model `PHO ≈ b + c_pure·Sci_pure + c_ACD·Sci_ACD + β·Wide + γ·Large` (5 coefficients, fit per `(box, det)`) is suspected to overfit — too many free parameters relative to the underlying physics. We want to test simpler forms (3-coef, 2-coef, fixed-ratio variants) on a tightly scoped, high-quality subset that minimises nuisance variance.

Two principles drive the data choice:

1. **Short time span** (6 months) to avoid year-scale parameter drift (PMT outgassing, HV setpoint adjustments, gain drift).
2. **Clean geometry & time** — equatorial belt (avoid radiation belts and high-latitude charged particle flux), SAA excluded (geometric box), and any GBM-triggered burst second excluded.

Within that window, the cache retains all engineering counters needed to fit any subset of the candidate models — column inclusion is driven by analysis flexibility, not a specific model form.

## Inputs

| Source | Location (server) | Used for |
|---|---|---|
| Per-second parquet | `/scratchfs/gecam/guohx/blink/per_sec_parquet/YYYYMMDD.parquet` | Raw per `(date, box, det, met_sec)` rows |
| GBM trigger catalog | Fetched from HEASARC `https://heasarc.gsfc.nasa.gov/W3Browse/fermi/fermigtrig.html` (`fermigtrig`) → cached locally as `n_below_study/gbm_triggers.parquet` | Burst-second exclusion |

## Time window

**2020-01-01T00:00:00 UTC ≤ date ≤ 2020-06-30T23:59:59 UTC** (inclusive on both ends, 182 calendar days).

## Filter chain

Applied in this order. Each stage's row count is logged.

### Stage 1 — Detector state

- `L_cycles > 50_000` (livetime > 0.8 s; reject sub-second boundary samples)
- `HV ∈ (-1100, -900)` (detector high-voltage operating range; HV outside this means the detector was being ramped or off)

### Stage 2 — Data integrity

- `HV`, `Lat`, `Lon` not NaN (orbit/HV gaps surface as NaN in extracted parquet; ~4% of some days)
- All raw counters `≥ 0`: `PHO`, `OOC`, `Wide`, `Large`, `Dt`, `Sci_094`, `Sci_pure_094`, `Sci_ACD1_094`, `Sci_ACDN_094`, `Sci_1s`, `Sci_pure_1s`, `Sci_ACD1_1s`, `Sci_ACDN_1s` (negative means extraction bug)
- Partition invariant: `Sci_pure_{w} + Sci_ACD1_{w} + Sci_ACDN_{w} == Sci_{w}` for both `w ∈ {094, 1s}`

### Stage 3 — Spatial filter

- `|Lat| < 3.0°` (equatorial belt, avoids radiation belts and high-latitude particle bombardment)
- `NOT (Lon ∈ [-90°, +30°])` (SAA geometric exclusion; SAA central box spans ~`Lon ∈ [-90°, +30°], Lat ∈ [-50°, +5°]`, and the Lat side of the box is subsumed by the equatorial `|Lat| < 3°` constraint above, so only the Lon predicate is needed)

### Stage 4 — Burst filter

For each row with `met_sec = T`:

- Reject if `∃ trigger ∈ GBM` such that `|T − trigger.met_sec| ≤ 300` (i.e. within ±5 min of any GBM trigger)

Trigger times are converted from GBM MET (seconds since 2001-01-01 TT) to HXMT MET (seconds since 2012-01-01 UTC) using the standard offset. The catalog is fetched once, cached as parquet, and loaded into memory as a sorted `int64` array for `np.searchsorted` lookups.

**HXMT's own `tgfs.json` catalog is intentionally NOT used** — it is known to have quality issues (per user; not the source of truth for burst rejection).

### Stage 5 — Cross-detector completeness

Group by `(date, box, met_sec)`. After Stages 1–4, a second is kept only if **all 6 detectors** in that box survived. Group by `(date, met_sec)`: kept only if **all 3 boxes** are present.

Rationale: downstream model fits use box-level and group-level sums (e.g. `group_rate = sci_sec_total / livetime`); if a detector or box is missing, those sums silently underestimate.

## Derived columns

Computed after the filter chain, before write:

- `length = L_cycles · 16e-6` (seconds of live time)
- `*_rate = raw_count / length` for: `PHO`, `OOC`, `Wide`, `Large`, `Sci_094`, `Sci_1s`, `Sci_pure_094`, `Sci_pure_1s`, `Sci_ACD1_094`, `Sci_ACD1_1s`, `Sci_ACDN_094`, `Sci_ACDN_1s`
- `dt_frac = Dt / L_cycles` (fractional dead time)
- `Sci_ACD_094 = Sci_ACD1_094 + Sci_ACDN_094`, `Sci_ACD_1s = Sci_ACD1_1s + Sci_ACDN_1s`
- `Sci_ACD_rate_094`, `Sci_ACD_rate_1s`

## Output

**Path**: `n_below_study/clean_2020H1.parquet` (zstd compression)

**Granularity**: one row per `(date, box, det, met_sec)`, same as input.

**Estimated size**: ~5400 clean-minute/day estimate × 60 sec/min × 182 days × 18 dets × ~60 cols × ~4 B/cell, conservatively ~200 MB on disk after zstd, ~3 GB in RAM as pandas DataFrame.

### Column list

| Group | Columns |
|---|---|
| Identity | `date` (string `YYYY-MM-DD`), `box` (categorical `A/B/C`), `det` (int8 0–5), `met_sec` (int64) |
| Geometry (sanity) | `Lat`, `Lon` (float32) |
| Detector state | `L_cycles` (int32), `length` (float32), `HV` (float32), `dt_frac` (float32) |
| Raw counters | `PHO`, `OOC`, `Wide`, `Large`, `Dt`, `Sci_094`, `Sci_pure_094`, `Sci_ACD1_094`, `Sci_ACDN_094`, `Sci_ACD_094`, `Sci_1s`, `Sci_pure_1s`, `Sci_ACD1_1s`, `Sci_ACDN_1s`, `Sci_ACD_1s` (int32) |
| Rates | `pho_rate`, `ooc_rate`, `wide_rate`, `large_rate`, `sci_rate_094`, `sci_rate_1s`, `scipure_rate_094`, `scipure_rate_1s`, `acd1_rate_094`, `acd1_rate_1s`, `acdn_rate_094`, `acdn_rate_1s`, `acd_rate_094`, `acd_rate_1s` (float32) |

Total: ~38 columns.

## Implementation

**Single script**: `scripts/build_clean_cache.py`

**Modules** (within the single file):

| Module | Responsibility | LoC |
|---|---|---|
| `BurstCatalog` (class) | Fetch GBM catalog from HEASARC if not cached; expose sorted MET array; provide `any_within(met_sec_array, ±300s) -> bool[]` via `np.searchsorted` | ~80 |
| `apply_filters(df, catalog) -> df` | Sequentially apply Stages 1–5; log row count after each stage | ~120 |
| `derive_columns(df) -> df` | Compute `length`, all `*_rate`, `dt_frac`, `Sci_ACD_*` | ~40 |
| `process_one_day(date_str, out_dir) -> Path` | Load daily parquet → filter → derive → write `partial/{date}.parquet` | ~30 |
| `main()` | Iterate 182 days with `multiprocessing.Pool(processes=8)`, concat all partials, write final parquet, run assertions | ~60 |

**Dependencies**: `pandas`, `pyarrow`, `numpy`, `requests` (HEASARC fetch), `astropy.time` (MET conversions if needed).

**Single hlogin node, no hep_sub** — 8-way local parallel via `multiprocessing.Pool`, NFS-bound throughput. Estimated wall time 5–10 min.

## Failure handling

| Failure | Behavior |
|---|---|
| Daily parquet missing | Skip, log warning. Continue to next day. |
| GBM catalog fetch fails | Hard fail — abort entire run. No silent fallback. |
| Filter chain leaves 0 rows for a day | Skip partial write for that day, log info. |
| Single-worker process crashes (`multiprocessing` exception) | Pool propagates exception → main aborts. Partial files remain for inspection. |
| Final assertions fail | Output file still written, but log loud error so user notices. |

**Atomicity**: write to `n_below_study/clean_2020H1.parquet.tmp` first, then `os.rename` to final path on success.

## Sanity assertions (post-build)

The script asserts these immediately after writing, and exits non-zero on any failure:

1. `len(df) >= 1_000_000` — conservative lower bound (~50 min/day equatorial × 60 × 182 days × 18 dets × ~0.5 retention ≈ 5M expected; 1M is the floor)
2. `(df["Lat"].abs() < 3.0).all()` — Stage 3 enforced
3. `(~((df["Lon"] >= -90) & (df["Lon"] <= 30))).all()` — Stage 3 enforced (all kept rows lie outside the SAA Lon box)
4. For each `w ∈ ["094", "1s"]`: `(df[f"Sci_pure_{w}"] + df[f"Sci_ACD1_{w}"] + df[f"Sci_ACDN_{w}"] == df[f"Sci_{w}"]).all()`

## Smoke test workflow

Before the full 182-day run:

1. Run `process_one_day("20200115")` standalone — single representative mid-window day
2. Print row counts per filter stage
3. Print column dtypes / NaN counts / range for each derived rate
4. If anything looks off, iterate on filter thresholds or derived expressions before unleashing full run

## Out of scope

- **Model fitting itself** — this spec produces the cache; PHO model verification (which simpler model forms to test, RMS comparisons, plots) is a separate downstream task
- **Other time windows** — only 2020-H1 here. Same script can be re-pointed to different dates trivially (one CLI argument), but other windows are not validated
- **GECAM catalog** — irrelevant (post-launch 2020-12)
- **HXMT internal `tgfs.json`** — intentionally excluded (per-user quality concern)
- **Per-(box, det) NPZ partitioning** — old `perdet_npz/` is dropped; the 200 MB single parquet loads in seconds, no need to pre-split
