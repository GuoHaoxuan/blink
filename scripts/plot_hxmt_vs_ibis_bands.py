#!/usr/bin/env python3
"""Band-resolved HXMT/HE (NaI) vs INTEGRAL/IBIS-ISGRI light curves for
SGR 1935+2154 / FRB 200428, in three DEPOSITED-energy bands.

HXMT/HE: NaI events (pulse_width in [54,70], per the HXMT handbook) taken from
the 1B FIFO-gap reconstruction cache that now carries per-event channel AND
pulse_width; observed events have a hole during the FIFO gap, the reconstruction
(fillers, with jointly-recovered channel+pulse_width) fills it.  Deposited
energy comes from the CALDB detector-wise 3-piece-quadratic E-C
(hxmt_he_gain_20171030_v1.fits, averaged over the 18 NaI units).

INTEGRAL/IBIS-ISGRI: photon events with ISGRI_ENERGY (deposited energy in CdTe).

CAVEAT: NaI and CdTe redistribute an incident spectrum into deposited energy
differently, so a "deposited keV band" does NOT select identical incident
photons across the two instruments (largest at the low edge and near escape
features).  This is the standard deposited/PI-energy comparison, not an
incident-energy unfold.

Reuses the light-travel / time-alignment machinery from plot_hxmt_vs_ibis.py.
"""
import argparse
import sys
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from astropy.time import Time

sys.path.insert(0, str(Path(__file__).parent))
from plot_hxmt_vs_ibis import (  # noqa: E402
    compute_hxmt_light_travel, compute_integral_light_travel, load_ibis_events,
    HXMT_TRIGGER_UTC_STR, IBIS_FILE, INTEGRAL_ORBIT_FILE, HXMT_ORBIT_FILE,
)
sys.path.insert(0, str(Path(__file__).parent / "he_nai_cal"))
from nai_pha2pi import HEGainCalibration  # noqa: E402

CACHE_PW = "data/cache_frb200428_reconstruct_3box_pw.csv"
GAINFILE = "data/hxmt_aux/hxmt_he_gain_20171030_v1.fits"
NAI_PW = (54, 70)                      # NaI pulse-width window
BANDS = [(20, 50), (50, 100), (100, 200)]  # deposited keV

# Reference peak positions (s, on our geocentric axis, T0 = 14:34:24.011 UTC).
# Mereghetti+2020 three IBIS X-ray peaks: t1,t2,t3 = 0.434/0.462/0.493 s w.r.t.
# their T0 = 14:34:24 UTC (geocentric) → subtract 0.011 s for our T0.
XRAY_PEAKS = [(0.434 - 0.011, "t1"), (0.462 - 0.011, "t2"), (0.493 - 0.011, "t3")]
# CHIME/STARE2 radio pulses: P1 = 14:34:24.4265 UTC (→ +0.4155 s), P2 = P1+28.9 ms.
RADIO_PEAKS = [(0.4155, "P1"), (0.4155 + 0.0289, "P2")]


def channel_to_kev_lut():
    """Average (over 18 NaI dets) Normal-mode channel -> deposited keV LUT."""
    cal = HEGainCalibration(GAINFILE)
    ch = np.arange(256, dtype=float)
    E = np.vstack([cal.channel_to_energy(d, ch, obs_mode="Normal") for d in range(18)])
    return np.nanmean(E, axis=0)


def load_hxmt_nai(cache, trig_met, E_lut, light_travel):
    """NaI-selected HXMT events -> (obs_t, obs_e, fill_t, fill_e) rel. trigger."""
    obs_t, obs_e, fill_t, fill_e = [], [], [], []
    with open(cache) as f:
        next(f)
        for line in f:
            p = line.split(",")
            typ, met, ch, pw = p[1], float(p[2]), int(p[3]), int(p[4])
            if not (NAI_PW[0] <= pw <= NAI_PW[1]):
                continue
            t = met - trig_met + light_travel
            e = float(E_lut[min(max(ch, 0), 255)])
            if typ == "EVT":
                obs_t.append(t); obs_e.append(e)
            elif typ == "FILL_GAP":
                fill_t.append(t); fill_e.append(e)
    return (np.asarray(obs_t), np.asarray(obs_e),
            np.asarray(fill_t), np.asarray(fill_e))


