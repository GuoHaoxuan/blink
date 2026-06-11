# Clean PHO-Verification Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `n_below_study/clean_2020H1.parquet` — a single ~200MB cached parquet of clean per-second HXMT-HE engineering rows (2020-H1, |Lat|<3°, SAA-out, GBM-burst-excluded) for verifying simpler-coefficient PHO models.

**Architecture:** Single Python script `scripts/build_clean_cache.py` containing 5 internal modules (`BurstCatalog`, `apply_filters`, `derive_columns`, `process_one_day`, `run_build`/`main`). Local 8-way `multiprocessing.Pool` on one hlogin node; per-day filter+derive writes to partial parquets, then concat into final cache. Test-first: unit tests against synthetic DataFrames for all filter / derive logic; integration smoke test on real 20200115 data on server.

**Tech Stack:** Python 3 (`pandas`, `pyarrow`, `numpy`), `astropy.time` (MET conversion), `astroquery.heasarc` (GBM trigger fetch), `pytest` (tests). Server: hlogin compute node with NFS-shared `/scratchfs/gecam/guohx/blink/`.

---

## File Structure

**Create:**
- `scripts/build_clean_cache.py` — main script (~320 LoC), 5 modules + CLI
- `tests/test_build_clean_cache.py` — unit tests with synthetic DataFrames

**Modify:**
- None — `tests/conftest.py` already adds `scripts/` to import path

**Runtime artifacts (NFS only, not in git):**
- `/scratchfs/gecam/guohx/blink/n_below_study/gbm_triggers.parquet` — fetched GBM catalog (one-time)
- `/scratchfs/gecam/guohx/blink/n_below_study/clean_2020H1.parquet` — final cache (script output)
- `/scratchfs/gecam/guohx/blink/n_below_study/.partial_2020H1/{YYYYMMDD}.parquet` — temp per-day partials (cleaned up after concat)

---

## Task 1: Install astroquery + create test scaffolding

**Files:**
- Modify (Python env): install `astroquery` via uv pip
- Create: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Install astroquery in the venv**

Run:
```bash
source .venv/bin/activate && uv pip install astroquery
```
Expected: install completes; `python3 -c "from astroquery.heasarc import Heasarc"` exits 0.

- [ ] **Step 2: Create empty test file with synthetic-row builder**

Write `tests/test_build_clean_cache.py`:

```python
"""Tests for scripts/build_clean_cache.py."""
from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

# conftest.py already adds scripts/ to sys.path
# Tests will `import build_clean_cache as M` once that script exists.


# ------------------- Synthetic row builder -------------------

# Columns the per_sec_parquet schema gives us (subset relevant to this cache).
PER_SEC_COLS = [
    "date", "box", "det", "met_sec",
    "Lat", "Lon",
    "L_cycles", "HV",
    "PHO", "OOC", "Wide", "Large", "Dt",
    "Sci_094", "Sci_pure_094", "Sci_ACD1_094", "Sci_ACDN_094",
    "Sci_1s", "Sci_pure_1s", "Sci_ACD1_1s", "Sci_ACDN_1s",
]


def make_row(**overrides):
    """One canonical 'good' per-sec row that should pass all filters.

    Override any field to test edge cases.
    """
    base = dict(
        date="2020-01-15", box="A", det=0, met_sec=252633600,
        Lat=0.5, Lon=120.0,           # Pacific, far from SAA
        L_cycles=60_000, HV=-1000.0,
        PHO=100, OOC=5, Wide=20, Large=10, Dt=500,
        Sci_094=80, Sci_pure_094=70, Sci_ACD1_094=8, Sci_ACDN_094=2,
        Sci_1s=85, Sci_pure_1s=74, Sci_ACD1_1s=8, Sci_ACDN_1s=3,
    )
    base.update(overrides)
    return base


def make_df(rows):
    """Build a DataFrame from a list of dicts (output of make_row)."""
    return pd.DataFrame(rows, columns=PER_SEC_COLS)


def make_complete_second(box="A", met_sec=252633600, date="2020-01-15", **overrides):
    """Build all 6 (det 0..5) rows for one (box, met_sec) — passes Stage 5 box-completeness."""
    return [make_row(box=box, det=d, met_sec=met_sec, date=date, **overrides) for d in range(6)]


def make_complete_groupsec(met_sec=252633600, date="2020-01-15", **overrides):
    """Build 18 rows = 3 boxes × 6 dets for one met_sec — passes Stage 5 fully."""
    rows = []
    for b in "ABC":
        rows.extend(make_complete_second(box=b, met_sec=met_sec, date=date, **overrides))
    return rows


def test_builder_canonical_row_dict_has_all_columns():
    row = make_row()
    assert set(row.keys()) == set(PER_SEC_COLS)


def test_builder_complete_groupsec_makes_18_rows():
    rows = make_complete_groupsec()
    df = make_df(rows)
    assert len(df) == 18
    assert sorted(df["box"].unique().tolist()) == ["A", "B", "C"]
    assert sorted(df["det"].unique().tolist()) == [0, 1, 2, 3, 4, 5]
```

- [ ] **Step 3: Run the scaffolding tests**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v
```
Expected: 2 PASS.

- [ ] **Step 4: Commit**

```bash
git add tests/test_build_clean_cache.py
git commit -m "test: scaffold clean-cache test file with synthetic row builders"
```

---

## Task 2: BurstCatalog.any_within (in-memory logic)

**Files:**
- Create: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- BurstCatalog.any_within -------------------

def test_burstcatalog_any_within_empty_catalog():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    times = np.array([1000, 2000, 3000], dtype=np.int64)
    out = cat.any_within(times)
    assert out.tolist() == [False, False, False]


def test_burstcatalog_any_within_exact_hit():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([5000], dtype=np.int64), window_sec=300)
    out = cat.any_within(np.array([5000], dtype=np.int64))
    assert out.tolist() == [True]


def test_burstcatalog_any_within_at_boundary():
    """A trigger at T, window ±300s: T+300 and T-300 inclusive are within."""
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([5000], dtype=np.int64), window_sec=300)
    out = cat.any_within(np.array([4700, 4699, 5300, 5301], dtype=np.int64))
    assert out.tolist() == [True, False, True, False]


def test_burstcatalog_any_within_multi_trigger():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([1000, 5000, 9000], dtype=np.int64), window_sec=300)
    out = cat.any_within(np.array([1100, 5500, 9000, 3000, 7000], dtype=np.int64))
    assert out.tolist() == [True, False, True, False, False]


def test_burstcatalog_any_within_unsorted_input_triggers_sorted_internally():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([9000, 1000, 5000], dtype=np.int64), window_sec=300)
    out = cat.any_within(np.array([1100], dtype=np.int64))
    assert out.tolist() == [True]
```

