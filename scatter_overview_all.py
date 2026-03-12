#!/usr/bin/env python3
"""三个 GRB 的完整 overview — 每 GRB 每 Box 一张大图 (共 9 张)

4 行:
  Row 0: 光变曲线 (1K黑 vs 1B彩), bin=0.1s
  Row 1: 1K 散点 (Time vs Channel)
  Row 2: 1B 散点 (Time vs Channel)
  Row 3: zoom 到主峰区域的 1K/1B overlay
"""

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec
from matplotlib.patches import Patch
from astropy.io import fits
from astropy.time import Time

t0_ref = Time("2012-01-01T00:00:00", scale="utc")

GRBS = {
    "200415A": {
        "burst_utc": "2020-04-15T08:48:05.564",
        "dump_file": "dump_200415a_events.txt",
        "sat_file": "sat_200415a.txt",
        "fits_1k": "data/1K/Y202004/20200415-1036/HXMT_20200415T08_HE-Evt_FFFFFF_V2_1K.FITS",
        "view": (-5, 30),
        "zoom": (-0.5, 5),
        "lc_bin": 0.01,
    },
    "221009A": {
        "burst_utc": "2022-10-09T13:16:59.990",
        "dump_file": "dump_221009a_events.txt",
        "sat_file": "sat_221009a.txt",
        "fits_1k": "data/1K/Y202210/20221009-1943/HXMT_20221009T13_HE-Evt_FFFFFF_V1_1K.FITS",
        "view": (-50, 600),
        "zoom": (200, 350),
        "lc_bin": 0.1,
    },
    "260226A": {
        "burst_utc": "2026-02-26T10:37:55",
        "dump_file": "dump_260226a_events.txt",
        "sat_file": "sat_260226a.txt",
        "fits_1k": "data/1K/Y202602/20260226-3179/HXMT_20260226T10_HE-Evt_FFFFFF_V1_1K.FITS",
        "view": (-5, 80),
        "zoom": (-1, 15),
        "lc_bin": 0.05,
    },
}

box_cfg = {
    "A": {"det": (0, 5), "c1b": "#1f77b4"},
    "B": {"det": (6, 11), "c1b": "#ff7f0e"},
    "C": {"det": (12, 17), "c1b": "#2ca02c"},
}

SAT_COLOR = "#e74c3c"
SAT_ALPHA = 0.12


def draw_sat(ax, sat_intervals, xlim):
    for s, e in sat_intervals:
        if e > xlim[0] and s < xlim[1]:
            ax.axvspan(max(s, xlim[0]), min(e, xlim[1]),
                       color=SAT_COLOR, alpha=SAT_ALPHA, zorder=0)


