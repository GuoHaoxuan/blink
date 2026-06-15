#!/usr/bin/env python3
"""Plot HXMT/HE reconstructed light curve vs Fermi/GBM.

Usage:
    python3 scripts/plot_hxmt_vs_gbm.py --bin 0.1
    python3 scripts/plot_hxmt_vs_gbm.py --bin 1.0 --before 20 --after 200
"""

import argparse, subprocess, os, sys
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from astropy.io import fits
from datetime import datetime, timezone, timedelta

# ── Config ──
HXMT_MET_EPOCH = datetime(2012, 1, 1, tzinfo=timezone.utc)
HXMT_TRIGGER_UTC = "2026-02-26T10:37:53"
HXMT_EPOCH = "2026-02-26T10"

GBM_DIR = "data/fermi_gbm/bn260226443"
GBM_TRIGGER_MET = 793795080.95811  # Fermi MET (TT seconds since 2001-01-01 UTC)
# GBM detectors: n0, n3 triggered; b0, b1 BGO
GBM_DETS = ["n0", "n3", "b0", "b1"]

# HXMT trigger in HXMT MET (naive, same convention as blink_cli)
HXMT_TRIGGER_MET = (datetime.strptime(HXMT_TRIGGER_UTC, "%Y-%m-%dT%H:%M:%S")
                     .replace(tzinfo=timezone.utc) - HXMT_MET_EPOCH).total_seconds()

# ── Time alignment ──
#
# Time systems:
#   HXMT/HE: MET counts SI seconds since 2012-01-01T00:00:00 UTC.
#     blink_cli uses naive (chrono) subtraction, same as Python datetime:
#     both give MET=446726273 for the string "10:37:53", ignoring 3 leap seconds
#     (2012-06, 2015-06, 2016-12). The actual UTC is 10:37:50, but blink_cli
#     output METs use the same naive basis, so relative times are self-consistent.
#   Fermi/GBM: TIMESYS=TT, MET in TT seconds since MJD 51910.0 UTC (2001-01-01).
#     MJDREFF = 64.184/86400 = TT-UTC at epoch. No leap second within the
#     ~10-minute observation window, so relative TT times equal relative UTC times.
#
# Absolute trigger times (astropy, accounting for leap seconds + TT-UTC):
#   GBM trigger UTC:  2026-02-26 10:37:55.958
#   HXMT trigger UTC: 2026-02-26 10:37:50.000
#   Offset: GBM T=0 is 5.958s after HXMT T=0
#
# Light travel time correction:
#   Fermi ECI at trigger: [4761, 4718, 1491] km, projection = +22.8 ms
#   HXMT in LEO (~550 km): max projection = +23.1 ms
#   Maximum differential LTT between the two LEO satellites: <47 ms
#   Negligible at 0.5s bin resolution.
#
# The correct HXMT trigger UTC for labeling is 10:37:50, not 10:37:53.
HXMT_TRIGGER_UTC_LABEL = "2026-02-26T10:37:50"
GBM_TO_HXMT_OFFSET = 5.958  # GBM T=0 is 5.958s after HXMT T=0


def load_hxmt_reconstruct(before, after):
    """Load HXMT 1B reconstructed events (observed + filled)."""
    cmd = ["./target/release/blink_cli", "sat", "reconstruct", HXMT_TRIGGER_UTC,
           "--before", str(before), "--after", str(after)]
    env = os.environ.copy()
    env.setdefault("HXMT_1B_DIR", "data/1B")
    env.setdefault("HXMT_1K_DIR", "data/1K")
    print(f"  Running: {' '.join(cmd)}", file=sys.stderr)
    proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if proc.stderr:
        for line in proc.stderr.strip().split("\n"):
            print(f"    {line}", file=sys.stderr)

    obs, fill = [], []
    for line in proc.stdout.strip().split("\n"):
        p = line.split(",")
        if len(p) < 3 or p[0] == "box":
            continue
        met = float(p[2])
        t = met - HXMT_TRIGGER_MET
        if p[1] == "EVT":
            obs.append(t)
        elif p[1] == "FILL_GAP":
            fill.append(t)
    return np.array(obs), np.array(fill)