- [ ] **Step 2: Run tests, expect them to fail**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k burstcatalog
```
Expected: FAIL with "ModuleNotFoundError: No module named 'build_clean_cache'".

- [ ] **Step 3: Create the script with minimal BurstCatalog**

Write `scripts/build_clean_cache.py`:

```python
#!/usr/bin/env python3
"""Build clean PHO-verification cache from per_sec_parquet.

See docs/superpowers/specs/2026-05-20-clean-pho-cache-design.md
"""
from __future__ import annotations

from dataclasses import dataclass
import numpy as np


# ============================================================
# Module 1: BurstCatalog
# ============================================================


@dataclass
class BurstCatalog:
    """In-memory store of burst trigger MET-seconds, with ±window membership test."""

    triggers_met: np.ndarray   # int64, sorted ascending
    window_sec: int            # half-width of exclusion zone (e.g. 300 = ±5 min)

    @classmethod
    def from_array(cls, met_array: np.ndarray, window_sec: int) -> "BurstCatalog":
        """Build from an unsorted MET array; sorts internally."""
        sorted_met = np.sort(met_array.astype(np.int64))
        return cls(triggers_met=sorted_met, window_sec=int(window_sec))

    def any_within(self, query_met: np.ndarray) -> np.ndarray:
        """Return bool[n] — True where query_met[i] is within ±window of any trigger.

        Algorithm: for each query T, find nearest trigger via searchsorted;
        check distance to either side neighbour is <= window.
        """
        q = np.asarray(query_met, dtype=np.int64)
        if self.triggers_met.size == 0:
            return np.zeros(q.size, dtype=bool)

        idx = np.searchsorted(self.triggers_met, q)
        left = np.clip(idx - 1, 0, self.triggers_met.size - 1)
        right = np.clip(idx, 0, self.triggers_met.size - 1)
        dist_left = np.abs(q - self.triggers_met[left])
        dist_right = np.abs(q - self.triggers_met[right])
        return (dist_left <= self.window_sec) | (dist_right <= self.window_sec)


# ============================================================
# CLI entry point (filled out in later tasks)
# ============================================================

if __name__ == "__main__":
    raise SystemExit("not yet implemented")
```

- [ ] **Step 4: Run tests, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k burstcatalog
```
Expected: all 5 burstcatalog tests PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): BurstCatalog in-memory ±window membership test"
```

---

## Task 3: BurstCatalog GBM fetch + parquet caching

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests for the cache layer**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- BurstCatalog.fetch_or_load -------------------

def test_burstcatalog_loads_from_existing_parquet(tmp_path):
    import build_clean_cache as M

    cache = tmp_path / "gbm.parquet"
    # Pretend we already fetched: write a tiny parquet with one trigger at MET 12345.
    pd.DataFrame({"trigger_met_hxmt": [12345]}).to_parquet(cache)

    cat = M.BurstCatalog.fetch_or_load(cache, window_sec=300, allow_fetch=False)
    assert cat.triggers_met.tolist() == [12345]
    assert cat.window_sec == 300


def test_burstcatalog_raises_when_missing_and_fetch_disabled(tmp_path):
    import build_clean_cache as M

    cache = tmp_path / "nonexistent.parquet"
    with pytest.raises(FileNotFoundError):
        M.BurstCatalog.fetch_or_load(cache, window_sec=300, allow_fetch=False)


def test_gbm_iso_to_hxmt_met_2020_01_01():
    """2020-01-01 00:00:00 UTC ≈ HXMT MET 252633600 (8 years × 365.25 × 86400 ≈ 252633600,
    modulo leap seconds astropy resolves)."""
    import build_clean_cache as M

    met = M.gbm_iso_to_hxmt_met("2020-01-01T00:00:00")
    # Allow 5-second tolerance to swallow any leap-second / epoch convention drift.
    assert abs(met - 252633600) < 5
```

- [ ] **Step 2: Run tests, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "burstcatalog_loads or burstcatalog_raises or gbm_iso"
```
Expected: FAIL — `BurstCatalog.fetch_or_load` and `gbm_iso_to_hxmt_met` not defined.

- [ ] **Step 3: Add fetch_or_load + gbm_iso_to_hxmt_met to the script**

Replace the entire `BurstCatalog` block in `scripts/build_clean_cache.py` (and add the helper above it):

```python
# ============================================================
# MET conversion (GBM ISO time -> HXMT MET)
# ============================================================

# HXMT MET epoch: 2012-01-01T00:00:00 UTC. We compute the offset once via astropy
# so leap-seconds are handled correctly.
_HXMT_EPOCH_ISO = "2012-01-01T00:00:00"


def gbm_iso_to_hxmt_met(iso_string: str) -> int:
    """Convert a GBM trigger time (ISO 8601 UTC) to HXMT MET seconds (int)."""
    from astropy.time import Time
    t = Time(iso_string, format="isot", scale="utc")
    epoch = Time(_HXMT_EPOCH_ISO, format="isot", scale="utc")
    return int(round((t - epoch).sec))


def _fetch_gbm_triggers_from_heasarc():
    """Query HEASARC's fermigtrig table for all triggers, return DataFrame with
    a single column 'trigger_met_hxmt' (int64 HXMT MET seconds).

    Network call; runs on a node with HTTPS access to HEASARC.
    """
    import pandas as pd
    from astroquery.heasarc import Heasarc

    heasarc = Heasarc()
    # query_region with a huge radius effectively returns the whole table
    table = heasarc.query_region(
        position="0d 0d", mission="fermigtrig", radius="180 deg", resultmax=100000
    )
    times_iso = [str(t).strip() for t in table["TIME"]]
    metsec = [gbm_iso_to_hxmt_met(s) for s in times_iso if s and s != "--"]
    return pd.DataFrame({"trigger_met_hxmt": metsec})


# ============================================================
# Module 1: BurstCatalog
# ============================================================


