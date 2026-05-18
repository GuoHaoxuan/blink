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


def read_he_hv(path) -> dict:
    """Read one 1K HE-HV FITS file. Returns dict with 'Time' and 'HV'.

    HV shape: (n_sec, 18) — 18 global detectors.
    """
    with fits.open(path, memmap=False) as f:
        d = f["HE_HV_PHODet"].data
        n = len(d)
        cols = [d[f"HV_PHODet_{j}"].astype(np.float32) for j in range(18)]
        return {
            "Time": d["Time"].astype(np.int64),
            "HV":   np.stack(cols, axis=1),
        }


def read_orbit(path) -> dict:
    """Read one 1K Orbit FITS file. Returns dict of numpy arrays.

    Schema (~3601 rows @ 1 Hz):
        Time:         int64 seconds (1K MET)
        X, Y, Z:      float64 m
        Vx, Vy, Vz:   float64 m/s
        Lon, Lat:     float64 degrees
        Alt:          float64 m
    """
    with fits.open(path, memmap=False) as f:
        d = f["Orbit"].data
        # Time stored as D (float64); cast to int64 (Orbit is at integer-second
        # cadence, verified empirically).
        time_int = np.round(d["Time"].astype(np.float64)).astype(np.int64)
        return {
            "Time": time_int,
            "X":    d["X"].astype(np.float64),
            "Y":    d["Y"].astype(np.float64),
            "Z":    d["Z"].astype(np.float64),
            "Vx":   d["Vx"].astype(np.float64),
            "Vy":   d["Vy"].astype(np.float64),
            "Vz":   d["Vz"].astype(np.float64),
            "Lon":  d["Lon"].astype(np.float64),
            "Lat":  d["Lat"].astype(np.float64),
            "Alt":  d["Alt"].astype(np.float64),
        }


def _nearest_sample(att_time: np.ndarray, target_secs: np.ndarray) -> np.ndarray:
    """For each integer second in ``target_secs``, return the index of the Att
    sample whose Time is closest (in absolute distance).

    Returns shape (len(target_secs),) int64. If a target_sec is outside the Att
    coverage, the boundary index is returned (caller must check NaN-fill later).
    """
    # searchsorted gives position just to the right
    pos = np.searchsorted(att_time, target_secs.astype(np.float64))
    # clamp
    pos_clip = np.clip(pos, 1, len(att_time) - 1)
    # compare distance to pos-1 vs pos
    left  = pos_clip - 1
    right = pos_clip
    pick_right = (
        np.abs(att_time[right] - target_secs) <
        np.abs(att_time[left]  - target_secs)
    )
    return np.where(pick_right, right, left)


_ATT_HDU_COLS = {
    "ATT_Pointing": ["Ra", "Dec", "Delta_Ra", "Delta_Dec", "Delta"],
    "ATT_Euler":    ["Euler_Phi", "Euler_Theta", "Euler_Psi"],
    "ATT_Quater":   ["Q1", "Q2", "Q3"],
    "ATT_Omega":    ["Omega_X", "Omega_Y", "Omega_Z"],
}


def read_att(path, target_secs: np.ndarray) -> dict:
    """Read one 1K Att FITS, downsample 4 Hz → 1 Hz by nearest-frame.

    ``target_secs``: int64 array of integer met_sec values to sample at.
    Returns: dict with 14 keys, each a (len(target_secs),) float32 array.
    """
    out = {}
    with fits.open(path, memmap=False) as f:
        for hdu_name, col_names in _ATT_HDU_COLS.items():
            d = f[hdu_name].data
            t = d["Time"].astype(np.float64)
            idx = _nearest_sample(t, target_secs)
            for c in col_names:
                out[c] = d[c][idx].astype(np.float32)
    return out


if __name__ == "__main__":
    raise NotImplementedError("CLI is implemented in Task 12")
