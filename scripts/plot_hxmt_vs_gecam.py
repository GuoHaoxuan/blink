#!/usr/bin/env python3
"""Plot HXMT/HE reconstructed light curve vs GECAM-C for GRB 221009A.

Usage:
    python3 scripts/plot_hxmt_vs_gecam.py --bin 1.0
    python3 scripts/plot_hxmt_vs_gecam.py --bin 0.5 --before 50 --after 700
"""

import argparse, subprocess, os, sys
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from astropy.io import fits
from astropy.time import Time
import astropy.units as u

# ── HXMT config ──
HXMT_TRIGGER_UTC = "2022-10-09T13:17:02"
HXMT_EPOCH = "2022-10-09T13"

# ── GECAM-C config ──
GECAM_EVT = "data/gecam_c/gcg_evt_221009_13_v09.fits"
GECAM_MJDREFI = 59215
GECAM_MJDREFF = 0.00080074074


def compute_time_offset():
    """Compute offset: GECAM T=0 relative to HXMT T=0, both in UTC."""
    # HXMT trigger in UTC (using astropy for leap second correctness)
    hxmt_trigger = Time(HXMT_TRIGGER_UTC, scale='utc')

    # GECAM epoch: TIMESYS=TT, so MJDREFI+MJDREFF is in TT scale
    # MJDREFF=0.00080074074 days = 69.184s = TT-UTC offset, so epoch = MJD 59215.0 UTC
    gecam_epoch = Time(GECAM_MJDREFI + GECAM_MJDREFF, format='mjd', scale='tt')
    # GRB trigger MET in GECAM system (TT seconds since TT epoch)
    gecam_trigger_met = (hxmt_trigger.tt - gecam_epoch).sec

    # HXMT MET system: epoch = 2012-01-01 UTC, events in SI seconds
    hxmt_epoch = Time('2012-01-01T00:00:00', scale='utc')
    hxmt_trigger_met = (hxmt_trigger - hxmt_epoch).sec

    # Python datetime gives wrong MET (ignores leap seconds)
    from datetime import datetime, timezone
    hxmt_trigger_met_python = (
        datetime(2022, 10, 9, 13, 17, 2, tzinfo=timezone.utc) -
        datetime(2012, 1, 1, tzinfo=timezone.utc)
    ).total_seconds()

    # Offset: how many seconds Python MET is wrong by
    hxmt_met_correction = hxmt_trigger_met_python - hxmt_trigger_met

    print(f"  HXMT trigger UTC: {hxmt_trigger.iso}", file=sys.stderr)
    print(f"  HXMT trigger MET (astropy): {hxmt_trigger_met:.3f}", file=sys.stderr)
    print(f"  HXMT trigger MET (python):  {hxmt_trigger_met_python:.3f} "
          f"(off by {hxmt_met_correction:.3f}s due to leap seconds)", file=sys.stderr)
    print(f"  GECAM epoch: {gecam_epoch.iso}", file=sys.stderr)
    print(f"  GECAM trigger MET: {gecam_trigger_met:.3f}", file=sys.stderr)

    return gecam_trigger_met, hxmt_met_correction


def load_hxmt_reconstruct(before, after):
    """Load HXMT 1B reconstructed events."""
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

    # HXMT events have MET from Python datetime (wrong by leap seconds)
    # We'll correct when plotting
    obs, fill = [], []
    for line in proc.stdout.strip().split("\n"):
        p = line.split(",")
        if len(p) < 3 or p[0] == "box":
            continue
        met = float(p[2])
        if p[1] == "EVT":
            obs.append(met)
        elif p[1] == "FILL_GAP":
            fill.append(met)
    return np.array(obs), np.array(fill)


