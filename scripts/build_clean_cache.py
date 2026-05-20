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
