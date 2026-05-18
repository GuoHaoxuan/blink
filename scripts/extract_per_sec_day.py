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
from astropy.io import fits

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


def read_he_eng(path) -> dict:
    """Read one 1B HE_Eng FITS file. Returns dict of numpy arrays.

    Schema (per-second, ~3600 rows per file):
        Time, Length_Time_Cycle:                shape (n,)         int
        UTC_Last_Bdc, sTime_Last_Bdc:           shape (n,)         int
        Cnt_PHODet, Cnt_OOCDet,
        Cnt_CsI_PHODet, Cnt_LargeEvt,
        DeadTime_PHODet:                        shape (n, 6)       int  (per-det)
        BUS_Time_Bdc:                           shape (n, 6)       uint8 (raw)
        Error_code:                             shape (n, 4)       uint8 (raw)
    """
    with fits.open(path, memmap=False) as f:
        d = f["HE_Eng"].data
        n = len(d)

        def per_det(base: str) -> np.ndarray:
            cols = [d[f"{base}_{i}"].astype(np.int32) for i in range(6)]
            return np.stack(cols, axis=1)   # (n, 6)

        out = {
            "Time":              d["Time"].astype(np.int64),
            "Length_Time_Cycle": d["Length_Time_Cycle"].astype(np.int32),
            "UTC_Last_Bdc":      d["UTC_Last_Bdc"].astype(np.int64),
            "sTime_Last_Bdc":    d["sTime_Last_Bdc"].astype(np.int64),
            "Cnt_PHODet":        per_det("Cnt_PHODet"),
            "Cnt_OOCDet":        per_det("Cnt_OOCDet"),
            "Cnt_CsI_PHODet":    per_det("Cnt_CsI_PHODet"),
            "Cnt_LargeEvt":      per_det("Cnt_LargeEvt"),
            "DeadTime_PHODet":   per_det("DeadTime_PHODet"),
            "BUS_Time_Bdc":      np.asarray(d["BUS_Time_Bdc"], dtype=np.uint8).reshape(n, 6),
            "Error_code":        np.asarray(d["Error_code"], dtype=np.uint8).reshape(n, 4),
        }
        return out


if __name__ == "__main__":
    raise NotImplementedError("CLI is implemented in Task 12")