@dataclass
class BurstCatalog:
    """In-memory store of burst trigger MET-seconds, with ±window membership test."""

    triggers_met: np.ndarray   # int64, sorted ascending
    window_sec: int            # half-width of exclusion zone (e.g. 300 = ±5 min)

    @classmethod
    def from_array(cls, met_array: np.ndarray, window_sec: int) -> "BurstCatalog":
        sorted_met = np.sort(met_array.astype(np.int64))
        return cls(triggers_met=sorted_met, window_sec=int(window_sec))

    @classmethod
    def fetch_or_load(cls, cache_path, window_sec: int, allow_fetch: bool = True) -> "BurstCatalog":
        """Load triggers from a cached parquet file. If missing and allow_fetch=True,
        fetch from HEASARC and write the cache first.

        cache_path: pathlib.Path to a parquet file with one int64 column 'trigger_met_hxmt'.
        """
        import pandas as pd
        from pathlib import Path
        cache_path = Path(cache_path)

        if not cache_path.exists():
            if not allow_fetch:
                raise FileNotFoundError(
                    f"GBM trigger cache not found: {cache_path}. "
                    f"Re-run with allow_fetch=True (must have HEASARC HTTPS access)."
                )
            df = _fetch_gbm_triggers_from_heasarc()
            cache_path.parent.mkdir(parents=True, exist_ok=True)
            df.to_parquet(cache_path, compression="zstd")

        df = pd.read_parquet(cache_path)
        arr = df["trigger_met_hxmt"].to_numpy().astype(np.int64)
        return cls.from_array(arr, window_sec=window_sec)

    def any_within(self, query_met: np.ndarray) -> np.ndarray:
        q = np.asarray(query_met, dtype=np.int64)
        if self.triggers_met.size == 0:
            return np.zeros(q.size, dtype=bool)
        idx = np.searchsorted(self.triggers_met, q)
        left = np.clip(idx - 1, 0, self.triggers_met.size - 1)
        right = np.clip(idx, 0, self.triggers_met.size - 1)
        dist_left = np.abs(q - self.triggers_met[left])
        dist_right = np.abs(q - self.triggers_met[right])
        return (dist_left <= self.window_sec) | (dist_right <= self.window_sec)
```

- [ ] **Step 4: Run tests, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "burstcatalog or gbm_iso"
```
Expected: all 8 tests PASS (5 from Task 2 + 3 new).

- [ ] **Step 5: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): GBM trigger fetch + parquet cache layer"
```

---

## Task 4: apply_filters Stages 1-3 (detector state, integrity, spatial)

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- apply_filters stages 1-3 -------------------

def test_filter_drops_low_lcycles():
    import build_clean_cache as M
    df = make_df([
        make_row(L_cycles=60_000),    # keep
        make_row(L_cycles=50_000),    # drop (strict >)
        make_row(L_cycles=10_000),    # drop
    ])
    out = M._apply_stage1_detector_state(df)
    assert len(out) == 1
    assert out["L_cycles"].iloc[0] == 60_000


def test_filter_drops_bad_hv():
    import build_clean_cache as M
    df = make_df([
        make_row(HV=-1000),   # keep
        make_row(HV=-1100),   # drop (strict >)
        make_row(HV=-900),    # drop (strict <)
        make_row(HV=-1200),   # drop
        make_row(HV=-800),    # drop
    ])
    out = M._apply_stage1_detector_state(df)
    assert len(out) == 1
    assert out["HV"].iloc[0] == -1000


def test_filter_drops_nan_in_critical_cols():
    import build_clean_cache as M
    df = make_df([
        make_row(),
        make_row(HV=np.nan),
        make_row(Lat=np.nan),
        make_row(Lon=np.nan),
    ])
    out = M._apply_stage2_integrity(df)
    assert len(out) == 1


def test_filter_drops_negative_counters():
    import build_clean_cache as M
    df = make_df([
        make_row(),
        make_row(PHO=-1),
        make_row(Wide=-5),
        make_row(Dt=-1),
    ])
    out = M._apply_stage2_integrity(df)
    assert len(out) == 1


def test_filter_drops_sci_breakdown_mismatch():
    import build_clean_cache as M
    df = make_df([
        # Good: 80 = 70 + 8 + 2
        make_row(Sci_094=80, Sci_pure_094=70, Sci_ACD1_094=8, Sci_ACDN_094=2),
        # Bad: 80 != 70 + 8 + 3
        make_row(Sci_094=80, Sci_pure_094=70, Sci_ACD1_094=8, Sci_ACDN_094=3),
        # Bad 1s window
        make_row(Sci_1s=85, Sci_pure_1s=74, Sci_ACD1_1s=8, Sci_ACDN_1s=4),
    ])
    out = M._apply_stage2_integrity(df)
    assert len(out) == 1


def test_filter_keeps_equator_belt():
    import build_clean_cache as M
    df = make_df([
        make_row(Lat=0.0),    # keep
        make_row(Lat=2.9),    # keep
        make_row(Lat=-2.9),   # keep
        make_row(Lat=3.0),    # drop (strict <)
        make_row(Lat=10.0),   # drop
        make_row(Lat=-50.0),  # drop
    ])
    out = M._apply_stage3_spatial(df)
    assert len(out) == 3
    assert out["Lat"].abs().max() < 3.0


def test_filter_excludes_saa_lon_box():
    import build_clean_cache as M
    df = make_df([
        make_row(Lon=120.0),    # keep (Pacific)
        make_row(Lon=-120.0),   # keep (Pacific)
        make_row(Lon=-89.9),    # keep (just outside SAA)
        make_row(Lon=-90.0),    # drop (boundary, inclusive)
        make_row(Lon=0.0),      # drop (in SAA)
        make_row(Lon=30.0),     # drop (boundary, inclusive)
        make_row(Lon=30.1),     # keep
    ])
    out = M._apply_stage3_spatial(df)
    assert len(out) == 4
    assert ((out["Lon"] < -90) | (out["Lon"] > 30)).all()
```

- [ ] **Step 2: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "filter_drops or filter_keeps or filter_excludes"
```
Expected: FAIL — stage functions not defined.

- [ ] **Step 3: Implement stages 1-3**

Add to `scripts/build_clean_cache.py` before the `if __name__` block:

```python
# ============================================================
# Module 2: apply_filters (Stages 1-5)
# ============================================================

# Constants from spec
L_CYCLES_MIN = 50_000           # Stage 1: livetime > 0.8s
HV_LO, HV_HI = -1100.0, -900.0  # Stage 1: detector operating range (exclusive both sides)
LAT_MAX_ABS = 3.0               # Stage 3: equatorial belt half-width
SAA_LON_LO, SAA_LON_HI = -90.0, 30.0   # Stage 3: SAA longitude box (inclusive both sides)

