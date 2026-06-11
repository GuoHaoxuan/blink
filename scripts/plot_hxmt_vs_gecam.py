#!/usr/bin/env python3
"""Plot HXMT/HE reconstructed light curve vs GECAM-C for GRB 221009A.

Usage:
    python3 scripts/plot_hxmt_vs_gecam.py --bin 1.0
    python3 scripts/plot_hxmt_vs_gecam.py --bin 0.5 --before 50 --after 700
    python3 scripts/plot_hxmt_vs_gecam.py --btime          # use BTIME + revisited orbit bkg
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
GECAM_BTIME = "data/gecam_c/gcg_btime_221009_13_v12.fits"
GECAM_BTIME_BKG = "data/gecam_c/gcg_btime_221014_13_v06.fits"  # revisited orbit (+5 days)
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


def load_gecam_btime(gecam_trigger_met, before, after, bin_w, channels="lg",
                     subtract_revisited=True):
    """Load GECAM-C BTIME data (GRD01).

    During GRB 221009A, GECAM-C was in a high-latitude particle region and
    only GRD01 was active (An et al. 2023, arXiv:2303.01203).

    channels: "hg" (ch0-5, 6-350 keV), "lg" (ch7-12, 0.4-6 MeV),
              "all" (ch0-12, exclude overflow ch6,13)
    subtract_revisited: if True, subtract revisited-orbit background.
                        if False, return raw rates (caller handles bkg).
    Returns (left_edges, rate, ch_label) — 1-D arrays rebinned to *bin_w* seconds.
    """
    ch_slices = {
        "hg": (slice(0, 6), "6-350 keV"),
        "lg": (slice(7, 13), "0.4-6 MeV"),
        "all": (None, "6 keV-6 MeV"),
    }
    ch_slice, ch_label = ch_slices[channels]

    print(f"  Loading GECAM-C BTIME: {GECAM_BTIME}  [{channels}: {ch_label}]",
          file=sys.stderr)
    f_burst = fits.open(GECAM_BTIME)

    dt = 0.05  # native 50 ms bins
    sp_burst = f_burst['SPECTRUM01'].data
    t_burst = sp_burst['STARTTIME']

    if ch_slice is not None:
        counts_burst = sp_burst['COUNTS'][:, ch_slice].sum(axis=1).astype(float)
    else:
        counts_burst = (sp_burst['COUNTS'][:, 0:6].sum(axis=1) +
                        sp_burst['COUNTS'][:, 7:13].sum(axis=1)).astype(float)

    if subtract_revisited:
        f_bkg = fits.open(GECAM_BTIME_BKG)
        sp_bkg = f_bkg['SPECTRUM01'].data
        if ch_slice is not None:
            counts_bkg_raw = sp_bkg['COUNTS'][:, ch_slice].sum(axis=1).astype(float)
        else:
            counts_bkg_raw = (sp_bkg['COUNTS'][:, 0:6].sum(axis=1) +
                              sp_bkg['COUNTS'][:, 7:13].sum(axis=1)).astype(float)
        orbit_offset = f_bkg[0].header['TSTART'] - f_burst[0].header['TSTART']
        t_bkg_aligned = sp_bkg['STARTTIME'] - orbit_offset
        counts_bkg = np.interp(t_burst, t_bkg_aligned, counts_bkg_raw,
                               left=np.nan, right=np.nan)
        rate = (counts_burst - counts_bkg) / dt
        f_bkg.close()
        print(f"    Orbit offset: {orbit_offset:.0f}s ({orbit_offset/86400:.1f} days)",
              file=sys.stderr)
    else:
        rate = counts_burst / dt

    t_rel = t_burst - gecam_trigger_met

    # Rebin to requested bin width
    edges = np.arange(-before, after + bin_w, bin_w)
    left = edges[:-1]
    sum_rate, _ = np.histogram(t_rel, bins=edges, weights=rate)
    n_bins, _ = np.histogram(t_rel, bins=edges)
    good = n_bins > 0
    rebinned = np.full(len(left), np.nan)
    rebinned[good] = sum_rate[good] / n_bins[good]

    print(f"    Channels: {channels} ({ch_label})", file=sys.stderr)
    print(f"    Peak: {np.nanmax(rebinned):.0f} cts/s "
          f"at T{left[np.nanargmax(rebinned)]:+.1f}s", file=sys.stderr)

    f_burst.close()
    return left, rebinned, ch_label


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", type=float, default=1.0)
    parser.add_argument("--before", type=float, default=50.0)
    parser.add_argument("--after", type=float, default=700.0)
    parser.add_argument("--bkg", type=float, nargs=4, default=[-50, -10, 500, 700],
                        metavar=("T1", "T2", "T3", "T4"))
    parser.add_argument("--xlim", type=float, nargs=2, default=None,
                        metavar=("XMIN", "XMAX"), help="Plot x-axis range (default=full)")
    parser.add_argument("--scale-range", type=float, nargs="+", default=None,
                        metavar="T", help="Time ranges for scaling (pairs: S1 S2 [S3 S4 ...])")
    parser.add_argument("--cache", type=str, default=None,
                        help="Read HXMT reconstruct from cached CSV instead of re-running CLI")
    parser.add_argument("--btime", action="store_true",
                        help="Use BTIME (binned) data instead of events")
    parser.add_argument("--raw", action="store_true",
                        help="With --btime: skip revisited-orbit subtraction, use flat bkg from --bkg")
    parser.add_argument("--channels", choices=["hg", "lg", "all"], default="lg",
                        help="BTIME channel selection: hg (6-350 keV), lg (0.4-6 MeV), all")
    parser.add_argument("--ylim", type=float, default=None,
                        help="Upper y-axis limit for light curve panel")
    parser.add_argument("--mask", type=float, nargs="+", default=None,
                        metavar="T", help="Shutdown intervals to exclude (pairs: T_start T_end ...)")
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

    # ── Binning ──
    bin_w = args.bin
    edges = np.arange(-args.before, args.after + bin_w, bin_w)
    x = edges[:-1]
    t1, t2, t3, t4 = args.bkg

    # Load GECAM-C
    if args.btime:
        use_revisited = not args.raw
        mode_str = "revisited-orbit bkg" if use_revisited else "raw (flat bkg)"
        print(f"Loading GECAM-C BTIME ({mode_str})...", file=sys.stderr)
        gecam_x, gecam_rate, ch_label = load_gecam_btime(
            gecam_trigger_met, args.before, args.after, bin_w, args.channels,
            subtract_revisited=use_revisited)
        x = gecam_x
        edges = np.append(x, x[-1] + bin_w)
        if args.raw:
            bkg_mask_g = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
            bkg_gecam = np.nanmean(gecam_rate[bkg_mask_g]) if bkg_mask_g.sum() > 0 else 0
            net_gecam = gecam_rate - bkg_gecam
            print(f"  GECAM background (flat): {bkg_gecam:.0f} cts/s", file=sys.stderr)
        else:
            net_gecam = gecam_rate
        gecam_label = f"GECAM-C GRD01 {args.channels.upper()}"
    else:
        gecam_t = load_gecam_c(gecam_trigger_met, args.before, args.after)
        r_gecam = np.histogram(gecam_t, bins=edges)[0] / bin_w
        bkg_mask_g = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
        bkg_gecam = np.mean(r_gecam[bkg_mask_g]) if bkg_mask_g.sum() > 0 else 0
        net_gecam = r_gecam - bkg_gecam
        gecam_label = "GECAM-C low-gain (evt)"
        print(f"  GECAM background: {bkg_gecam:.0f} evt/s", file=sys.stderr)

    # HXMT binning & background
    r_hxmt_obs = np.histogram(hxmt_obs_t, bins=edges)[0] / bin_w
    r_hxmt_all = np.histogram(hxmt_all_t, bins=edges)[0] / bin_w
    bkg_mask = ((x >= t1) & (x < t2)) | ((x >= t3) & (x < t4))
    n_bkg = bkg_mask.sum()
    bkg_hxmt = np.mean(r_hxmt_all[bkg_mask]) if n_bkg > 0 else 0
    print(f"  HXMT background: {bkg_hxmt:.0f} evt/s", file=sys.stderr)

    net_hxmt_obs = r_hxmt_obs - bkg_hxmt
    net_hxmt_all = r_hxmt_all - bkg_hxmt

    # Mask shutdown intervals
    if args.mask:
        pairs = list(zip(args.mask[::2], args.mask[1::2]))
        for m_start, m_end in pairs:
            shutdown = (x >= m_start) & (x < m_end)
            net_hxmt_obs[shutdown] = np.nan
            net_hxmt_all[shutdown] = np.nan
            print(f"  Masked shutdown: T+{m_start:.0f} to T+{m_end:.0f} "
                  f"({shutdown.sum()} bins)", file=sys.stderr)

    # Scale GECAM to HXMT
    if args.scale_range:
        pairs = list(zip(args.scale_range[::2], args.scale_range[1::2]))
        scale_mask = np.zeros(len(x), dtype=bool)
        for s1, s2 in pairs:
            scale_mask |= (x >= s1) & (x < s2)
        scale_mask &= np.isfinite(net_gecam) & np.isfinite(net_hxmt_all)
    else:
        scale_mask = (x >= t2) & (x < t3) & np.isfinite(net_gecam) & np.isfinite(net_hxmt_all)
    sum_hxmt = np.nansum(net_hxmt_all[scale_mask])
    sum_gecam = np.nansum(net_gecam[scale_mask])
    scale = sum_hxmt / sum_gecam if sum_gecam > 0 else 1.0
    net_gecam_scaled = net_gecam * scale
    print(f"  Scale factor: {scale:.2f}", file=sys.stderr)

    # ── Plot ──
    fig, (ax_lc, ax_ratio) = plt.subplots(
        2, 1, figsize=(12, 7), sharex=True,
        gridspec_kw={"height_ratios": [3, 1], "hspace": 0.05})

    # HXMT observed (blue)
    ax_lc.step(x, net_hxmt_obs, where="post", color="C0", lw=0.8,
               label="HXMT/HE observed", zorder=3)
    ax_lc.fill_between(x, 0, net_hxmt_obs, step="post", alpha=0.15, color="C0")
    # HXMT observed + filled (orange)
    ax_lc.step(x, net_hxmt_all, where="post", color="C1", lw=0.8,
               label=f"HXMT/HE + reconstructed (+{len(hxmt_fill):,})", zorder=2)
    ax_lc.fill_between(x, net_hxmt_obs, net_hxmt_all, step="post", alpha=0.3, color="C1")
    # GECAM-C (green)
    ax_lc.step(x, np.nan_to_num(net_gecam_scaled), where="post", color="C2", lw=1.0,
               label=f"{gecam_label} (\u00d7{scale:.1f})", zorder=4)

    ax_lc.set_ylabel("Net count rate (evt/s)")
    ax_lc.legend(loc="upper right")
    ax_lc.axhline(0, color="gray", lw=0.5, ls="--")
    if args.ylim:
        ax_lc.set_ylim(-args.ylim * 0.05, args.ylim)
    ax_lc.set_title(f"GRB 221009A: HXMT/HE vs GECAM-C  [{bin_w}s bins, geocentric]",
                    fontweight="bold")

    # Ratio panel — require both instruments above threshold
    with np.errstate(divide='ignore', invalid='ignore'):
        peak_gecam = np.nanmax(np.nan_to_num(net_gecam_scaled))
        peak_hxmt = np.nanmax(np.nan_to_num(net_hxmt_all))
        thr_g = max(peak_gecam * 0.05, 100)
        thr_h = max(peak_hxmt * 0.05, 100)
        both_sig = (np.isfinite(net_gecam_scaled) & (net_gecam_scaled > thr_g) &
                    np.isfinite(net_hxmt_all) & (net_hxmt_all > thr_h))
        ratio = np.where(both_sig, net_hxmt_all / net_gecam_scaled, np.nan)
    ax_ratio.step(x, ratio, where="post", color="k", lw=0.8)
    ax_ratio.axhline(1.0, color="gray", lw=0.5, ls="--")
    ax_ratio.set_ylabel("HXMT / GECAM")
    ax_ratio.set_ylim(0.5, 1.5)
    rv = ratio[~np.isnan(ratio)]
    if len(rv) > 0:
        ax_ratio.text(0.98, 0.85, f"ratio = {np.mean(rv):.2f} \u00b1 {np.std(rv):.2f} ({len(rv)} bins)",
                      transform=ax_ratio.transAxes, ha="right", va="top", fontsize=9,
                      bbox=dict(facecolor="white", alpha=0.8))
    ax_ratio.set_xlabel(f"Time since trigger (s)  [$T_0$ = {HXMT_TRIGGER_UTC} UTC]")
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