def fit_background(x, rate, bkgm, deg=1):
    """Polynomial background fit (default linear) over the off-burst windows,
    evaluated across the whole time axis — interpolates the background trend
    through the burst instead of using a flat mean."""
    coef = np.polyfit(x[bkgm], rate[bkgm], deg)
    return np.polyval(coef, x)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", type=float, default=0.005)
    ap.add_argument("--bkg-deg", type=int, default=1,
                    help="background polynomial degree (1 = linear)")
    ap.add_argument("--before", type=float, default=0.3)
    ap.add_argument("--after", type=float, default=0.9)
    ap.add_argument("--bkg", type=float, nargs=4, default=[-0.3, -0.05, 0.7, 0.9])
    ap.add_argument("--scale-range", type=float, nargs=2, default=[0.35, 0.65])
    ap.add_argument("--xlim", type=float, nargs=2, default=None,
                    metavar=("T1", "T2"),
                    help="display window (s); rates/background still use full range")
    ap.add_argument("-o", "--output", default="hxmt_vs_ibis_bands.png")
    args = ap.parse_args()

    trig = Time(HXMT_TRIGGER_UTC_STR, scale="utc")
    trig_met = (trig - Time("2012-01-01T00:00:00", scale="utc")).sec
    trig_ijd = (trig.tt - Time("2000-01-01T00:00:00", scale="tt")).to("day").value
    hlt = compute_hxmt_light_travel(trig_met, trig, HXMT_ORBIT_FILE)
    ilt = compute_integral_light_travel(trig_ijd, IBIS_FILE, INTEGRAL_ORBIT_FILE)
    print(f"  HXMT light-travel {hlt*1e3:+.1f} ms, INTEGRAL {ilt:+.3f} s", file=sys.stderr)

    E_lut = channel_to_kev_lut()
    obs_t, obs_e, fill_t, fill_e = load_hxmt_nai(CACHE_PW, trig_met, E_lut, hlt)
    all_t = np.concatenate([obs_t, fill_t])
    all_e = np.concatenate([obs_e, fill_e])
    ibis_t, ibis_e = load_ibis_events(IBIS_FILE, trig_ijd, ilt)
    print(f"  HXMT NaI: {len(obs_t):,} obs + {len(fill_t):,} fill;  "
          f"IBIS: {len(ibis_t):,} events", file=sys.stderr)

    edges = np.arange(-args.before, args.after + args.bin, args.bin)
    x = edges[:-1] + args.bin / 2
    t1, t2, t3, t4 = args.bkg
    bkgm = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
    s1, s2 = args.scale_range
    sm = (x >= s1) & (x < s2)

    fig, axes = plt.subplots(len(BANDS), 1, figsize=(10, 10), sharex=True)
    for ax, (elo, ehi) in zip(axes, BANDS):
        def rate(t, e, m0):
            m = m0 & (e >= elo) & (e < ehi)
            return np.histogram(t[m], bins=edges)[0] / args.bin
        r_obs = rate(obs_t, obs_e, np.ones(len(obs_t), bool))
        r_all = rate(all_t, all_e, np.ones(len(all_t), bool))
        r_ibis = rate(ibis_t, ibis_e, np.ones(len(ibis_t), bool))
        # IBIS 每 bin 原始计数 → 泊松误差(率误差 = sqrt(N)/bin)
        ibis_m = (ibis_e >= elo) & (ibis_e < ehi)
        n_ibis_raw = np.histogram(ibis_t[ibis_m], bins=edges)[0]
        ibis_err = np.sqrt(n_ibis_raw) / args.bin
        n_obs = r_obs - fit_background(x, r_obs, bkgm, args.bkg_deg)
        n_all = r_all - fit_background(x, r_all, bkgm, args.bkg_deg)
        n_ibis = r_ibis - fit_background(x, r_ibis, bkgm, args.bkg_deg)
        scale = n_all[sm].sum() / n_ibis[sm].sum() if n_ibis[sm].sum() > 0 else 1.0

        ax.fill_between(x, 0, n_obs, step="mid", alpha=0.5, color="C0", zorder=1)
        ax.fill_between(x, n_obs, n_all, step="mid", alpha=0.28, color="C0", zorder=2)
        ax.step(x, n_obs, where="mid", color="navy", lw=0.9,
                label="HXMT/HE NaI observed", zorder=3)
        ax.step(x, n_all, where="mid", color="C0", lw=0.9,
                label="HXMT/HE NaI reconstructed", zorder=4)
        ax.fill_between(x, (n_ibis - ibis_err) * scale, (n_ibis + ibis_err) * scale,
                        step="mid", color="tab:orange", alpha=0.25, lw=0, zorder=3)
        ax.step(x, n_ibis * scale, where="mid", color="tab:orange", lw=1.1,
                label=f"INTEGRAL/IBIS-ISGRI ×{scale:.1f} (±√N band)", zorder=5)
        ax.axhline(0, color="grey", lw=0.5)
        # 标注 X 射线三峰(红虚)与射电两峰(绿点)
        for tp, lbl in XRAY_PEAKS:
            ax.axvline(tp, color="tab:red", ls="--", lw=0.9, alpha=0.6, zorder=0)
        for rp, lbl in RADIO_PEAKS:
            ax.axvline(rp, color="tab:green", ls=":", lw=1.0, alpha=0.7, zorder=0)
        ax.set_ylabel("net rate (counts/s)")
        if args.xlim:
            ax.set_xlim(*args.xlim)
            vis = (x >= args.xlim[0]) & (x < args.xlim[1])
            ytop = max(n_all[vis].max(), (n_ibis * scale)[vis].max()) * 1.18
            ax.set_ylim(min(0, n_all[vis].min() * 1.1), ytop)
        else:
            ax.set_xlim(-args.before, args.after)
        ymax = ax.get_ylim()[1]
        if ax is axes[0]:
            for tp, lbl in XRAY_PEAKS:
                ax.text(tp, ymax * 0.80, lbl, color="tab:red", fontsize=7, ha="center")
            for rp, lbl in RADIO_PEAKS:
                ax.text(rp, ymax * 0.63, lbl, color="tab:green", fontsize=7, ha="center")
        ax.text(0.015, 0.90, f"{elo}–{ehi} keV (deposited)",
                transform=ax.transAxes, fontweight="bold")
        h, la = ax.get_legend_handles_labels()
        h += [Line2D([0], [0], color="tab:red", ls="--", lw=0.9),
              Line2D([0], [0], color="tab:green", ls=":", lw=1.0)]
        la += ["IBIS X-ray peaks (Mereghetti+20)", "radio pulses (CHIME)"]
        ax.legend(h, la, fontsize=7, loc="upper right")

    axes[-1].set_xlabel(
        f"time since trigger (s)   [T0 = {HXMT_TRIGGER_UTC_STR} UTC]")
    axes[0].set_title(
        "FRB/XRB 200428  —  HXMT/HE (NaI) vs INTEGRAL/IBIS "
        f"[{args.bin*1e3:.0f} ms bins]")
    fig.tight_layout()
    fig.savefig(args.output, dpi=130)
    print(f"wrote {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