_RAW_COUNTERS = [
    "PHO", "OOC", "Wide", "Large", "Dt",
    "Sci_094", "Sci_pure_094", "Sci_ACD1_094", "Sci_ACDN_094",
    "Sci_1s", "Sci_pure_1s", "Sci_ACD1_1s", "Sci_ACDN_1s",
]


def _apply_stage1_detector_state(df):
    """Keep rows with L_cycles > 50_000 AND HV strictly inside (-1100, -900)."""
    mask = (df["L_cycles"] > L_CYCLES_MIN) & (df["HV"] > HV_LO) & (df["HV"] < HV_HI)
    return df.loc[mask].copy()


def _apply_stage2_integrity(df):
    """Drop NaN in HV/Lat/Lon, negative counters, Sci-partition violations."""
    # NaN check
    mask = df[["HV", "Lat", "Lon"]].notna().all(axis=1)
    # Non-negative counters
    for c in _RAW_COUNTERS:
        mask &= (df[c] >= 0)
    # Sci breakdown invariant for both windows
    for w in ("094", "1s"):
        lhs = df[f"Sci_pure_{w}"] + df[f"Sci_ACD1_{w}"] + df[f"Sci_ACDN_{w}"]
        mask &= (lhs == df[f"Sci_{w}"])
    return df.loc[mask].copy()


def _apply_stage3_spatial(df):
    """Keep rows in equatorial belt AND outside SAA Lon box."""
    mask = (df["Lat"].abs() < LAT_MAX_ABS) & ~(
        (df["Lon"] >= SAA_LON_LO) & (df["Lon"] <= SAA_LON_HI)
    )
    return df.loc[mask].copy()
```

- [ ] **Step 4: Run, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "filter_drops or filter_keeps or filter_excludes"
```
Expected: all 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): apply_filters stages 1-3 (detector / integrity / spatial)"
```

---

## Task 5: apply_filters Stage 4 (burst exclusion)

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- apply_filters stage 4 -------------------

def test_filter_drops_rows_within_burst_window():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([252633600], dtype=np.int64), window_sec=300)
    df = make_df([
        make_row(met_sec=252633600),         # drop (exact hit)
        make_row(met_sec=252633600 + 299),   # drop (inside ±300)
        make_row(met_sec=252633600 + 300),   # drop (boundary inclusive)
        make_row(met_sec=252633600 + 301),   # keep (outside ±300)
        make_row(met_sec=252633600 - 1000),  # keep
        make_row(met_sec=252633600 + 1000),  # keep
    ])
    out = M._apply_stage4_burst(df, cat)
    assert len(out) == 3
    keeps = set(out["met_sec"].tolist())
    assert 252633600 + 301 in keeps
    assert 252633600 - 1000 in keeps
    assert 252633600 + 1000 in keeps


def test_filter_burst_empty_catalog_keeps_all():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    df = make_df([make_row(met_sec=t) for t in (1000, 2000, 3000)])
    out = M._apply_stage4_burst(df, cat)
    assert len(out) == 3
```

- [ ] **Step 2: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "filter_drops_rows_within_burst or filter_burst_empty"
```
Expected: FAIL — `_apply_stage4_burst` not defined.

- [ ] **Step 3: Implement Stage 4**

Insert into `scripts/build_clean_cache.py` after `_apply_stage3_spatial`:

```python
def _apply_stage4_burst(df, burst_catalog):
    """Drop rows whose met_sec is within ±window_sec of any GBM trigger."""
    times = df["met_sec"].to_numpy().astype(np.int64)
    drop_mask = burst_catalog.any_within(times)
    return df.loc[~drop_mask].copy()
```

- [ ] **Step 4: Run, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "filter_drops_rows_within_burst or filter_burst_empty"
```
Expected: 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): apply_filters stage 4 (GBM burst exclusion)"
```

---

## Task 6: apply_filters Stage 5 (cross-detector completeness) + orchestrator

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests for Stage 5**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- apply_filters stage 5 (completeness) -------------------

def test_completeness_keeps_full_18_row_second():
    import build_clean_cache as M
    df = make_df(make_complete_groupsec(met_sec=252633600))
    out = M._apply_stage5_completeness(df)
    assert len(out) == 18


def test_completeness_drops_second_missing_one_det():
    import build_clean_cache as M
    rows = make_complete_groupsec(met_sec=252633600)
    # Drop box-A det-3
    rows = [r for r in rows if not (r["box"] == "A" and r["det"] == 3)]
    df = make_df(rows)
    out = M._apply_stage5_completeness(df)
    assert len(out) == 0


def test_completeness_drops_second_missing_one_box():
    import build_clean_cache as M
    rows = make_complete_groupsec(met_sec=252633600)
    # Drop entire box C
    rows = [r for r in rows if r["box"] != "C"]
    df = make_df(rows)
    out = M._apply_stage5_completeness(df)
    assert len(out) == 0


def test_completeness_mixed_seconds():
    """One full second + one broken second → only the full one survives."""
    import build_clean_cache as M
    good = make_complete_groupsec(met_sec=252633600)
    bad = [r for r in make_complete_groupsec(met_sec=252633700) if r["box"] != "B"]
    df = make_df(good + bad)
    out = M._apply_stage5_completeness(df)
    assert len(out) == 18
    assert out["met_sec"].unique().tolist() == [252633600]
```

- [ ] **Step 2: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "completeness"
```
Expected: FAIL — `_apply_stage5_completeness` not defined.

- [ ] **Step 3: Implement Stage 5**

Insert into `scripts/build_clean_cache.py` after `_apply_stage4_burst`:

```python
def _apply_stage5_completeness(df):
    """Keep only (date, met_sec) groups where all 18 (3 boxes × 6 dets) rows are present."""
    counts = df.groupby(["date", "met_sec"]).size()
    full_keys = counts[counts == 18].index
    df_idx = df.set_index(["date", "met_sec"])
    keep = df_idx.index.isin(full_keys)
    return df_idx.loc[keep].reset_index()
```

- [ ] **Step 4: Run, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "completeness"
```
Expected: 4 tests PASS.

- [ ] **Step 5: Add tests for the apply_filters orchestrator**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- apply_filters orchestrator -------------------

