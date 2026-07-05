#!/usr/bin/env python3
"""Quantitative numbers for the GRB 221009A HE light-curve paper.

Recomputes every number quoted in the paper from the corrected pipeline
(gross/f_live - B dead-time correction) and prints a CSV to stdout.
Run from the blink repo root:
    ./.venv/bin/python scripts/numbers_221009a.py > ../paper-hxmt-221009a/numbers.csv
"""
import sys
sys.path.insert(0, "scripts")
import numpy as np
from astropy.stats import bayesian_blocks

from make_f10 import (load_hxmt, load_eng_dtau, to_grid, GECAM_SCALE)
from plot_hxmt_vs_gecam import compute_time_offset, load_gecam_btime

BEFORE, AFTER, BIN = 50.0, 700.0, 1.0

rows = []
def emit(metric, value, unit="", note=""):
    rows.append((metric, value, unit, note))

# ---- load ----
x, edges, obs, allr, n_fill, B_evt = load_hxmt(BEFORE, AFTER, BIN)
et, dtau, flive, tau_us, B_eng = load_eng_dtau(BEFORE, AFTER)
g_met, _, _ = compute_time_offset()
_, g_rate, _ = load_gecam_btime(g_met, BEFORE, AFTER, BIN, "lg", True)
g_rate = np.nan_to_num(g_rate, nan=0.0)
gecam_s = g_rate * GECAM_SCALE

flive_g = to_grid(x, et, flive)
with np.errstate(divide="ignore", invalid="ignore"):
    all_dt = np.where(flive_g > 0.05, (allr + B_evt) / flive_g - B_evt, np.nan)

emit("B_evt", f"{B_evt:.0f}", "evt/s", "gross event-level background, mean of T0-50..-10")
emit("tau_det_eng_us", f"{np.nanmean(tau_us):.1f}", "us",
     "per-detector per-event processing time, D/Cnt_PHODet tail T0+300..700")

# ---- live fraction extremes ----
win_main = (et >= 170) & (et < 310)
i_min = np.nanargmin(np.where(win_main, flive, np.nan))
emit("flive_min", f"{flive[i_min]:.3f}", "", "deepest 18-det mean live fraction, main window")
emit("flive_min_time", f"{et[i_min]:.0f}", "s", "time of deepest live fraction")

# ---- main-pulse peaks (DT-corrected), split at T0+240 ----
m1 = (x >= 170) & (x < 240)
m2 = (x >= 240) & (x < 310)
for name, m in (("peak1", m1), ("peak2", m2)):
    r = np.where(m, all_dt, np.nan)
    i = np.nanargmax(r)
    emit(f"{name}_time", f"{x[i]:.0f}", "s", f"{name} time of DT-corrected maximum")
    emit(f"{name}_rate_dt", f"{r[i]:.3e}", "evt/s", f"{name} DT-corrected net rate")
emit("peak_obs_ceiling", f"{np.nanmax(np.where((x>=170)&(x<310), obs, np.nan)):.3e}",
     "evt/s", "max observed (FIFO-limited) rate in main window")

# ---- main-pulse integrated counts ----
mwin = (x >= 180) & (x < 300)
vals = all_dt[mwin]
emit("mainpulse_window", "180..300", "s", "integration window")
emit("mainpulse_counts_dt", f"{np.nansum(vals)*BIN:.3e}", "counts",
     "DT-corrected net counts, NaN bins excluded")
emit("mainpulse_nan_bins", f"{int(np.sum(~np.isfinite(vals)))}", "bins",
     "bins excluded (SAA-mode shutoff / f_live<=0.05 / no eng frame)")

# ---- flare: Bayesian blocks + T90 on the reconstructed net rate ----
fwin = (x >= 400) & (x < 700)
xf, rf = x[fwin], allr[fwin]
ok = np.isfinite(rf)
bkg_sig = np.nanstd(allr[(x >= -50) & (x < -10)])
edges_bb = bayesian_blocks(xf[ok], rf[ok], sigma=bkg_sig, fitness="measures", p0=0.001)
rate_bb = [np.nanmean(rf[ok][(xf[ok] >= lo) & (xf[ok] < hi)])
           for lo, hi in zip(edges_bb[:-1], edges_bb[1:])]
above = [i for i, r in enumerate(rate_bb) if r > 3 * bkg_sig]
bb_start, bb_end = edges_bb[above[0]], edges_bb[above[-1] + 1]
emit("flare_bb_start", f"{bb_start:.0f}", "s", "Bayesian-blocks flare start (3sigma)")
emit("flare_bb_end", f"{bb_end:.0f}", "s", "Bayesian-blocks flare end (3sigma)")
sel = (xf >= bb_start) & (xf < bb_end) & ok
cum = np.cumsum(rf[sel]) / np.sum(rf[sel])
t_sel = xf[sel]
t05, t95 = t_sel[np.searchsorted(cum, 0.05)], t_sel[np.searchsorted(cum, 0.95)]
emit("flare_t90", f"{t95 - t05:.0f}", "s", "5%..95% cumulative net counts inside BB bounds")
emit("flare_t90_start", f"{t05:.1f}", "s", "")
emit("flare_t90_end", f"{t95:.1f}", "s", "")
emit("flare_peak_time", f"{t_sel[np.nanargmax(rf[sel])]:.0f}", "s", "brightest 1-s bin")
fill_mask = (x >= bb_start) & (x < bb_end)
emit("flare_recovered_events", f"{np.nansum((allr - obs)[fill_mask])*BIN:.2e}", "counts",
     "reconstructed-minus-observed net events across flare")

# ---- ratio stats (same significance cut as make_f10 main mode) ----
def robust(rr):
    v = rr[np.isfinite(rr)]
    if len(v) == 0:
        return None
    med = np.median(v)
    iqr = np.subtract(*np.percentile(v, [75, 25]))
    return med, iqr / 1.349, len(v)

g_net = gecam_s / GECAM_SCALE
# main window (170..310) and flare window (440..610); same window-local
# G_FLOOR=50 cts/s significance cut make_f10 uses (H_FLOOR=0 off the full burst).
for win_tag, (wlo, whi) in (("main", (170, 310)), ("flare", (440, 610))):
    win = (x >= wlo) & (x < whi)
    sig = win & np.isfinite(g_net) & (g_net > 50.0) & np.isfinite(allr) & (allr > 0.0)
    for kind, num in (("raw", allr), ("dt", all_dt)):
        with np.errstate(divide="ignore", invalid="ignore"):
            rr = np.where(sig & np.isfinite(num), num / gecam_s, np.nan)
        st = robust(rr)
        med, s, n = st if st else ("nan", "nan", 0)
        tag = f"ratio_{win_tag}_{kind}"
        emit(f"{tag}_med", f"{med:.2f}" if st else "nan", "", "HXMT/GECAM median")
        emit(f"{tag}_sig", f"{s:.2f}" if st else "nan", "", "IQR-derived robust sigma")
        emit(f"{tag}_n", f"{n}", "bins", "significant 1-s bins (GECAM net > 50 cts/s)")

print("metric,value,unit,note")
for r in rows:
    print(",".join(str(c) for c in r))