def load_gbm_tte(det, before, after):
    """Load Fermi/GBM TTE events for one detector, all channels (no energy filter)."""
    path = os.path.join(GBM_DIR, f"glg_tte_{det}_bn260226443_v00.fit")
    if not os.path.exists(path):
        return np.array([])
    with fits.open(path, memmap=True) as f:
        times = f["EVENTS"].data["TIME"]
    # Convert to time relative to HXMT trigger (apply offset)
    t = (times - GBM_TRIGGER_MET) + GBM_TO_HXMT_OFFSET
    mask = (t >= -before) & (t <= after)
    return t[mask]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", type=float, default=0.5, help="Bin width (seconds)")
    parser.add_argument("--before", type=float, default=10.0)
    parser.add_argument("--after", type=float, default=80.0)
    parser.add_argument("--det", type=str, nargs="+", default=["n0", "n3"],
                        help="GBM detectors to use")
    # Energy filtering removed: use all channels, let scale factor absorb
    # effective area / energy band differences
    parser.add_argument("--bkg", type=float, nargs=4, default=[-10, -2, 60, 80],
                        metavar=("T1", "T2", "T3", "T4"),
                        help="Background intervals: [T1,T2] and [T3,T4]")
    parser.add_argument("-o", "--output", default="hxmt_vs_gbm.png")
    args = parser.parse_args()

    # Load HXMT data
    print("Loading HXMT/HE reconstruct...", file=sys.stderr)
    hxmt_obs, hxmt_fill = load_hxmt_reconstruct(args.before, args.after)
    hxmt_all = np.concatenate([hxmt_obs, hxmt_fill]) if len(hxmt_fill) > 0 else hxmt_obs
    print(f"  HXMT: {len(hxmt_obs):,} obs + {len(hxmt_fill):,} fill = {len(hxmt_all):,}",
          file=sys.stderr)

    # Load GBM data
    gbm_events = {}
    for det in args.det:
        print(f"Loading GBM {det}...", file=sys.stderr)
        evts = load_gbm_tte(det, args.before, args.after)
        gbm_events[det] = evts
        print(f"  {det}: {len(evts):,} events", file=sys.stderr)

    # Combine NaI detectors
    gbm_combined = np.concatenate([gbm_events[d] for d in args.det])
    print(f"  GBM combined ({'+'.join(args.det)}): {len(gbm_combined):,}", file=sys.stderr)

    # ── Background subtraction ──
    bin_w = args.bin
    edges = np.arange(-args.before, args.after + bin_w, bin_w)
    x = edges[:-1]
    t1, t2, t3, t4 = args.bkg

    # HXMT rates
    r_hxmt_obs = np.histogram(hxmt_obs, bins=edges)[0] / bin_w
    r_hxmt_all = np.histogram(hxmt_all, bins=edges)[0] / bin_w

    # GBM rates
    r_gbm = np.histogram(gbm_combined, bins=edges)[0] / bin_w

    # Background: average rate in [t1,t2] + [t3,t4]
    bkg_mask = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
    n_bkg = bkg_mask.sum()
    if n_bkg > 0:
        bkg_hxmt = np.mean(r_hxmt_all[bkg_mask])
        bkg_gbm = np.mean(r_gbm[bkg_mask])
    else:
        bkg_hxmt = 0
        bkg_gbm = 0
    print(f"  Background: HXMT={bkg_hxmt:.0f} evt/s, GBM={bkg_gbm:.0f} evt/s "
          f"(from [{t1},{t2}]+[{t3},{t4}], {n_bkg} bins)", file=sys.stderr)

    # Net rates
    net_hxmt_obs = r_hxmt_obs - bkg_hxmt
    net_hxmt_all = r_hxmt_all - bkg_hxmt
    net_gbm = r_gbm - bkg_gbm

    # Scale GBM net to HXMT net: match total net counts in burst region
    burst_mask = (x >= t2) & (x < t3)
    sum_hxmt = np.sum(net_hxmt_all[burst_mask])
    sum_gbm = np.sum(net_gbm[burst_mask])
    scale = sum_hxmt / sum_gbm if sum_gbm > 0 else 1.0
    net_gbm_scaled = net_gbm * scale
    print(f"  Scale factor: {scale:.2f} (matched in [{t2},{t3}])", file=sys.stderr)

    # ── Plot ──
    fig, (ax_lc, ax_ratio) = plt.subplots(
        2, 1, figsize=(12, 7), sharex=True,
        gridspec_kw={"height_ratios": [3, 1], "hspace": 0.05})

    # 1) HXMT observed (blue)
    ax_lc.step(x, net_hxmt_obs, where="post", color="C0", lw=0.8,
               label="HXMT/HE observed", zorder=3)
    ax_lc.fill_between(x, 0, net_hxmt_obs, step="post", alpha=0.15, color="C0")
    # 2) HXMT observed + filled (orange)
    ax_lc.step(x, net_hxmt_all, where="post", color="C1", lw=0.8,
               label=f"HXMT/HE + reconstructed (+{len(hxmt_fill):,})", zorder=2)
    ax_lc.fill_between(x, net_hxmt_obs, net_hxmt_all, step="post", alpha=0.3, color="C1")
    # 3) Reference instrument (green)
    ax_lc.step(x, net_gbm_scaled, where="post", color="C2", lw=1.0,
               label=f"Fermi/GBM {'+'.join(args.det)} (\u00d7{scale:.1f})", zorder=4)

    ax_lc.set_ylabel("Net count rate (evt/s)")
    ax_lc.legend(loc="upper right")
    ax_lc.axhline(0, color="gray", lw=0.5, ls="--")
    ax_lc.set_title(f"GRB 260226A: HXMT/HE vs Fermi/GBM  [{bin_w}s bins, geocentric]",
                    fontweight="bold")

    # Ratio panel
    with np.errstate(divide='ignore', invalid='ignore'):
        # Only show ratio where GBM signal is significant (>5% of peak)
        peak_gbm = np.nanmax(net_gbm_scaled)
        threshold = max(peak_gbm * 0.05, 100)
        ratio = np.where(net_gbm_scaled > threshold, net_hxmt_all / net_gbm_scaled, np.nan)
    ax_ratio.step(x, ratio, where="post", color="k", lw=0.8)
    ax_ratio.axhline(1.0, color="gray", lw=0.5, ls="--")
    ax_ratio.set_ylabel("HXMT / GBM")
    ax_ratio.set_ylim(0.5, 1.5)
    # Add ratio stats
    rv = ratio[~np.isnan(ratio)]
    if len(rv) > 0:
        ax_ratio.text(0.98, 0.85, f"ratio = {np.mean(rv):.2f} ± {np.std(rv):.2f} ({len(rv)} bins)",
                      transform=ax_ratio.transAxes, ha="right", va="top", fontsize=9,
                      bbox=dict(facecolor="white", alpha=0.8))
    ax_ratio.set_xlabel(f"Time since HXMT trigger (s)  [$T_0$ = {HXMT_TRIGGER_UTC_LABEL} UTC]")
    ax_ratio.set_xlim(-args.before, args.after)

    plt.tight_layout()
    plt.savefig(args.output, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"Saved: {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