def test_apply_filters_runs_all_stages_and_logs():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    df = make_df(make_complete_groupsec(met_sec=252633600))
    out, counts = M.apply_filters(df, cat)
    assert len(out) == 18
    assert counts["start"] == 18
    assert counts["after_stage1"] == 18
    assert counts["after_stage5"] == 18


def test_apply_filters_drops_all_when_lat_too_high():
    import build_clean_cache as M
    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    df = make_df(make_complete_groupsec(met_sec=252633600, Lat=10.0))
    out, counts = M.apply_filters(df, cat)
    assert len(out) == 0
    assert counts["after_stage3"] == 0
```

- [ ] **Step 6: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "apply_filters_runs or apply_filters_drops_all"
```
Expected: FAIL — `apply_filters` not defined.

- [ ] **Step 7: Add the apply_filters orchestrator**

Insert into `scripts/build_clean_cache.py` after `_apply_stage5_completeness`:

```python
def apply_filters(df, burst_catalog):
    """Run all 5 filter stages in order; return (filtered_df, counts_dict).

    counts_dict keys: start, after_stage1, after_stage2, after_stage3, after_stage4,
    after_stage5 — for logging row counts at each stage.
    """
    counts = {"start": len(df)}
    df = _apply_stage1_detector_state(df)
    counts["after_stage1"] = len(df)
    df = _apply_stage2_integrity(df)
    counts["after_stage2"] = len(df)
    df = _apply_stage3_spatial(df)
    counts["after_stage3"] = len(df)
    df = _apply_stage4_burst(df, burst_catalog)
    counts["after_stage4"] = len(df)
    df = _apply_stage5_completeness(df)
    counts["after_stage5"] = len(df)
    return df, counts
```

- [ ] **Step 8: Run all filter tests, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "filter or completeness or apply_filters"
```
Expected: all 15 filter-related tests PASS.

- [ ] **Step 9: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): apply_filters stage 5 + orchestrator with per-stage counts"
```

---

## Task 7: derive_columns

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- derive_columns -------------------

def test_derive_length_and_dt_frac():
    import build_clean_cache as M
    df = make_df([make_row(L_cycles=62_500, Dt=12_500)])  # 1s livetime, dt_frac=0.2
    out = M.derive_columns(df)
    assert abs(out["length"].iloc[0] - 1.0) < 1e-6
    assert abs(out["dt_frac"].iloc[0] - 0.2) < 1e-6


def test_derive_rates():
    import build_clean_cache as M
    df = make_df([make_row(L_cycles=62_500, PHO=100, Wide=50, Sci_094=80, Sci_1s=85)])
    out = M.derive_columns(df)
    # length = 1.0, so rate = count
    assert abs(out["pho_rate"].iloc[0] - 100) < 1e-3
    assert abs(out["wide_rate"].iloc[0] - 50) < 1e-3
    assert abs(out["sci_rate_094"].iloc[0] - 80) < 1e-3
    assert abs(out["sci_rate_1s"].iloc[0] - 85) < 1e-3


def test_derive_sci_acd_sums():
    import build_clean_cache as M
    df = make_df([make_row(Sci_ACD1_094=8, Sci_ACDN_094=2, Sci_ACD1_1s=8, Sci_ACDN_1s=3)])
    out = M.derive_columns(df)
    assert out["Sci_ACD_094"].iloc[0] == 10
    assert out["Sci_ACD_1s"].iloc[0] == 11


def test_derive_includes_all_expected_columns():
    import build_clean_cache as M
    df = make_df([make_row()])
    out = M.derive_columns(df)
    expected_new = {
        "length", "dt_frac",
        "Sci_ACD_094", "Sci_ACD_1s",
        "pho_rate", "ooc_rate", "wide_rate", "large_rate",
        "sci_rate_094", "sci_rate_1s",
        "scipure_rate_094", "scipure_rate_1s",
        "acd1_rate_094", "acd1_rate_1s",
        "acdn_rate_094", "acdn_rate_1s",
        "acd_rate_094", "acd_rate_1s",
    }
    missing = expected_new - set(out.columns)
    assert not missing, f"missing derived columns: {missing}"
```

- [ ] **Step 2: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "derive"
```
Expected: FAIL — `derive_columns` not defined.

- [ ] **Step 3: Implement derive_columns**

Insert into `scripts/build_clean_cache.py` after `apply_filters`:

```python
# ============================================================
# Module 3: derive_columns
# ============================================================

CYCLE_SEC = 16e-6   # 16 µs per L_cycles tick

# (raw_col, derived_col) for the standard "/length" rate transform.
_RATE_PAIRS = [
    ("PHO", "pho_rate"), ("OOC", "ooc_rate"), ("Wide", "wide_rate"), ("Large", "large_rate"),
    ("Sci_094", "sci_rate_094"), ("Sci_1s", "sci_rate_1s"),
    ("Sci_pure_094", "scipure_rate_094"), ("Sci_pure_1s", "scipure_rate_1s"),
    ("Sci_ACD1_094", "acd1_rate_094"), ("Sci_ACD1_1s", "acd1_rate_1s"),
    ("Sci_ACDN_094", "acdn_rate_094"), ("Sci_ACDN_1s", "acdn_rate_1s"),
]


def derive_columns(df):
    """Add length, dt_frac, Sci_ACD_*, and all *_rate columns.

    Returns a new DataFrame (does not mutate the input).
    """
    df = df.copy()
    df["length"] = df["L_cycles"].astype("float32") * CYCLE_SEC
    df["dt_frac"] = (df["Dt"].astype("float32") / df["L_cycles"].astype("float32")).astype("float32")
    df["Sci_ACD_094"] = (df["Sci_ACD1_094"] + df["Sci_ACDN_094"]).astype("int32")
    df["Sci_ACD_1s"]  = (df["Sci_ACD1_1s"] + df["Sci_ACDN_1s"]).astype("int32")
    for raw, derived in _RATE_PAIRS:
        df[derived] = (df[raw].astype("float32") / df["length"]).astype("float32")
    df["acd_rate_094"] = (df["Sci_ACD_094"].astype("float32") / df["length"]).astype("float32")
    df["acd_rate_1s"]  = (df["Sci_ACD_1s"].astype("float32") / df["length"]).astype("float32")
    return df
```

- [ ] **Step 4: Run, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "derive"
```
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): derive_columns (length, rates, dt_frac, Sci_ACD sums)"
```

---

## Task 8: process_one_day

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- process_one_day -------------------

