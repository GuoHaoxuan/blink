#!/usr/bin/env python3
"""Build clean PHO-verification cache from per_sec_parquet.

See docs/superpowers/specs/2026-05-20-clean-pho-cache-design.md
"""
from __future__ import annotations

from dataclasses import dataclass
import numpy as np


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


def _apply_stage4_burst(df, burst_catalog):
    """Drop rows whose met_sec is within ±window_sec of any GBM trigger."""
    times = df["met_sec"].to_numpy().astype(np.int64)
    drop_mask = burst_catalog.any_within(times)
    return df.loc[~drop_mask].copy()


def _apply_stage5_completeness(df):
    """Keep only (date, met_sec) groups where all 18 (3 boxes × 6 dets) rows are present."""
    counts = df.groupby(["date", "met_sec"]).size()
    full_keys = counts[counts == 18].index
    df_idx = df.set_index(["date", "met_sec"])
    keep = df_idx.index.isin(full_keys)
    return df_idx.loc[keep].reset_index()


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


# ============================================================
# CLI entry point (filled out in later tasks)
# ============================================================

if __name__ == "__main__":
    raise SystemExit("not yet implemented")
