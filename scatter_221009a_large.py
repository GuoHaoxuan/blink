#!/usr/bin/env python3
"""三机箱 1K vs 1B — GRB 221009A (史上最亮GRB)

每张图 4 行:
  Row 0: 光变曲线 (1K黑 vs 1B彩), bin=0.1s (时间跨度大)
  Row 1: 1K 散点 (Time vs Channel)
  Row 2: 1B 散点 (Time vs Channel)
  Row 3: zoom 到主峰区域的 1K/1B 对比

SAT 区间用红色半透明标出。
"""

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec
from matplotlib.patches import Patch
from astropy.io import fits
from astropy.time import Time

burst_utc = Time("2022-10-09T13:16:59.990", scale="utc")
t0_ref = Time("2012-01-01T00:00:00", scale="utc")
burst_met = (burst_utc - t0_ref).sec

DUMP_FILE = "dump_221009a_events.txt"
SAT_FILE = "sat_221009a.txt"
FITS_1K = "data/1K/Y202210/20221009-1943/HXMT_20221009T13_HE-Evt_FFFFFF_V1_1K.FITS"

VIEW = (-50, 600)
ZOOM = (200, 350)

box_cfg = {
    "A": {"det": (0, 5), "c1b": "#1f77b4"},
    "B": {"det": (6, 11), "c1b": "#ff7f0e"},
    "C": {"det": (12, 17), "c1b": "#2ca02c"},
}

SAT_COLOR = "#e74c3c"
SAT_ALPHA = 0.12

# ── SAT ──
sat = {b: [] for b in box_cfg}
with open(SAT_FILE) as f:
    for line in f:
        if line.startswith("SAT,"):
            p = line.strip().split(",")
            sat[p[1]].append((float(p[2]) - burst_met, float(p[3]) - burst_met))

# ── 1B events ──
print("读取 1B ...")
ev1b = {b: {"t": [], "ch": []} for b in box_cfg}
with open(DUMP_FILE) as f:
    for line in f:
        if line.startswith("#") or not line.strip():
            continue
        p = line.split(",")
        if len(p) >= 6 and p[3] == "EVT" and p[0] in ev1b:
            ev1b[p[0]]["t"].append(float(p[5]))
            ev1b[p[0]]["ch"].append(int(p[4]))
for b in box_cfg:
    ev1b[b]["t"] = np.array(ev1b[b]["t"])
    ev1b[b]["ch"] = np.array(ev1b[b]["ch"])
    print(f"  1B Box {b}: {len(ev1b[b]['t']):,}")

# ── 1K ──
print("读取 1K ...")
with fits.open(FITS_1K) as hdul:
    d = hdul[1].data
    m = (d["Time"] >= burst_met + VIEW[0]) & (d["Time"] <= burst_met + VIEW[1])
    time_all, det_all, ch_all = d["Time"][m], d["Det_ID"][m], d["Channel"][m]

ev1k = {}
for b, cfg in box_cfg.items():
    lo, hi = cfg["det"]
    m = (det_all >= lo) & (det_all <= hi)
    ev1k[b] = {"t": time_all[m], "ch": ch_all[m]}
    print(f"  1K Box {b}: {len(ev1k[b]['t']):,}")


def draw_sat(ax, bname, xlim):
    for s, e in sat[bname]:
        if e > xlim[0] and s < xlim[1]:
            ax.axvspan(
                max(s, xlim[0]),
                min(e, xlim[1]),
                color=SAT_COLOR,
                alpha=SAT_ALPHA,
                zorder=0,
            )