def test_process_one_day_writes_partial(tmp_path):
    import build_clean_cache as M

    input_dir = tmp_path / "per_sec_parquet"
    input_dir.mkdir()
    df = make_df(make_complete_groupsec(date="2020-01-15", met_sec=252633600))
    df.to_parquet(input_dir / "20200115.parquet")

    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    out_dir = tmp_path / "partials"
    out_dir.mkdir()

    result = M.process_one_day("20200115", input_dir, out_dir, cat)

    assert result is not None
    assert result.exists()
    pq = pd.read_parquet(result)
    assert len(pq) == 18  # all 18 rows survive (Lat=0.5, Lon=120 → safe)
    assert "pho_rate" in pq.columns  # derived columns applied


def test_process_one_day_returns_none_when_no_rows_survive(tmp_path):
    import build_clean_cache as M

    input_dir = tmp_path / "per_sec_parquet"
    input_dir.mkdir()
    # All rows have Lat=10° → Stage 3 kills everything
    df = make_df(make_complete_groupsec(date="2020-01-15", Lat=10.0))
    df.to_parquet(input_dir / "20200115.parquet")

    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    out_dir = tmp_path / "partials"
    out_dir.mkdir()

    result = M.process_one_day("20200115", input_dir, out_dir, cat)
    assert result is None
    assert not (out_dir / "20200115.parquet").exists()


def test_process_one_day_missing_input_returns_none(tmp_path):
    import build_clean_cache as M
    input_dir = tmp_path / "per_sec_parquet"
    input_dir.mkdir()
    cat = M.BurstCatalog.from_array(np.array([], dtype=np.int64), window_sec=300)
    out_dir = tmp_path / "partials"
    out_dir.mkdir()

    result = M.process_one_day("99999999", input_dir, out_dir, cat)
    assert result is None
```

- [ ] **Step 2: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "process_one_day"
```
Expected: FAIL — `process_one_day` not defined.

- [ ] **Step 3: Implement process_one_day**

Insert into `scripts/build_clean_cache.py` after `derive_columns`:

```python
# ============================================================
# Module 4: process_one_day
# ============================================================

def process_one_day(date_str, input_dir, partial_dir, burst_catalog):
    """Load one day's per_sec_parquet, run filters + derives, write a partial.

    Args:
        date_str: 'YYYYMMDD'
        input_dir: Path to directory containing {date_str}.parquet
        partial_dir: Path where {date_str}.parquet partial is written
        burst_catalog: BurstCatalog instance

    Returns: Path to written partial, or None if no rows survived / input missing.
    """
    from pathlib import Path
    import pandas as pd

    input_dir = Path(input_dir)
    partial_dir = Path(partial_dir)
    in_path = input_dir / f"{date_str}.parquet"

    if not in_path.exists():
        print(f"[process_one_day] {date_str}: input parquet missing, skipping")
        return None

    df = pd.read_parquet(in_path)
    df, counts = apply_filters(df, burst_catalog)
    print(f"[process_one_day] {date_str}: {counts}")

    if len(df) == 0:
        return None

    df = derive_columns(df)
    out_path = partial_dir / f"{date_str}.parquet"
    df.to_parquet(out_path, compression="zstd")
    return out_path
```

- [ ] **Step 4: Run, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "process_one_day"
```
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): process_one_day (load → filter → derive → write partial)"
```

---

## Task 9: run_build (multiprocessing + concat + assertions) + main CLI

**Files:**
- Modify: `scripts/build_clean_cache.py`
- Modify: `tests/test_build_clean_cache.py`

- [ ] **Step 1: Add failing integration test**

Append to `tests/test_build_clean_cache.py`:

```python
# ------------------- run_build integration -------------------

def test_run_build_writes_final_parquet_and_passes_assertions(tmp_path):
    import build_clean_cache as M

    input_dir = tmp_path / "per_sec_parquet"
    input_dir.mkdir()
    for date_str, date_iso in [("20200115", "2020-01-15"),
                                 ("20200116", "2020-01-16"),
                                 ("20200117", "2020-01-17")]:
        # Two seconds per day, all clean
        rows = (make_complete_groupsec(date=date_iso, met_sec=252633600)
                + make_complete_groupsec(date=date_iso, met_sec=252633700))
        make_df(rows).to_parquet(input_dir / f"{date_str}.parquet")

    # Empty GBM cache so we don't try to fetch
    gbm_cache = tmp_path / "gbm.parquet"
    pd.DataFrame({"trigger_met_hxmt": []}).to_parquet(gbm_cache)

    output = tmp_path / "clean.parquet"
    partial_dir = tmp_path / "partial"
    partial_dir.mkdir()

    M.run_build(
        input_dir=input_dir,
        output=output,
        partial_dir=partial_dir,
        gbm_cache=gbm_cache,
        dates=["20200115", "20200116", "20200117"],
        workers=2,
        min_rows=1,        # lower the assertion floor for tests
    )

    assert output.exists()
    df = pd.read_parquet(output)
    # 3 days × 2 seconds × 18 rows = 108
    assert len(df) == 108
    assert "pho_rate" in df.columns


def test_run_build_raises_when_under_min_rows(tmp_path):
    import build_clean_cache as M
    input_dir = tmp_path / "per_sec_parquet"
    input_dir.mkdir()
    # One day, all dropped (Lat too high)
    make_df(make_complete_groupsec(date="2020-01-15", Lat=10.0)).to_parquet(
        input_dir / "20200115.parquet"
    )
    gbm_cache = tmp_path / "gbm.parquet"
    pd.DataFrame({"trigger_met_hxmt": []}).to_parquet(gbm_cache)
    (tmp_path / "partial").mkdir()

    with pytest.raises(AssertionError, match="No partials"):
        M.run_build(
            input_dir=input_dir,
            output=tmp_path / "clean.parquet",
            partial_dir=tmp_path / "partial",
            gbm_cache=gbm_cache,
            dates=["20200115"],
            workers=1,
            min_rows=1,
        )
```

- [ ] **Step 2: Run, expect FAIL**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "run_build"
```
Expected: FAIL — `run_build` not defined.

- [ ] **Step 3: Implement run_build + main + CLI**

Insert into `scripts/build_clean_cache.py` after `process_one_day` (and delete the placeholder `if __name__` stub at the bottom):

```python
# ============================================================
# Module 5: run_build (main pipeline) + CLI
# ============================================================

from functools import partial as _functools_partial


