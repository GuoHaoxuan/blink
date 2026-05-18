"""HXMT/HE per-second raw data product extractor (one date per invocation).

Reads 1B HE_Eng + 1K HE-HV / Orbit / Att / HE-Evt FITS files for one UTC date
and emits ``per_sec_parquet/{YYYYMMDD}.parquet`` (~48 columns, ~1.56M rows).
ETL only — no filtering. Idempotent: if output exists, exits 0.

Server paths default to:
    1B   /hxmtfs/data/Archive_tmp/1B
    1K   /hxmt/work/HXMT-DATA/1K
Override via environment variables BLINK_1B_ROOT and BLINK_1K_ROOT for local
testing.

CLI:
    python3 extract_per_sec_day.py YYYYMMDD [--output-dir DIR]
"""
from __future__ import annotations

import os
from pathlib import Path

import numpy as np

MET_CORRECTION = 4.0  # 1B Time → 1K MET, verified sub-microsecond

BOX_PORTS = {"A": "0766", "B": "1009", "C": "1781"}
BOX_INDEX = {"A": 0, "B": 1, "C": 2}

DEFAULT_1B_ROOT = "/hxmtfs/data/Archive_tmp/1B"
DEFAULT_1K_ROOT = "/hxmt/work/HXMT-DATA/1K"


def root_1b() -> Path:
    return Path(os.environ.get("BLINK_1B_ROOT", DEFAULT_1B_ROOT))


def root_1k() -> Path:
    return Path(os.environ.get("BLINK_1K_ROOT", DEFAULT_1K_ROOT))


def compute_offset(utc_last_bdc: int, stime_last_bdc: int) -> int:
    """1B Time-to-UTC offset for one HE_Eng file (constant across rows).

    Use [0]-th element from the file's HE_Eng table — same for every row in
    a given file.
    """
    return int(utc_last_bdc) - int(stime_last_bdc)


def compute_met_float(time_1b, offset: int) -> float:
    """Convert 1B HE_Eng ``Time`` to 1K-aligned MET (float seconds).

    Works on scalars or numpy arrays. ``MET_CORRECTION = 4.0s`` is the
    empirical 1B→1K offset (verified sub-microsecond elsewhere in the project).
    """
    return time_1b + offset + MET_CORRECTION


def count_acd_bits(acd: np.ndarray) -> np.ndarray:
    """Per-event popcount over the 18-bit ACD shield mask.

    Input: shape (n_events, 18) boolean array.
    Output: shape (n_events,) int8 array of popcounts (0..18).
    """
    return acd.sum(axis=1).astype(np.int8)


def window_indices(times: np.ndarray, t0: float, t1: float) -> tuple[int, int]:
    """Half-open interval [t0, t1) → (i_start, i_end) into a sorted ``times`` array.

    ``times[i_start:i_end]`` are the events in the window.
    """
    i_start = int(np.searchsorted(times, t0, side="left"))
    i_end = int(np.searchsorted(times, t1, side="left"))
    return i_start, i_end


if __name__ == "__main__":
    raise NotImplementedError("CLI is implemented in Task 12")