for grb_name, cfg in GRBS.items():
    print(f"\n{'='*60}")
    print(f"GRB {grb_name}")
    print(f"{'='*60}")

    burst_utc = Time(cfg["burst_utc"], scale="utc")
    burst_met = (burst_utc - t0_ref).sec
    VIEW = cfg["view"]
    ZOOM = cfg["zoom"]
    lc_bin = cfg["lc_bin"]

    # ── SAT ──
    sat = {b: [] for b in box_cfg}
    try:
        with open(cfg["sat_file"]) as f:
            for line in f:
                if line.startswith("SAT,"):
                    p = line.strip().split(",")
                    sat[p[1]].append((float(p[2]) - burst_met, float(p[3]) - burst_met))
    except FileNotFoundError:
        print(f"  WARNING: {cfg['sat_file']} not found, no SAT shading")

    # ── 1B events ──
    print("  读取 1B ...")
    ev1b = {b: {"t": [], "ch": []} for b in box_cfg}
    met_min = burst_met + VIEW[0]
    met_max = burst_met + VIEW[1]
    try:
        with open(cfg["dump_file"]) as f:
            for line in f:
                if line.startswith("#") or not line.strip():
                    continue
                p = line.split(",")
                if len(p) >= 6 and p[3] == "EVT" and p[0] in ev1b:
                    t = float(p[5])
                    if met_min <= t <= met_max:
                        ev1b[p[0]]["t"].append(t)
                        ev1b[p[0]]["ch"].append(int(p[4]))
    except FileNotFoundError:
        print(f"  ERROR: {cfg['dump_file']} not found!")
        continue
    for b in box_cfg:
        ev1b[b]["t"] = np.array(ev1b[b]["t"])
        ev1b[b]["ch"] = np.array(ev1b[b]["ch"])
        print(f"    1B Box {b}: {len(ev1b[b]['t']):,}")

    # ── 1K ──
    print("  读取 1K ...")
    with fits.open(cfg["fits_1k"]) as hdul:
        d = hdul[1].data
        m = (d["Time"] >= met_min) & (d["Time"] <= met_max)
        time_all, det_all, ch_all = d["Time"][m], d["Det_ID"][m], d["Channel"][m]

    ev1k = {}
    for b, bcfg in box_cfg.items():
        lo, hi = bcfg["det"]
        mb = (det_all >= lo) & (det_all <= hi)
        ev1k[b] = {"t": time_all[mb], "ch": ch_all[mb]}
        print(f"    1K Box {b}: {len(ev1k[b]['t']):,}")

    # ── 逐 Box 画图 ──
    for bname in ["A", "B", "C"]:
        print(f"  画 Box {bname} ...")
        c1b = box_cfg[bname]["c1b"]

        t1k = ev1k[bname]["t"] - burst_met
        c1k = ev1k[bname]["ch"]
        t1b = ev1b[bname]["t"] - burst_met
        c1b_ch = ev1b[bname]["ch"]
        n1k, n1b = len(t1k), len(t1b)

        fig = plt.figure(figsize=(30, 22))
        gs = gridspec.GridSpec(4, 1, height_ratios=[1.2, 2, 2, 2.5], hspace=0.25)

        # ── Row 0: 光变 ──
        ax0 = fig.add_subplot(gs[0])
        draw_sat(ax0, sat[bname], VIEW)
        bins_lc = np.arange(VIEW[0], VIEW[1] + lc_bin, lc_bin)
        ax0.hist(t1k, bins=bins_lc, histtype="step", color="black", lw=0.8,
                 label=f"1K  n={n1k:,}")
        ax0.hist(t1b, bins=bins_lc, histtype="step", color=c1b, lw=0.8, alpha=0.85,
                 label=f"1B  n={n1b:,}")
        ax0.set_xlim(*VIEW)
        ax0.set_ylabel(f"Counts / {lc_bin}s", fontsize=12)
        ax0.set_title(f"Box {bname} — Light Curve  (1K vs 1B)", fontsize=15, fontweight="bold")
        handles, labels = ax0.get_legend_handles_labels()
        handles.append(Patch(facecolor=SAT_COLOR, alpha=0.35, label="SAT interval"))
        ax0.legend(handles=handles, fontsize=11, loc="upper right")
        ax0.grid(alpha=0.15)

        # ── Row 1: 1K scatter ──
        ax1 = fig.add_subplot(gs[1], sharex=ax0)
        draw_sat(ax1, sat[bname], VIEW)
        ax1.scatter(t1k, c1k, s=0.1, c="black", alpha=0.05, rasterized=True)
        ax1.set_ylim(-5, 260)
        ax1.set_ylabel("Channel", fontsize=12)
        ax1.set_title(f"1K Events  (n={n1k:,})", fontsize=13, fontweight="bold", color="black")
        ax1.grid(alpha=0.1)

        # ── Row 2: 1B scatter ──
        ax2 = fig.add_subplot(gs[2], sharex=ax0)
        draw_sat(ax2, sat[bname], VIEW)
        ax2.scatter(t1b, c1b_ch, s=0.1, c=c1b, alpha=0.05, rasterized=True)
        ax2.set_ylim(-5, 260)
        ax2.set_ylabel("Channel", fontsize=12)
        delta = n1b - n1k
        pct = (n1b / max(n1k, 1) - 1) * 100
        ax2.set_title(
            f"1B Events  (n={n1b:,},  Δ={delta:+,} = {pct:+.1f}%)",
            fontsize=13, fontweight="bold", color=c1b)
        ax2.set_xlabel("Time − T₀ (s)", fontsize=12)
        ax2.grid(alpha=0.1)

        # ── Row 3: zoom overlay ──
        ax3 = fig.add_subplot(gs[3])
        draw_sat(ax3, sat[bname], ZOOM)

        m1k_z = (t1k >= ZOOM[0]) & (t1k <= ZOOM[1])
        m1b_z = (t1b >= ZOOM[0]) & (t1b <= ZOOM[1])
        nz1k, nz1b = int(m1k_z.sum()), int(m1b_z.sum())

        ax3.scatter(t1k[m1k_z], c1k[m1k_z], s=0.5, c="black", alpha=0.15,
                    label=f"1K ({nz1k:,})", rasterized=True)
        ax3.scatter(t1b[m1b_z], c1b_ch[m1b_z], s=0.5, c=c1b, alpha=0.10,
                    label=f"1B ({nz1b:,})", rasterized=True)
        ax3.set_xlim(*ZOOM)
        ax3.set_ylim(-5, 260)
        ax3.set_ylabel("Channel", fontsize=12)
        ax3.set_xlabel("Time − T₀ (s)", fontsize=12)
        ax3.set_title(
            f"Zoom T+{ZOOM[0]}~{ZOOM[1]}s — 1K (black) vs 1B ({bname} color) overlay",
            fontsize=13, fontweight="bold")
        ax3.legend(fontsize=11, loc="upper right", markerscale=8)
        ax3.grid(alpha=0.1)

        fig.suptitle(
            f"GRB {grb_name}  Box {bname} — Event Scatter\n"
            f"T₀ = {burst_utc.iso}   MET = {burst_met:.3f}   "
            f"Red shading = SAT interval",
            fontsize=15, fontweight="bold", y=0.995)

        outfile = f"scatter_{grb_name}_box{bname}.png"
        fig.savefig(outfile, dpi=150, bbox_inches="tight")
        plt.close(fig)
        print(f"    Saved: {outfile}")

print("\nDone.")