def _process_one_day_for_pool(date_str, input_dir, partial_dir, gbm_cache, window_sec):
    """Top-level wrapper for multiprocessing.Pool.

    Re-loads BurstCatalog from cache in each worker (fast, ~ms; avoids serialising
    a 3000+ entry array per worker invocation)."""
    cat = BurstCatalog.fetch_or_load(gbm_cache, window_sec=window_sec, allow_fetch=False)
    try:
        return process_one_day(date_str, input_dir, partial_dir, cat)
    except Exception as exc:
        print(f"[worker] {date_str}: FAILED with {type(exc).__name__}: {exc}")
        raise


def run_build(input_dir, output, partial_dir, gbm_cache, dates, workers=8, min_rows=1_000_000):
    """Top-level pipeline: per-day fan-out via multiprocessing.Pool, then concat + assertions."""
    from pathlib import Path
    from multiprocessing import Pool
    import pyarrow as pa
    import pyarrow.parquet as pq

    input_dir = Path(input_dir)
    output = Path(output)
    partial_dir = Path(partial_dir)
    gbm_cache = Path(gbm_cache)
    partial_dir.mkdir(parents=True, exist_ok=True)
    output.parent.mkdir(parents=True, exist_ok=True)

    worker = _functools_partial(
        _process_one_day_for_pool,
        input_dir=input_dir,
        partial_dir=partial_dir,
        gbm_cache=gbm_cache,
        window_sec=300,
    )

    print(f"Starting pool with {workers} workers over {len(dates)} dates")
    if workers == 1:
        partials = [worker(d) for d in dates]
    else:
        with Pool(processes=workers) as pool:
            partials = pool.map(worker, dates)

    written = [p for p in partials if p is not None]
    print(f"Day-level done: {len(written)}/{len(dates)} days wrote partials")

    if not written:
        raise AssertionError("No partials written — entire run produced zero rows")

    print("Concatenating partials...")
    tables = [pq.read_table(p) for p in written]
    combined = pa.concat_tables(tables)
    tmp_out = output.with_suffix(output.suffix + ".tmp")
    pq.write_table(combined, tmp_out, compression="zstd")

    n_rows = combined.num_rows
    print(f"Concatenated {n_rows:,} rows")

    df = combined.to_pandas()
    assert n_rows >= min_rows, f"Final row count {n_rows:,} below minimum {min_rows:,}"
    assert (df["Lat"].abs() < LAT_MAX_ABS).all(), "Lat assertion failed"
    assert (~((df["Lon"] >= SAA_LON_LO) & (df["Lon"] <= SAA_LON_HI))).all(), "Lon assertion failed"
    for w in ("094", "1s"):
        lhs = df[f"Sci_pure_{w}"] + df[f"Sci_ACD1_{w}"] + df[f"Sci_ACDN_{w}"]
        assert (lhs == df[f"Sci_{w}"]).all(), f"Sci invariant failed for window {w}"

    tmp_out.rename(output)
    print(f"Final cache written: {output} ({output.stat().st_size / 1e6:.1f} MB)")
    for p in written:
        p.unlink()
    print(f"Partial files cleaned up")
    return output


def _parse_args():
    import argparse
    p = argparse.ArgumentParser()
    p.add_argument("--input-dir", required=True, help="Dir of per_sec_parquet/{YYYYMMDD}.parquet")
    p.add_argument("--output", required=True, help="Output parquet path")
    p.add_argument("--partial-dir", required=True, help="Scratch dir for per-day partials")
    p.add_argument("--gbm-cache", required=True, help="Path to GBM trigger parquet")
    p.add_argument("--workers", type=int, default=8)
    p.add_argument("--start", default="2020-01-01")
    p.add_argument("--end", default="2020-06-30")
    p.add_argument("--allow-fetch", action="store_true",
                    help="If GBM cache missing, fetch from HEASARC (needs HTTPS)")
    return p.parse_args()