# ── 逐 Box 画大图 ──
for bname in ["A", "B", "C"]:
    print(f"\n画 Box {bname} ...")
    c1b = box_cfg[bname]["c1b"]

    t1k = ev1k[bname]["t"] - burst_met
    c1k = ev1k[bname]["ch"]
    t1b = ev1b[bname]["t"] - burst_met
    c1b_ch = ev1b[bname]["ch"]
    n1k, n1b = len(t1k), len(t1b)

    fig = plt.figure(figsize=(30, 22))
    gs = gridspec.GridSpec(4, 1, height_ratios=[1.2, 2, 2, 2.5], hspace=0.25)

    # ── Row 0: 光变 (bin=0.1s for long GRB) ──
    ax0 = fig.add_subplot(gs[0])
    draw_sat(ax0, bname, VIEW)
    bins_lc = np.arange(VIEW[0], VIEW[1] + 0.1, 0.1)
    ax0.hist(
        t1k,
        bins=bins_lc,
        histtype="step",
        color="black",
        lw=0.8,
        label=f"1K  n={n1k:,}",
    )
    ax0.hist(
        t1b,
        bins=bins_lc,
        histtype="step",
        color=c1b,
        lw=0.8,
        alpha=0.85,
        label=f"1B  n={n1b:,}",
    )
    ax0.set_xlim(*VIEW)
    ax0.set_ylabel("Counts / 0.1s", fontsize=12)
    ax0.set_title(
        f"Box {bname} — Light Curve  (1K vs 1B)", fontsize=15, fontweight="bold"
    )
    handles, labels = ax0.get_legend_handles_labels()
    handles.append(Patch(facecolor=SAT_COLOR, alpha=0.35, label="SAT interval"))
    ax0.legend(handles=handles, fontsize=11, loc="upper right")
    ax0.grid(alpha=0.15)

    # ── Row 1: 1K scatter ──
    ax1 = fig.add_subplot(gs[1], sharex=ax0)
    draw_sat(ax1, bname, VIEW)
    ax1.scatter(t1k, c1k, s=0.1, c="black", alpha=0.05, rasterized=True)
    ax1.set_ylim(-5, 260)
    ax1.set_ylabel("Channel", fontsize=12)
    ax1.set_title(
        f"1K Events  (n={n1k:,})", fontsize=13, fontweight="bold", color="black"
    )
    ax1.grid(alpha=0.1)

    # ── Row 2: 1B scatter ──
    ax2 = fig.add_subplot(gs[2], sharex=ax0)
    draw_sat(ax2, bname, VIEW)
    ax2.scatter(t1b, c1b_ch, s=0.1, c=c1b, alpha=0.05, rasterized=True)
    ax2.set_ylim(-5, 260)
    ax2.set_ylabel("Channel", fontsize=12)
    ax2.set_title(
        f"1B Events  (n={n1b:,},  \u0394={n1b - n1k:+,} = {(n1b / max(n1k, 1) - 1) * 100:+.1f}%)",
        fontsize=13,
        fontweight="bold",
        color=c1b,
    )
    ax2.set_xlabel("Time \u2212 T\u2080 (s)", fontsize=12)
    ax2.grid(alpha=0.1)

    # ── Row 3: zoom 对比 ──
    ax3 = fig.add_subplot(gs[3])
    draw_sat(ax3, bname, ZOOM)

    m1k_z = (t1k >= ZOOM[0]) & (t1k <= ZOOM[1])
    m1b_z = (t1b >= ZOOM[0]) & (t1b <= ZOOM[1])
    nz1k, nz1b = int(m1k_z.sum()), int(m1b_z.sum())

    ax3.scatter(
        t1k[m1k_z],
        c1k[m1k_z],
        s=0.5,
        c="black",
        alpha=0.15,
        label=f"1K ({nz1k:,})",
        rasterized=True,
    )
    ax3.scatter(
        t1b[m1b_z],
        c1b_ch[m1b_z],
        s=0.5,
        c=c1b,
        alpha=0.10,
        label=f"1B ({nz1b:,})",
        rasterized=True,
    )
    ax3.set_xlim(*ZOOM)
    ax3.set_ylim(-5, 260)
    ax3.set_ylabel("Channel", fontsize=12)
    ax3.set_xlabel("Time \u2212 T\u2080 (s)", fontsize=12)
    ax3.set_title(
        f"Zoom T+{ZOOM[0]}\u2013{ZOOM[1]}s — 1K (black) vs 1B ({bname} color) overlay",
        fontsize=13,
        fontweight="bold",
    )
    ax3.legend(fontsize=11, loc="upper right", markerscale=8)
    ax3.grid(alpha=0.1)

    fig.suptitle(
        f"GRB 221009A  Box {bname} \u2014 Event Scatter\n"
        f"T\u2080 = {burst_utc.iso}   MET = {burst_met:.3f}   "
        f"Red shading = SAT (saturation gap, 1B lost anchor)",
        fontsize=15,
        fontweight="bold",
        y=0.995,
    )

    outfile = f"scatter_221009a_box{bname}_large.png"
    fig.savefig(outfile, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  Saved: {outfile}")

print("\nDone.")
