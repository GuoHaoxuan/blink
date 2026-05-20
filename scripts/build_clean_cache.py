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
# CLI entry point (filled out in later tasks)
# ============================================================

if __name__ == "__main__":
    raise SystemExit("not yet implemented")