def load_gecam_c(gecam_trigger_met, before, after):
    """Load GECAM-C low-gain events."""
    print(f"  Loading GECAM-C: {GECAM_EVT}", file=sys.stderr)
    f = fits.open(GECAM_EVT, memmap=True)

    all_times = []
    for i in range(1, 11):
        hdu_name = f'EVENTS{i:02d}'
        try:
            ev = f[hdu_name].data
            mask = ((ev['GAIN_TYPE'] == 1) &
                    (ev['TIME'] >= gecam_trigger_met - before) &
                    (ev['TIME'] <= gecam_trigger_met + after))
            times = ev['TIME'][mask] - gecam_trigger_met
            all_times.append(times)
            print(f"    {hdu_name}: {mask.sum():,} low-gain events", file=sys.stderr)
        except Exception:
            break

    f.close()
    combined = np.concatenate(all_times)
    print(f"  GECAM-C total: {len(combined):,} events", file=sys.stderr)
    return combined


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", type=float, default=1.0)
    parser.add_argument("--before", type=float, default=50.0)
    parser.add_argument("--after", type=float, default=700.0)
    parser.add_argument("--bkg", type=float, nargs=4, default=[-50, -10, 500, 700],
                        metavar=("T1", "T2", "T3", "T4"))
    parser.add_argument("--xlim", type=float, nargs=2, default=None,
                        metavar=("XMIN", "XMAX"), help="Plot x-axis range (default=full)")
    parser.add_argument("--scale-range", type=float, nargs=2, default=None,
                        metavar=("S1", "S2"), help="Time range for scaling (default=burst region)")
    parser.add_argument("--cache", type=str, default=None,
                        help="Read HXMT reconstruct from cached CSV instead of re-running CLI")
    parser.add_argument("-o", "--output", default="hxmt_vs_gecam.png")
    args = parser.parse_args()

    # Time alignment
    print("Computing time alignment...", file=sys.stderr)
    gecam_trigger_met, hxmt_met_correction = compute_time_offset()

    # HXMT trigger MET as computed by Python datetime (what the CLI uses)
    from datetime import datetime, timezone
    hxmt_trigger_met_python = (
        datetime(2022, 10, 9, 13, 17, 2, tzinfo=timezone.utc) -
        datetime(2012, 1, 1, tzinfo=timezone.utc)
    ).total_seconds()

    # Load HXMT
    if args.cache:
        print(f"Loading HXMT/HE from cache: {args.cache}", file=sys.stderr)
        hxmt_obs, hxmt_fill = [], []
        with open(args.cache) as cf:
            for line in cf:
                p = line.strip().split(",")
                if len(p) < 3 or p[0] == "box":
                    continue
                met = float(p[2])
                if p[1] == "EVT":
                    hxmt_obs.append(met)
                elif p[1] == "FILL_GAP":
                    hxmt_fill.append(met)
        hxmt_obs, hxmt_fill = np.array(hxmt_obs), np.array(hxmt_fill)
    else:
        print("Loading HXMT/HE reconstruct...", file=sys.stderr)
        hxmt_obs, hxmt_fill = load_hxmt_reconstruct(args.before, args.after)
    # Convert HXMT MET to time relative to true trigger (correct for leap seconds)
    hxmt_obs_t = hxmt_obs - hxmt_trigger_met_python + hxmt_met_correction
    hxmt_fill_t = hxmt_fill - hxmt_trigger_met_python + hxmt_met_correction if len(hxmt_fill) > 0 else np.array([])
    hxmt_all_t = np.concatenate([hxmt_obs_t, hxmt_fill_t]) if len(hxmt_fill_t) > 0 else hxmt_obs_t
    print(f"  HXMT: {len(hxmt_obs):,} obs + {len(hxmt_fill):,} fill", file=sys.stderr)

    # Load GECAM-C (already relative to trigger)
    gecam_t = load_gecam_c(gecam_trigger_met, args.before, args.after)

    # ── Binning & background subtraction ──
    bin_w = args.bin
    edges = np.arange(-args.before, args.after + bin_w, bin_w)
    x = edges[:-1]
    t1, t2, t3, t4 = args.bkg

    r_hxmt_obs = np.histogram(hxmt_obs_t, bins=edges)[0] / bin_w
    r_hxmt_all = np.histogram(hxmt_all_t, bins=edges)[0] / bin_w
    r_gecam = np.histogram(gecam_t, bins=edges)[0] / bin_w

    bkg_mask = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
    n_bkg = bkg_mask.sum()
    bkg_hxmt = np.mean(r_hxmt_all[bkg_mask]) if n_bkg > 0 else 0
    bkg_gecam = np.mean(r_gecam[bkg_mask]) if n_bkg > 0 else 0
    print(f"  Background: HXMT={bkg_hxmt:.0f}, GECAM={bkg_gecam:.0f} evt/s", file=sys.stderr)

    net_hxmt_obs = r_hxmt_obs - bkg_hxmt
    net_hxmt_all = r_hxmt_all - bkg_hxmt
    net_gecam = r_gecam - bkg_gecam

    # Scale GECAM to HXMT
    if args.scale_range:
        s1, s2 = args.scale_range
    else:
        s1, s2 = t2, t3
    scale_mask = (x >= s1) & (x < s2)
    sum_hxmt = np.sum(net_hxmt_all[scale_mask])
    sum_gecam = np.sum(net_gecam[scale_mask])
    scale = sum_hxmt / sum_gecam if sum_gecam > 0 else 1.0
    net_gecam_scaled = net_gecam * scale
    print(f"  Scale factor: {scale:.2f}", file=sys.stderr)

    # ── Plot ──
    fig, (ax_lc, ax_ratio) = plt.subplots(
        2, 1, figsize=(16, 8), sharex=True,
        gridspec_kw={"height_ratios": [3, 1], "hspace": 0.06})

    C0, C1, C2 = plt.cm.tab10(0), plt.cm.tab10(1), plt.cm.tab10(2)

    # HXMT observed: // C0
    ax_lc.fill_between(x, np.maximum(net_hxmt_obs, 0), step="post",
                       facecolor="none", hatch="//", edgecolor=C0, alpha=0.5,
                       linewidth=0, zorder=2, label="HXMT/HE observed")
    ax_lc.step(x, net_hxmt_obs, where="post", color=C0, lw=1.0, zorder=3)

    # HXMT filled part: // C1
    ax_lc.fill_between(x, np.maximum(net_hxmt_obs, 0), np.maximum(net_hxmt_all, 0),
                       step="post",
                       facecolor="none", hatch="//", edgecolor=C1, alpha=0.5,
                       linewidth=0, zorder=4,
                       label=f"HXMT/HE filled (+{len(hxmt_fill):,})")
    ax_lc.step(x, net_hxmt_all, where="post", color=C1, lw=1.0, zorder=5)

    # GECAM-C: \\ C2
    ax_lc.fill_between(x, np.maximum(net_gecam_scaled, 0), step="post",
                       facecolor="none", hatch="\\\\", edgecolor=C2, alpha=0.5,
                       linewidth=0, zorder=6,
                       label=f"GECAM-C low-gain (×{scale:.1f})")
    ax_lc.step(x, net_gecam_scaled, where="post", color=C2, lw=1.0, zorder=7)

    ax_lc.set_ylabel("Net count rate (evt/s)", fontsize=13)
    ax_lc.legend(loc="upper right", fontsize=10)
    ax_lc.set_ylim(bottom=0)
    ax_lc.grid(alpha=0.15)
    ax_lc.set_title(f"GRB 221009A: HXMT/HE (reconstructed) vs GECAM-C  "
                     f"[{bin_w}s bins, bkg subtracted]",
                     fontsize=13, fontweight="bold")

    # Ratio panel
    with np.errstate(divide='ignore', invalid='ignore'):
        peak = np.nanmax(net_gecam_scaled)
        threshold = max(peak * 0.05, 100)
        ratio = np.where(net_gecam_scaled > threshold, net_hxmt_all / net_gecam_scaled, np.nan)
    ax_ratio.step(x, ratio, where="post", color="#333333", lw=0.8, zorder=2)
    ax_ratio.axhline(1.0, color="black", lw=0.5, ls="--", alpha=0.5)
    ax_ratio.set_ylabel("HXMT / GECAM", fontsize=13)
    ax_ratio.set_ylim(0.5, 1.5)
    ax_ratio.grid(alpha=0.15)
    ax_ratio.set_xlabel(f"Time since trigger (s)  [T₀ = {HXMT_TRIGGER_UTC} UTC]", fontsize=12)
    if args.xlim:
        ax_ratio.set_xlim(args.xlim[0], args.xlim[1])
    else:
        ax_ratio.set_xlim(-args.before, args.after)

    plt.tight_layout()
    plt.savefig(args.output, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"Saved: {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
