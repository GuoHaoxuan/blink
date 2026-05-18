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

import argparse
import glob
import os
import sys
import tempfile
from pathlib import Path

import numpy as np
import pandas as pd
from astropy.io import fits

MET_CORRECTION = 4.0  # 1B Time → 1K MET, verified sub-microsecond

BOX_PORTS = {"A": "0766", "B": "1009", "C": "1781"}
BOX_INDEX = {"A": 0, "B": 1, "C": 2}

DEFAULT_1B_ROOT = "/hxmtfs/data/Archive_tmp/1B"
DEFAULT_1K_ROOT = "/hxmt/work/HXMT-DATA/1K"

# NOTE: crc_box is left NaN. CRC failures are a 1B-level artefact (events that
# fail CRC are dropped before 1K). Recovering per-second CRC counts would
# require running blink_cli sat-style 1B parsing — deferred to a later spec.
# The column is preserved in the schema for forward compatibility.


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
        col_names = set(f["HE_Eng"].columns.names)

        # Detect column naming format: 2017 uses per-box indices, 2026 uses global indices
        # Try 2017 format first (per-box: 0-5), fall back to 2026 format if not found
        has_per_box = all(f"Cnt_PHODet_{i}" in col_names for i in range(6))

        def per_det(base: str, box_port: str | None = None) -> np.ndarray:
            if has_per_box:
                # 2017 format: indices 0-5 (per-box)
                cols = [d[f"{base}_{i}"].astype(np.int32) for i in range(6)]
            else:
                # 2026 format: indices are global (0-5 for Box A, 6-11 for B, 12-17 for C)
                # Infer box from port if available, or try to find available indices
                if box_port in ["0766", None]:
                    start_idx = 0
                elif box_port == "1009":
                    start_idx = 6
                elif box_port == "1781":
                    start_idx = 12
                else:
                    # Fallback: find first column and infer
                    for i in range(18):
                        if f"{base}_{i}" in col_names:
                            start_idx = (i // 6) * 6
                            break
                cols = [d[f"{base}_{start_idx + i}"].astype(np.int32) for i in range(6)]
            return np.stack(cols, axis=1)   # (n, 6)

        # Infer box port from path
        path_str = str(path)
        box_port = None
        for port in ["0766", "1009", "1781"]:
            if port in path_str:
                box_port = port
                break

        out = {
            "Time":              d["Time"].astype(np.int64),
            "Length_Time_Cycle": d["Length_Time_Cycle"].astype(np.int32),
            "UTC_Last_Bdc":      d["UTC_Last_Bdc"].astype(np.int64),
            "sTime_Last_Bdc":    d["sTime_Last_Bdc"].astype(np.int64),
            "Cnt_PHODet":        per_det("Cnt_PHODet", box_port),
            "Cnt_OOCDet":        per_det("Cnt_OOCDet", box_port),
            "Cnt_CsI_PHODet":    per_det("Cnt_CsI_PHODet", box_port),
            "Cnt_LargeEvt":      per_det("Cnt_LargeEvt", box_port),
            "DeadTime_PHODet":   per_det("DeadTime_PHODet", box_port),
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


def read_he_evt(path) -> dict:
    """Read one 1K HE-Evt FITS file. Returns dict with sorted event arrays.

    Output:
        Time:          float64 (n_events,)  — sorted MET seconds
        Det_ID:        int8    (n_events,)  — global 0..17
        ACD_popcount:  int8    (n_events,)  — 0..18, computed from 18-bit ACD field
    """
    with fits.open(path, memmap=False) as f:
        d = f["Events"].data
        acd = np.asarray(d["ACD"], dtype=bool)   # (n, 18)
        return {
            "Time":         d["Time"].astype(np.float64),
            "Det_ID":       d["Det_ID"].astype(np.int8),
            "ACD_popcount": count_acd_bits(acd),
        }


def aggregate_he_evt(
    evt: dict,
    met_floats: np.ndarray,
    box_index: int,
    det: int,
    window_s_094: float = 0.94,
    window_s_1s: float = 1.0,
) -> dict:
    """Aggregate one box/det's events into per-second counts for two windows.

    For each second in ``met_floats``, emit:
        Sci_094, Sci_pure_094, Sci_ACD1_094, Sci_ACDN_094,
        Sci_1s,  Sci_pure_1s,  Sci_ACD1_1s,  Sci_ACDN_1s

    Pre-filter events to the target det_global = box_index*6 + det.
    """
    det_global = box_index * 6 + det
    mask = evt["Det_ID"] == det_global
    # Event arrays restricted to this det, still sorted by Time.
    t  = evt["Time"][mask]
    pc = evt["ACD_popcount"][mask]

    n = len(met_floats)
    out = {k: np.zeros(n, dtype=np.int32) for k in [
        "Sci_094", "Sci_pure_094", "Sci_ACD1_094", "Sci_ACDN_094",
        "Sci_1s",  "Sci_pure_1s",  "Sci_ACD1_1s",  "Sci_ACDN_1s",
    ]}

    for i, t0 in enumerate(met_floats):
        for tag, dt in (("094", window_s_094), ("1s", window_s_1s)):
            i_start, i_end = window_indices(t, t0, t0 + dt)
            pc_win = pc[i_start:i_end]
            total = i_end - i_start
            n_pure = int((pc_win == 0).sum())
            n_acd1 = int((pc_win == 1).sum())
            n_acdn = total - n_pure - n_acd1
            out[f"Sci_{tag}"][i]      = total
            out[f"Sci_pure_{tag}"][i] = n_pure
            out[f"Sci_ACD1_{tag}"][i] = n_acd1
            out[f"Sci_ACDN_{tag}"][i] = n_acdn

    return out


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


def find_he_eng_path(date: str, hour: int, port: str) -> Path | None:
    """Locate the 1B HE_Eng file for one (date, hour, port).

    Expected layout::
        {BLINK_1B_ROOT}/{YYYY}/{YYYYMMDD}/{port}/HXMT_1B_{port}_{YYYYMMDD}T{HH}0000_*.fits

    Returns None if no matching file found.
    """
    year = date[:4]
    pattern = str(
        root_1b() / year / date / port
        / f"HXMT_1B_{port}_{date}T{hour:02d}0000_*.fits"
    )
    matches = sorted(glob.glob(pattern))
    return Path(matches[0]) if matches else None


def find_1k_aux_path(date: str, hour: int, product: str) -> Path | None:
    """Locate a 1K auxiliary file (HE-HV / Orbit / Att / HE-Evt) for (date, hour).

    Expected layout::
        {BLINK_1K_ROOT}/Y{YYYYMM}/{YYYYMMDD}-{seq}/HXMT_{YYYYMMDD}T{HH}_{product}_FFFFFF_V*_1K.FITS

    The ``{seq}`` is the mission-day counter — we don't compute it; just glob
    across all sequence directories for the given date.
    """
    ym = date[:6]
    pattern = str(
        root_1k() / f"Y{ym}" / f"{date}-*"
        / f"HXMT_{date}T{hour:02d}_{product}_FFFFFF_V*_1K.FITS"
    )
    matches = sorted(glob.glob(pattern))
    return Path(matches[0]) if matches else None


def _box_hour_rows(
    date: str, box: str, hour: int,
    hv_lookup: dict | None,
    orbit_lookup: dict | None,
    att_lookup: dict | None,
    evt: dict | None,
) -> list[dict]:
    """Build all (sec × 6 det) rows for one (box, hour). Returns empty if HE_Eng missing."""
    eng_path = find_he_eng_path(date, hour, BOX_PORTS[box])
    if eng_path is None:
        return []
    d = read_he_eng(eng_path)
    n_sec = len(d["Time"])
    box_idx = BOX_INDEX[box]
    offset = compute_offset(int(d["UTC_Last_Bdc"][0]), int(d["sTime_Last_Bdc"][0]))
    met_float = compute_met_float(d["Time"], offset)
    met_sec   = np.floor(met_float).astype(np.int64)

    # Orbit/HV lookup tables: built once per hour
    orbit_vals = {}
    if orbit_lookup is not None:
        for c in ["X", "Y", "Z", "Vx", "Vy", "Vz", "Lon", "Lat", "Alt"]:
            arr = np.full(n_sec, np.nan, dtype=np.float64)
            idx_of = orbit_lookup["__index_of"]
            for i, s in enumerate(met_sec):
                if int(s) in idx_of:
                    arr[i] = orbit_lookup[c][idx_of[int(s)]]
            orbit_vals[c] = arr
    else:
        for c in ["X", "Y", "Z", "Vx", "Vy", "Vz", "Lon", "Lat", "Alt"]:
            orbit_vals[c] = np.full(n_sec, np.nan)

    # HV: needs per-det extraction
    if hv_lookup is not None:
        hv_full = np.full((n_sec, 18), np.nan, dtype=np.float32)
        idx_of = hv_lookup["__index_of"]
        for i, s in enumerate(met_sec):
            if int(s) in idx_of:
                hv_full[i] = hv_lookup["HV"][idx_of[int(s)]]
    else:
        hv_full = np.full((n_sec, 18), np.nan, dtype=np.float32)

    # Att: read with the actual met_sec array → all 14 cols
    if att_lookup is not None:
        att_vals = att_lookup   # already indexed by met_sec
    else:
        att_vals = {k: np.full(n_sec, np.nan, dtype=np.float32) for k in [
            "Ra", "Dec", "Delta_Ra", "Delta_Dec", "Delta",
            "Euler_Phi", "Euler_Theta", "Euler_Psi",
            "Q1", "Q2", "Q3",
            "Omega_X", "Omega_Y", "Omega_Z"]}

    # Build rows per det
    rows = []
    for det in range(6):
        det_global = box_idx * 6 + det
        if evt is not None:
            sci = aggregate_he_evt(evt, met_float, box_idx, det)
        else:
            sci = {k: np.full(n_sec, np.nan) for k in [
                "Sci_094", "Sci_pure_094", "Sci_ACD1_094", "Sci_ACDN_094",
                "Sci_1s",  "Sci_pure_1s",  "Sci_ACD1_1s",  "Sci_ACDN_1s"]}

        for i in range(n_sec):
            row = {
                "date":          date[:4] + "-" + date[4:6] + "-" + date[6:],
                "box":           box,
                "det":           det,
                "met_sec":       int(met_sec[i]),
                "time_float":    float(met_float[i]),
                "L_cycles":      int(d["Length_Time_Cycle"][i]),
                "PHO":           int(d["Cnt_PHODet"][i, det]),
                "OOC":           int(d["Cnt_OOCDet"][i, det]),
                "Wide":          int(d["Cnt_CsI_PHODet"][i, det]),
                "Large":         int(d["Cnt_LargeEvt"][i, det]),
                "Dt":            int(d["DeadTime_PHODet"][i, det]),
                "HV":            float(hv_full[i, det_global]),
                "Sci_094":       sci["Sci_094"][i],
                "Sci_pure_094":  sci["Sci_pure_094"][i],
                "Sci_ACD1_094":  sci["Sci_ACD1_094"][i],
                "Sci_ACDN_094":  sci["Sci_ACDN_094"][i],
                "Sci_1s":        sci["Sci_1s"][i],
                "Sci_pure_1s":   sci["Sci_pure_1s"][i],
                "Sci_ACD1_1s":   sci["Sci_ACD1_1s"][i],
                "Sci_ACDN_1s":   sci["Sci_ACDN_1s"][i],
                "crc_box":       np.nan,
                "utc_last_bdc":  int(d["UTC_Last_Bdc"][i]),
                "stime_last_bdc": int(d["sTime_Last_Bdc"][i]),
                "error_code":    bytes(d["Error_code"][i]),
                "bus_time_bdc":  bytes(d["BUS_Time_Bdc"][i]),
            }
            for c in ["X", "Y", "Z", "Vx", "Vy", "Vz", "Lon", "Lat", "Alt"]:
                row[c] = float(orbit_vals[c][i])
            for c in att_vals:
                row[c] = float(att_vals[c][i])
            rows.append(row)
    return rows


def _index_by_time(table: dict, time_key: str = "Time") -> dict:
    """Attach an int64 met_sec → row-index dict for fast lookup."""
    idx = {int(t): i for i, t in enumerate(table[time_key])}
    table["__index_of"] = idx
    return table


def extract_day(date: str) -> pd.DataFrame:
    """Build the full per-sec dataframe for one UTC date."""
    rows: list[dict] = []
    for hour in range(24):
        # Load 1K aux once per hour (covers all 18 dets)
        hv_path    = find_1k_aux_path(date, hour, "HE-HV")
        orbit_path = find_1k_aux_path(date, hour, "Orbit")
        att_path   = find_1k_aux_path(date, hour, "Att")
        evt_path   = find_1k_aux_path(date, hour, "HE-Evt")

        hv_table    = _index_by_time(read_he_hv(hv_path))   if hv_path    else None
        orbit_table = _index_by_time(read_orbit(orbit_path)) if orbit_path else None
        evt_table   = read_he_evt(evt_path)                  if evt_path   else None

        for box in ["A", "B", "C"]:
            # Att depends on target seconds: defer to per-(box, hour) where we know n_sec
            eng_path = find_he_eng_path(date, hour, BOX_PORTS[box])
            if eng_path is None:
                continue
            d_probe = read_he_eng(eng_path)
            offset = compute_offset(int(d_probe["UTC_Last_Bdc"][0]),
                                     int(d_probe["sTime_Last_Bdc"][0]))
            met_sec_probe = np.floor(
                compute_met_float(d_probe["Time"], offset)
            ).astype(np.int64)
            att_vals = (read_att(att_path, met_sec_probe)
                        if att_path else None)

            rows.extend(_box_hour_rows(
                date, box, hour,
                hv_lookup=hv_table,
                orbit_lookup=orbit_table,
                att_lookup=att_vals,
                evt=evt_table,
            ))

    return pd.DataFrame(rows)


def write_parquet_atomic(df: pd.DataFrame, output_path: Path) -> None:
    """Write DataFrame to parquet atomically: write to temp file, then rename.

    This ensures that a partial write never overwrites the destination.
    """
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        suffix=".parquet",
        dir=output_path.parent,
        delete=False,
    ) as tmp:
        tmp_path = Path(tmp.name)
    try:
        df.to_parquet(tmp_path, index=False)
        tmp_path.replace(output_path)
    except Exception:
        # Clean up temp file on error
        tmp_path.unlink(missing_ok=True)
        raise


def main() -> int:
    """CLI entry point: extract per-second HXMT/HE data for one UTC date."""
    parser = argparse.ArgumentParser(
        description="Extract per-second HXMT/HE data (one date per invocation)."
    )
    parser.add_argument(
        "date",
        help="UTC date in YYYYMMDD format (e.g. 20260410)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("per_sec_parquet"),
        help="Output directory for parquet files (default: per_sec_parquet)",
    )

    args = parser.parse_args()

    # Validate date format
    date_str = args.date
    if len(date_str) != 8 or not date_str.isdigit():
        print(f"Error: date must be in YYYYMMDD format, got '{date_str}'", file=sys.stderr)
        return 1

    output_file = args.output_dir / f"{date_str}.parquet"

    # Idempotency: if output exists, exit 0 (no-op)
    if output_file.exists():
        print(f"Output already exists: {output_file}", file=sys.stderr)
        return 0

    print(f"Extracting {date_str}...", file=sys.stderr)
    try:
        df = extract_day(date_str)
        print(f"  Loaded {len(df)} rows", file=sys.stderr)
        write_parquet_atomic(df, output_file)
        print(f"Wrote {output_file}", file=sys.stderr)
        return 0
    except Exception as e:
        print(f"Error extracting {date_str}: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
