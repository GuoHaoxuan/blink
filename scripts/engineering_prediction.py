#!/usr/bin/env python3
"""Engineering-channel S_rec^eng prediction loader.

Shared by plot_hxmt_vs_gbm.py and plot_hxmt_vs_gecam.py to build the
1-Hz engineering-channel rate trace (C25 model applied to per-second
P/L/W/D counters, summed across 18 detectors).
"""
from __future__ import annotations
import json, sys
from pathlib import Path
import numpy as np
from astropy.io import fits
from scipy.interpolate import RegularGridInterpolator

MET_CORRECTION = 4.0
L_CYC_TO_SEC = 16e-6
BOX_OFFSET = {"A": 0, "B": 6, "C": 12}
BOX_CODE = {"A": "0766", "B": "1009", "C": "1781"}
T_REF = np.datetime64("2017-06-22")

_C25 = json.loads(Path("/tmp/per_det_25param.json").read_text())
_A_DET = np.array(_C25["a_det"])
_ALPHA = _C25["alpha"]; _MU_M = _C25["mu_m"]; _K_M = _C25["k_m"]
_AMP0 = _C25["amp0"]; _MU_T = _C25["mu_t"]; _K_T = _C25["k_t"]; _C0 = _C25["C0"]


def _sigm(x):
    return 1.0 / (1.0 + np.exp(-np.clip(x, -50, 50)))


def _c25_baseline(det_global, mlat_abs, t_years):
    A = _A_DET[det_global]
    sm = _sigm((mlat_abs - _MU_M) / _K_M)
    st = _sigm((t_years - _MU_T) / _K_T)
    g = 1.0 + _ALPHA * sm
    return A * g * (1.0 - _AMP0 * g * st) + _C0


def _unwrap_large(pho, large):
    """Self-calibrating 10-bit Large unwrap.
    Same recipe as per_burst_eng_ratio.py.
    """
    pho = pho.astype(float); large = large.astype(float)
    low = (pho > 200) & (pho < 2500) & (large < 900)
    r = (large[low] / pho[low]).mean() if low.sum() >= 20 else 0.3
    predicted = r * pho
    n_wraps = np.maximum(np.round((predicted - large) / 1024.).astype(int), 0)
    return large + n_wraps * 1024.0


def load_engineering_prediction(date_str, hour_str, trigger_met, before, after,
                                 t_years_const, orbit_path=None,
                                 aacgm_grid="n_below_study/aacgm_grid_2020.npz"):
    """Compute S_rec_eng at 1-Hz cadence summed across 18 detectors.

    Args:
        date_str:  'YYYYMMDD' (e.g., '20260226')
        hour_str:  'HHMMSS' for FITS file lookup (e.g., '100000')
        trigger_met: HXMT MET of T0 (in 'naive' system used by blink)
        before, after: seconds before/after trigger to load
        t_years_const: years since T_REF (constant for short bursts)
        orbit_path: optional path to orbit FITS for MLAT-resolved C25
        aacgm_grid: path to AACGM lookup grid (used only if orbit_path given)

    Returns:
        (t_rel, sci_eng_rate) where t_rel is seconds from trigger and
        sci_eng_rate is in evt/s summed across the 18 detectors.
        Returns (None, None) if no data is found.
    """
    t_lo = trigger_met - before
    t_hi = trigger_met + after

    # Optional MLAT lookup
    mlat_lookup = None
    if orbit_path and Path(orbit_path).exists():
        _grid = np.load(aacgm_grid)
        MLAT_INTERP = RegularGridInterpolator(
            (_grid["lat_grid"], _grid["lon_grid"]), _grid["mlat"],
            bounds_error=False, fill_value=0.0,
        )
        with fits.open(orbit_path) as orb:
            orb_t = orb[1].data["Time"].astype(float)
            orb_lat = orb[1].data["Lat"].astype(float)
            orb_lon = orb[1].data["Lon"].astype(float)
        mlat_lookup = (orb_t, orb_lat, orb_lon, MLAT_INTERP)

    sec_to_total = {}
    for box, code in BOX_CODE.items():
        folder = Path(f"data/1B/{date_str[:4]}/{date_str}/{code}")
        prefix = f"HXMT_1B_{code}_{date_str}T{hour_str}"
        matches = sorted(folder.glob(f"{prefix}*.fits"))
        if not matches:
            print(f"  WARN: no 1B FITS at {folder}/{prefix}*", file=sys.stderr)
            continue
        fe = fits.open(matches[0], memmap=True)
        d = fe["HE_Eng"].data
        offset = d["UTC_Last_Bdc"][0] - d["sTime_Last_Bdc"][0]
        met = d["Time"].astype(float) + offset + MET_CORRECTION
        lc_all = d["Length_Time_Cycle"].astype(float)
        mask = (met >= t_lo) & (met <= t_hi)
        met = met[mask]; lc = lc_all[mask]; L = lc * L_CYC_TO_SEC

        if mlat_lookup is not None:
            orb_t, orb_lat, orb_lon, MLAT_INTERP = mlat_lookup
            lat_at = np.interp(met, orb_t, orb_lat)
            lon_at = np.interp(met, orb_t, orb_lon)
            pts = np.column_stack([lat_at, lon_at])
            mlat_abs = np.abs(MLAT_INTERP(pts))
            mlat_abs = np.where(np.isnan(mlat_abs), 0.0, mlat_abs)
        else:
            mlat_abs = np.zeros_like(met)

        for det_local in range(6):
            det_global = BOX_OFFSET[box] + det_local
            pho = d[f"Cnt_PHODet_{det_global}"].astype(float)[mask]
            wide = d[f"Cnt_CsI_PHODet_{det_global}"].astype(float)[mask]
            large_raw = d[f"Cnt_LargeEvt_{det_global}"].astype(float)[mask]
            dt = d[f"DeadTime_PHODet_{det_global}"].astype(float)[mask]
            large = _unwrap_large(pho, large_raw)
            lf_det = 1.0 - dt / lc
            C_per = _c25_baseline(det_global, mlat_abs, t_years_const)
            sci_eng = (pho - large) * lf_det / L - wide / L - C_per
            for i in range(len(met)):
                key = int(round(met[i]))
                sec_to_total.setdefault(key, 0.0)
                sec_to_total[key] += sci_eng[i]
        fe.close()

    if not sec_to_total:
        return None, None
    secs = np.array(sorted(sec_to_total.keys()))
    rates = np.array([sec_to_total[s] for s in secs])
    t_rel = secs - trigger_met
    return t_rel, rates
