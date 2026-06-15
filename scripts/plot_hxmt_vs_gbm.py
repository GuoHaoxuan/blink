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
GBM_TRIGGER_MET = 793795080.95811  # Fermi MET
# GBM detectors: n0, n3 triggered; b0, b1 BGO
GBM_DETS = ["n0", "n3", "b0", "b1"]

# HXMT trigger in HXMT MET
HXMT_TRIGGER_MET = (datetime.strptime(HXMT_TRIGGER_UTC, "%Y-%m-%dT%H:%M:%S")
                     .replace(tzinfo=timezone.utc) - HXMT_MET_EPOCH).total_seconds()

# Time alignment (computed with astropy, accounting for leap seconds):
# GBM trigger UTC:  2026-02-26 10:37:55.958 (Fermi MET in TT seconds from 2001-01-01 UTC)
# HXMT trigger UTC: 2026-02-26 10:37:50.000 (HXMT MET in SI seconds from 2012-01-01 UTC)
# Note: Python datetime ignores 3 leap seconds (2012-06, 2015-06, 2016-12),
#       giving the wrong result 10:37:53. Must use astropy for correct conversion.
# Light travel time between two LEO satellites: <47ms, negligible at 0.5s bins.
GBM_TO_HXMT_OFFSET = 5.958  # GBM T=0 is 5.958s after HXMT T=0


def load_hxmt_reconstruct(before, after):
    """Load HXMT 1B reconstructed events (observed + filled)."""
    cmd = ["./target/release/blink_cli", "sat", HXMT_EPOCH,
           "reconstruct", HXMT_TRIGGER_UTC,
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


def load_gbm_tte(det, before, after, emin=200, emax=900):
    """Load Fermi/GBM TTE events for one detector, filtered to energy range."""
    path = os.path.join(GBM_DIR, f"glg_tte_{det}_bn260226443_v00.fit")
    if not os.path.exists(path):
        return np.array([])
    with fits.open(path, memmap=True) as f:
        times = f["EVENTS"].data["TIME"]
        pha = f["EVENTS"].data["PHA"]
        ebounds = f["EBOUNDS"].data
    # Find channels matching energy range
    ch_min = None
    ch_max = None
    for row in ebounds:
        if ch_min is None and row["E_MAX"] >= emin:
            ch_min = row["CHANNEL"]
        if row["E_MIN"] <= emax:
            ch_max = row["CHANNEL"]
    if ch_min is None or ch_max is None:
        return np.array([])
    # Exclude overflow channel
    overflow = 127 if det.startswith("n") else 127
    ch_max = min(ch_max, overflow - 1)
    print(f"    {det}: ch {ch_min}-{ch_max} = {ebounds[ch_min]['E_MIN']:.0f}-{ebounds[ch_max]['E_MAX']:.0f} keV",
          file=sys.stderr)
    # Filter
    # Convert to time relative to HXMT trigger (apply offset)
    t = (times - GBM_TRIGGER_MET) + GBM_TO_HXMT_OFFSET
    mask = (t >= -before) & (t <= after) & (pha >= ch_min) & (pha <= ch_max)
    return t[mask]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", type=float, default=0.5, help="Bin width (seconds)")
    parser.add_argument("--before", type=float, default=10.0)
    parser.add_argument("--after", type=float, default=80.0)
    parser.add_argument("--det", type=str, nargs="+", default=["n0", "n3"],
                        help="GBM detectors to use")
    parser.add_argument("--emin", type=float, default=200, help="GBM energy min (keV)")
    parser.add_argument("--emax", type=float, default=900, help="GBM energy max (keV)")
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
        evts = load_gbm_tte(det, args.before, args.after, args.emin, args.emax)
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
        2, 1, figsize=(16, 8), sharex=True,
        gridspec_kw={"height_ratios": [3, 1], "hspace": 0.06})

    C0, C1, C2 = plt.cm.tab10(0), plt.cm.tab10(1), plt.cm.tab10(2)
    # 1) HXMT observed: // C0
    ax_lc.fill_between(x, np.maximum(net_hxmt_obs, 0), step="post",
                       facecolor="none", hatch="//", edgecolor=C0, alpha=0.5,
                       linewidth=0, zorder=2,
                       label=f"HXMT/HE observed")
    ax_lc.step(x, net_hxmt_obs, where="post", color=C0, lw=1.0, zorder=3)
    # 2) HXMT filled part only (between obs and total): // C1
    ax_lc.fill_between(x, np.maximum(net_hxmt_obs, 0), np.maximum(net_hxmt_all, 0),
                       step="post",
                       facecolor="none", hatch="//", edgecolor=C1, alpha=0.5,
                       linewidth=0, zorder=4,
                       label=f"HXMT/HE filled (+{len(hxmt_fill):,})")
    ax_lc.step(x, net_hxmt_all, where="post", color=C1, lw=1.0, zorder=5)
    # 3) GBM: \\ C2
    ax_lc.fill_between(x, np.maximum(net_gbm_scaled, 0), step="post",
                       facecolor="none", hatch="\\\\", edgecolor=C2, alpha=0.5,
                       linewidth=0, zorder=6,
                       label=f"Fermi/GBM {'+'.join(args.det)} (×{scale:.1f})")
    ax_lc.step(x, net_gbm_scaled, where="post", color=C2, lw=1.0, zorder=7)

    ax_lc.set_ylabel("Net count rate (evt/s)", fontsize=13)
    ax_lc.legend(loc="upper right", fontsize=10)
    ax_lc.set_ylim(bottom=0)
    ax_lc.grid(alpha=0.15)
    ax_lc.set_title(f"GRB 260226A: HXMT/HE (reconstructed) vs Fermi/GBM  "
                     f"[{args.emin:.0f}-{args.emax:.0f} keV, {bin_w}s bins, bkg subtracted]",
                     fontsize=13, fontweight="bold")

    # Ratio panel
    with np.errstate(divide='ignore', invalid='ignore'):
        # Only show ratio where GBM signal is significant (>5% of peak)
        peak_gbm = np.nanmax(net_gbm_scaled)
        threshold = max(peak_gbm * 0.05, 100)
        ratio = np.where(net_gbm_scaled > threshold, net_hxmt_all / net_gbm_scaled, np.nan)
    ax_ratio.step(x, ratio, where="post", color="#333333", lw=0.8, zorder=2)
    ax_ratio.axhline(1.0, color="black", lw=0.5, ls="--", alpha=0.5)
    ax_ratio.set_ylabel("HXMT / GBM", fontsize=13)
    ax_ratio.set_ylim(0.5, 1.5)
    ax_ratio.grid(alpha=0.15)
    ax_ratio.set_xlabel(f"Time since HXMT trigger (s)  [T₀ = {HXMT_TRIGGER_UTC} UTC]",
                        fontsize=12)
    ax_ratio.set_xlim(-args.before, args.after)

    plt.tight_layout()
    plt.savefig(args.output, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"Saved: {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
