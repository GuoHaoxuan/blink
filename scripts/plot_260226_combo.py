#!/usr/bin/env python3
"""GRB 260226A vs Fermi/GBM: combined 4-panel aligned figure.

Top panel: full-band (all events) light curve with the GBM n0+n3+b0
reference and the engineering-channel prediction (former f7 upper
panel; constant background on (-6.5,-2)+(60,80), scale fit on the
filler-free bins of T0+20..40).
Lower three panels: NaI-selected deposited-energy bands versus GBM
NaI n0+n3, per-band scale on the same filler-free window, linear
background on (-8,-3)+(65,80) (former f14).

Replaces the separate f7/f14 pair in the paper.

Standard command (paper Fig. 6 position):
  .venv/bin/python scripts/plot_260226_combo.py \
      -o figures/f7_xsat_260226_gbm.pdf
"""
import argparse
import sys
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator

sys.path.insert(0, str(Path(__file__).parent))
from plot_hxmt_vs_gbm import (  # noqa: E402
    load_hxmt_reconstruct, load_gbm_tte, HXMT_TRIGGER_UTC_LABEL,
)
from plot_hxmt_vs_gbm_bands import load_hxmt_nai, load_gbm_nai  # noqa: E402
from plot_hxmt_vs_ibis_bands import fit_background, BANDS  # noqa: E402
from engineering_prediction import load_engineering_prediction, T_REF  # noqa: E402