def main():
    args = _parse_args()
    from pathlib import Path
    gbm_cache = Path(args.gbm_cache)

    # Pre-fetch the GBM cache up front (before forking workers) if requested
    if args.allow_fetch and not gbm_cache.exists():
        print("Fetching GBM triggers from HEASARC...")
        BurstCatalog.fetch_or_load(gbm_cache, window_sec=300, allow_fetch=True)
        print(f"  cached: {gbm_cache}")
    elif not gbm_cache.exists():
        raise FileNotFoundError(
            f"GBM cache missing ({gbm_cache}); pass --allow-fetch to download from HEASARC"
        )

    from datetime import date, timedelta
    d_start = date.fromisoformat(args.start)
    d_end = date.fromisoformat(args.end)
    dates = []
    d = d_start
    while d <= d_end:
        dates.append(d.strftime("%Y%m%d"))
        d += timedelta(days=1)

    run_build(
        input_dir=args.input_dir,
        output=args.output,
        partial_dir=args.partial_dir,
        gbm_cache=gbm_cache,
        dates=dates,
        workers=args.workers,
    )


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run, expect PASS**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v -k "run_build"
```
Expected: 2 PASS.

- [ ] **Step 5: Run full test suite**

Run:
```bash
.venv/bin/pytest tests/test_build_clean_cache.py -v
```
Expected: ALL tests PASS (around 30 tests).

- [ ] **Step 6: Commit**

```bash
git add scripts/build_clean_cache.py tests/test_build_clean_cache.py
git commit -m "feat(clean-cache): run_build + CLI (pool + concat + assertions)"
```

---

## Task 10: Deploy script to server + smoke test on 20200115

**Files:**
- No local file changes; this task deploys to the server.

- [ ] **Step 1: Sync script to server**

Run:
```bash
scp scripts/build_clean_cache.py guohx@hlogin01.ihep.ac.cn:/scratchfs/gecam/guohx/blink/scripts/build_clean_cache.py
```
Expected: scp completes, no errors.

- [ ] **Step 2: Install astroquery on server (lxlogin, if not already)**

Run:
```bash
ssh guohx@lxlogin.ihep.ac.cn 'python3 -m pip install --user --index-url https://pypi.tuna.tsinghua.edu.cn/simple astroquery'
```
Expected: install completes (might already be installed).

- [ ] **Step 3: Fetch GBM catalog on lxlogin (has internet)**

Run:
```bash
ssh guohx@lxlogin.ihep.ac.cn '
cd /scratchfs/gecam/guohx/blink
python3 -c "
import sys; sys.path.insert(0, \"scripts\")
import build_clean_cache as M
from pathlib import Path
cache = Path(\"n_below_study/gbm_triggers.parquet\")
M.BurstCatalog.fetch_or_load(cache, window_sec=300, allow_fetch=True)
print(\"fetched OK\")
import pandas as pd
df = pd.read_parquet(cache)
print(f\"  rows: {len(df)}\")
print(f\"  min/max MET: {df[\"trigger_met_hxmt\"].min()} / {df[\"trigger_met_hxmt\"].max()}\")
"'
```
Expected: prints "fetched OK", row count > 1000, MET range covering 2017–2026.

If fetch fails (astroquery API drift), open `scripts/build_clean_cache.py` and adjust `_fetch_gbm_triggers_from_heasarc()` accordingly, then retry.

- [ ] **Step 4: Run smoke test on hlogin01 (single day, 20200115)**

Run:
```bash
ssh guohx@hlogin01.ihep.ac.cn '
cd /scratchfs/gecam/guohx/blink
mkdir -p n_below_study/.partial_smoke
python3 -c "
import sys; sys.path.insert(0, \"scripts\")
import build_clean_cache as M
from pathlib import Path
cat = M.BurstCatalog.fetch_or_load(
    Path(\"n_below_study/gbm_triggers.parquet\"),
    window_sec=300, allow_fetch=False)
print(f\"GBM catalog: {cat.triggers_met.size} triggers\")
res = M.process_one_day(
    \"20200115\",
    Path(\"per_sec_parquet\"),
    Path(\"n_below_study/.partial_smoke\"),
    cat,
)
print(f\"smoke output: {res}\")
import pandas as pd
df = pd.read_parquet(res)
print(f\"rows: {len(df):,}\")
print(f\"cols: {list(df.columns)}\")
print(f\"Lat range: [{df[\"Lat\"].min():.2f}, {df[\"Lat\"].max():.2f}]\")
print(f\"Lon range: [{df[\"Lon\"].min():.2f}, {df[\"Lon\"].max():.2f}]\")
print(f\"PHO/sec sample: {df[\"pho_rate\"].describe()}\")
"
'
```
Expected: prints filter stage counts; output rows are 18×N where N = clean seconds in 20200115 (rough estimate 30-50 minutes worth = 540-900 rows); all Lat in [-3, 3]; all Lon outside [-90, 30]; PHO rates physical (10s–1000s/sec).

- [ ] **Step 5: Clean up smoke partials**

Run:
```bash
ssh guohx@hlogin01.ihep.ac.cn 'rm -rf /scratchfs/gecam/guohx/blink/n_below_study/.partial_smoke'
```
Expected: directory removed.

- [ ] **Step 6: No commit needed — server-side smoke runs only.**

---

## Task 11: Full 182-day run on hlogin01

**Files:**
- No file changes; this is the production run.

- [ ] **Step 1: Kick off the build**

Run:
```bash
ssh guohx@hlogin01.ihep.ac.cn '
cd /scratchfs/gecam/guohx/blink
mkdir -p n_below_study/.partial_2020H1
python3 scripts/build_clean_cache.py \
  --input-dir per_sec_parquet \
  --output n_below_study/clean_2020H1.parquet \
  --partial-dir n_below_study/.partial_2020H1 \
  --gbm-cache n_below_study/gbm_triggers.parquet \
  --start 2020-01-01 \
  --end 2020-06-30 \
  --workers 8 \
  2>&1 | tee n_below_study/build_2020H1.log
'
```
Expected: prints per-day filter counts, then concat message, then "Final cache written" + size in MB. Wall time 5–15 min.

- [ ] **Step 2: Verify output**

Run:
```bash
ssh guohx@hlogin01.ihep.ac.cn '
cd /scratchfs/gecam/guohx/blink
python3 -c "
import pandas as pd
from pathlib import Path
p = Path(\"n_below_study/clean_2020H1.parquet\")
df = pd.read_parquet(p)
print(f\"rows: {len(df):,}\")
print(f\"file size: {p.stat().st_size/1e6:.1f} MB\")
print(f\"unique dates: {df.date.nunique()}\")
print(f\"date range: {df.date.min()} -> {df.date.max()}\")
print(f\"unique (box, det): {df.groupby([\"box\",\"det\"]).ngroups} (expect 18)\")
print(f\"Lat: [{df.Lat.min():.2f}, {df.Lat.max():.2f}] (expect within [-3, 3])\")
print(f\"PHO rate quantiles (0.01/0.5/0.99): {df.pho_rate.quantile([0.01, 0.5, 0.99]).to_dict()}\")
"
'
```
Expected: rows in the millions, 120-180 unique dates (some days may be 0-row), 18 unique `(box, det)`, Lat strictly inside [-3, 3], PHO rates physical.

- [ ] **Step 3: Verify partial cleanup**

Run:
```bash
ssh guohx@hlogin01.ihep.ac.cn 'ls /scratchfs/gecam/guohx/blink/n_below_study/.partial_2020H1/ 2>&1 | head'
```
Expected: empty (script removes partials after concat).

- [ ] **Step 4: No commit — script behavior already committed; this is a runtime artifact.**

---

## Self-Review

**Spec coverage:**
- [x] Time window 2020-01-01..2020-06-30 → main() defaults, Task 11
- [x] Stage 1 detector state → Task 4
- [x] Stage 2 integrity → Task 4
- [x] Stage 3 spatial → Task 4
- [x] Stage 4 burst → Task 5 + Task 3 (BurstCatalog)
- [x] Stage 5 completeness → Task 6
- [x] Derived columns → Task 7
- [x] Output single parquet → Task 9 (concat in run_build)
- [x] Atomic rename → Task 9 (`tmp_out.rename(output)`)
- [x] Multiprocessing 8-way → Task 9
- [x] 4 post-build assertions → Task 9
- [x] Smoke test workflow → Task 10
- [x] Fetch GBM from HEASARC, cache as parquet → Task 3 + Task 10
- [x] Failure handling (missing daily parquet, 0-row day, fetch failure) → Task 8 + Task 9

**Placeholder check:** no TBDs / TODOs / "implement later" / hand-wavy lines. Every code block contains the actual code.

**Type / name consistency:** spot-checked — `_apply_stage{1..5}_*` naming, `BurstCatalog.from_array` / `fetch_or_load` / `any_within`, `apply_filters` / `derive_columns` / `process_one_day` / `run_build` / `main` — all consistent across tasks.

**Scope check:** single subsystem (one script + tests), single implementation plan. Appropriate.
