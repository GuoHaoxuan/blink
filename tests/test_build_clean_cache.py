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