BIN = 0.5
BEFORE, AFTER = 10.0, 80.0
XLIM = (-4.5, 80.0)
BB_BKG = (-6.5, -2.0, 60.0, 80.0)    # broadband: constant background
NAI_BKG = (-4.5, -0.5, 65.0, 80.0)   # NaI bands: linear bkg (left window in real data; data starts ~-5s)
SCALE_RANGE = (20.0, 40.0)
GBM_BB_DETS = ["n0", "n3", "b0"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--output", default="260226_combo.pdf")
    args = ap.parse_args()

    matplotlib.rcParams.update({
        "font.size": 12, "axes.labelsize": 13, "axes.linewidth": 0.9,
        "xtick.labelsize": 11, "ytick.labelsize": 11,
        "legend.fontsize": 9.5, "pdf.fonttype": 42,
        "xtick.direction": "in", "ytick.direction": "in",
        "xtick.top": True, "ytick.right": True,
    })

    edges = np.arange(-BEFORE, AFTER + BIN, BIN)
    x = edges[:-1] + BIN / 2

    # ---- broadband data (former f7) ----
    print("Loading HXMT/HE reconstruct...", file=sys.stderr)
    obs_t, fill_t = load_hxmt_reconstruct(BEFORE, AFTER)
    all_t = np.concatenate([obs_t, fill_t]) if len(fill_t) else obs_t
    print(f"  HXMT: {len(obs_t):,} obs + {len(fill_t):,} fill", file=sys.stderr)

    t_years = (np.datetime64("2026-02-26") - T_REF).astype(
        "timedelta64[D]").astype(float) / 365.25
    from plot_hxmt_vs_gbm import HXMT_TRIGGER_MET
    eng_t, eng_rate = load_engineering_prediction(
        date_str="20260226", hour_str="100000",
        trigger_met=HXMT_TRIGGER_MET, before=BEFORE, after=AFTER,
        t_years_const=t_years,
        orbit_path="data/hxmt_aux/HXMT_20260226T10_Orbit_FFFFFF_V1_1K.FITS")

    gbm_bb = np.concatenate([load_gbm_tte(d, BEFORE, AFTER, None, None)
                             for d in GBM_BB_DETS])

    t1, t2, t3, t4 = BB_BKG
    r_obs = np.histogram(obs_t, bins=edges)[0] / BIN
    r_all = np.histogram(all_t, bins=edges)[0] / BIN
    r_gbm = np.histogram(gbm_bb, bins=edges)[0] / BIN
    bkgm = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
    net_obs = r_obs - np.mean(r_all[bkgm])
    net_all = r_all - np.mean(r_all[bkgm])
    net_gbm = r_gbm - np.mean(r_gbm[bkgm])

    fill_bins = np.histogram(fill_t, bins=edges)[0] > 0
    sm = (x >= SCALE_RANGE[0]) & (x < SCALE_RANGE[1]) & ~fill_bins
    scale_bb = net_all[sm].sum() / net_gbm[sm].sum()
    print(f"  broadband GBM scale: {scale_bb:.2f} "
          f"({int(sm.sum())} filler-free bins)", file=sys.stderr)
    net_gbm_s = net_gbm * scale_bb

    with np.errstate(divide="ignore", invalid="ignore"):
        thr = max(np.nanmax(net_gbm_s) * 0.05, 100)
        rr = np.where(net_gbm_s > thr, net_all / net_gbm_s, np.nan)
    rg = rr[~np.isnan(rr)]
    print(f"  broadband HXMT/GBM = {np.mean(rg):.2f} ± {np.std(rg):.2f} "
          f"({len(rg)} bins)", file=sys.stderr)

    if eng_t is not None:
        ebm = ((eng_t >= t1) & (eng_t < t2)) | ((eng_t >= t3) & (eng_t < t4))
        net_eng = eng_rate - np.mean(eng_rate[ebm])

    # ---- NaI band data (former f14) ----
    # Loaded with the f14 standard window (--before 8): the filler
    # energy assignment slices gaps into sub-windows from the window
    # start, so the per-filler (channel, pw) draw depends on the
    # window; keep it identical to the published f14 numbers.
    NAI_BEFORE = 8.0
    nedges = np.arange(-NAI_BEFORE, AFTER + BIN, BIN)
    nx = nedges[:-1] + BIN / 2
    nobs_t, nobs_e, nfill_t, nfill_e = load_hxmt_nai(NAI_BEFORE, AFTER)
    nall_t = np.concatenate([nobs_t, nfill_t])
    nall_e = np.concatenate([nobs_e, nfill_e])
    gbm_t, gbm_e = load_gbm_nai(NAI_BEFORE, AFTER)
    print(f"  HXMT NaI: {len(nobs_t):,} obs + {len(nfill_t):,} fill;  "
          f"GBM NaI: {len(gbm_t):,}", file=sys.stderr)
    n1, n2, n3, n4 = NAI_BKG
    nbkgm = ((nx >= n1) & (nx < n2)) | ((nx >= n3) & (nx < n4))

    # ---- figure ----
    fig, axes = plt.subplots(
        4, 1, figsize=(10, 12), sharex=True,
        gridspec_kw={"hspace": 0.0})

    ax = axes[0]
    ax.fill_between(x, net_obs, net_all, step="mid", alpha=0.30,
                    color="#5b9bd5", zorder=2)
    ax.step(x, net_obs, where="mid", color="#20347e", lw=1.0,
            label="HXMT/HE observed", zorder=3)
    ax.step(x, net_all, where="mid", color="#5b9bd5", lw=1.2,
            label=f"HXMT/HE + reconstructed (+{len(fill_t):,})", zorder=4)
    ax.step(x, net_gbm_s, where="mid", color="#e07a12", lw=1.1,
            label="Fermi/GBM n0+n3+b0 " + rf"($\times${scale_bb:.2f})",
            zorder=5)
    if eng_t is not None:
        ax.step(eng_t, net_eng, where="post", color="#2e8b57", lw=1.1,
                label=r"engineering $\widehat{S}_{\rm rec}^{\rm eng}$"
                      " (1 Hz, 18-det sum)", zorder=6)
    ax.axhline(0, color="gray", lw=0.5)
    vis = (x >= XLIM[0]) & (x <= XLIM[1])
    ax.set_ylim(-2500, np.nanmax(net_all[vis]) * 1.10)
    ax.set_ylabel("net rate (evt/s)")
    ax.text(0.02, 0.92, "all events", transform=ax.transAxes,
            fontsize=12, fontweight="bold", va="top")
    ax.legend(loc="upper right")

    for ax, (elo, ehi) in zip(axes[1:], BANDS):
        def rate(t, e):
            m = (e >= elo) & (e < ehi)
            return np.histogram(t[m], bins=nedges)[0] / BIN
        r_o = rate(nobs_t, nobs_e)
        r_a = rate(nall_t, nall_e)
        r_g = rate(gbm_t, gbm_e)
        n_o = r_o - fit_background(nx, r_o, nbkgm, 1)
        n_a = r_a - fit_background(nx, r_a, nbkgm, 1)
        n_g = r_g - fit_background(nx, r_g, nbkgm, 1)
        fb = (r_a - r_o) > 1e-9
        smc = (nx >= SCALE_RANGE[0]) & (nx < SCALE_RANGE[1]) & ~fb
        sc = n_a[smc].sum() / n_g[smc].sum()
        print(f"  band {elo:.0f}-{ehi:.0f} keV: scale x{sc:.3f}",
              file=sys.stderr)

        ax.fill_between(nx, n_o, n_a, step="mid", alpha=0.30,
                        color="#5b9bd5", zorder=2)
        ax.step(nx, n_o, where="mid", color="#20347e", lw=0.9,
                label="HXMT/HE NaI observed", zorder=3)
        ax.step(nx, n_a, where="mid", color="#5b9bd5", lw=1.0,
                label="HXMT/HE NaI obs+recon", zorder=4)
        ax.step(nx, n_g * sc, where="mid", color="tab:orange", lw=1.1,
                label="Fermi/GBM NaI " + rf"$\times${sc:.2f}", zorder=5)
        ax.axhline(0, color="gray", lw=0.7, zorder=2)
        ax.set_ylabel("net rate (counts/s)")
        ax.text(0.02, 0.90,
                f"{elo:.0f}–{ehi:.0f} keV (deposited)",
                transform=ax.transAxes, fontsize=12, fontweight="bold",
                va="top")
        ax.legend(loc="upper right")

    for ax in axes:
        ax.yaxis.set_major_locator(MaxNLocator(nbins=5, prune="both"))
    axes[-1].set_xlim(*XLIM)
    axes[-1].set_xlabel(
        "time since trigger (s)   "
        rf"[$T_0$ = {HXMT_TRIGGER_UTC_LABEL} UTC]")
    Path(args.output).parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(args.output, bbox_inches="tight")
    print(f"wrote {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
