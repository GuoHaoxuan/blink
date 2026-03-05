#!/usr/bin/env python3
"""三机箱 1K vs 1B 逐事例散点图

每个 Box 分开绘制:
  - 左列: 2ms 光变曲线 (1K vs 1B) + SAT 区间
  - 中列: 散点图 Time vs Channel (全局) + SAT 着色
  - 右列: 散点图 zoom 到饱和峰区

用法:
    # 1. 生成 1B 事例 dump 和 SAT 区间
    HXMT_1B_DIR=data/1B target/release/blink_cli 2026-02-26T10 \
        --dump-events 446726298 25 > dump_full_burst_events.txt
    HXMT_1B_DIR=data/1B target/release/blink_cli 2026-02-26T10 \
        --dump-times 446726298 25 2>/dev/null | grep '^SAT,' > sat_intervals.txt

    # 2. 画图
    python3 scatter_per_box.py
"""

import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from astropy.io import fits
from astropy.time import Time

# ═══════════════ 参数 ═══════════════
burst_utc = Time("2026-02-26T10:37:55", scale="utc")
t0_ref = Time("2012-01-01T00:00:00", scale="utc")
burst_met = (burst_utc - t0_ref).sec
print(f"Burst MET = {burst_met:.3f}")

DUMP_FILE = "dump_full_burst_events.txt"
SAT_FILE = "sat_intervals.txt"
FITS_1K = "data/1K/Y202602/20260226-3179/HXMT_20260226T10_HE-Evt_FFFFFF_V1_1K.FITS"

# 显示范围（相对 burst）
VIEW_FULL = (-5, 45)
VIEW_ZOOM = (14, 22)  # 饱和峰区 zoom

box_cfg = {
    "A": {"det": (0, 5), "c1b": "#1f77b4", "csat": "#1f77b4"},
    "B": {"det": (6, 11), "c1b": "#ff7f0e", "csat": "#ff7f0e"},
    "C": {"det": (12, 17), "c1b": "#2ca02c", "csat": "#2ca02c"},
}

# ═══════════════ 1. 读 SAT 区间 ═══════════════
print("读取 SAT 区间 ...")
sat = {b: [] for b in box_cfg}
try:
    with open(SAT_FILE) as f:
        for line in f:
            if line.startswith("SAT,"):
                p = line.strip().split(",")
                box = p[1]
                if box in sat:
                    sat[box].append((float(p[2]) - burst_met, float(p[3]) - burst_met))
    for b in box_cfg:
        print(f"  Box {b}: {len(sat[b])} SAT intervals")
except FileNotFoundError:
    print("  (SAT file not found — skipping overlay)")

# ═══════════════ 2. 读 1B dump ═══════════════
print("读取 1B dump ...")
ev1b = {b: {"t": [], "ch": []} for b in box_cfg}

with open(DUMP_FILE) as f:
    for line in f:
        if line.startswith("#") or not line.strip():
            continue
        p = line.split(",")
        if len(p) >= 6 and p[3] == "EVT":
            box = p[0]
            if box in ev1b:
                ev1b[box]["t"].append(float(p[5]))
                ev1b[box]["ch"].append(int(p[4]))

for b in box_cfg:
    ev1b[b]["t"] = np.array(ev1b[b]["t"])
    ev1b[b]["ch"] = np.array(ev1b[b]["ch"])
    print(f"  1B Box {b}: {len(ev1b[b]['t']):,} events")

# ═══════════════ 3. 读 1K FITS ═══════════════
print("读取 1K FITS ...")
met_lo = burst_met + VIEW_FULL[0]
met_hi = burst_met + VIEW_FULL[1]

with fits.open(FITS_1K) as hdul:
    d = hdul[1].data
    m = (d["Time"] >= met_lo) & (d["Time"] <= met_hi)
    time_all = d["Time"][m]
    det_all = d["Det_ID"][m]
    ch_all = d["Channel"][m]

print(f"  1K in window: {len(time_all):,} events")

ev1k = {}
for b, cfg in box_cfg.items():
    lo, hi = cfg["det"]
    m = (det_all >= lo) & (det_all <= hi)
    ev1k[b] = {"t": time_all[m], "ch": ch_all[m]}
    print(f"  1K Box {b}: {len(ev1k[b]['t']):,} events")


# ═══════════════ helper: SAT 着色 ═══════════════
def draw_sat(ax, bname, xlim):
    """在 ax 上画半透明 SAT 区间"""
    color = box_cfg[bname]["csat"]
    for s, e in sat[bname]:
        if e > xlim[0] and s < xlim[1]:
            ax.axvspan(
                max(s, xlim[0]),
                min(e, xlim[1]),
                color=color,
                alpha=0.08,
                zorder=0,
            )


# ═══════════════ 4. 画图 ═══════════════
fig, axes = plt.subplots(
    3,
    3,
    figsize=(30, 18),
    gridspec_kw={"width_ratios": [2, 3, 2]},
)

for row, bname in enumerate(["A", "B", "C"]):
    c1b = box_cfg[bname]["c1b"]

    # 转换成相对 burst 的时间
    t1k = ev1k[bname]["t"] - burst_met
    c1k = ev1k[bname]["ch"]
    t1b = ev1b[bname]["t"] - burst_met
    c1b_arr = ev1b[bname]["ch"]

    n1k = len(t1k)
    n1b = len(t1b)
    diff = n1b - n1k
    pct = (n1b / max(n1k, 1) - 1) * 100

    # ─── 左列: 2ms 光变曲线 ───
    ax_lc = axes[row, 0]
    draw_sat(ax_lc, bname, VIEW_FULL)

    bins_lc = np.arange(VIEW_FULL[0], VIEW_FULL[1] + 0.002, 0.002)
    ax_lc.hist(
        t1k,
        bins=bins_lc,
        histtype="step",
        color="black",
        lw=0.6,
        alpha=0.85,
        label=f"1K ({n1k:,})",
    )
    ax_lc.hist(
        t1b,
        bins=bins_lc,
        histtype="step",
        color=c1b,
        lw=0.6,
        alpha=0.75,
        label=f"1B ({n1b:,})",
    )

    ax_lc.text(
        0.02,
        0.95,
        f"\u0394 = {diff:+,} ({pct:+.1f}%)",
        transform=ax_lc.transAxes,
        fontsize=9,
        va="top",
        bbox=dict(boxstyle="round,pad=0.3", fc="white", alpha=0.8),
    )

    ax_lc.set_xlim(*VIEW_FULL)
    ax_lc.set_ylabel("Counts / 2ms", fontsize=10)
    ax_lc.set_title(
        f"Box {bname} \u2014 Light Curve (shaded = SAT)", fontsize=12, fontweight="bold"
    )
    ax_lc.legend(loc="upper right", fontsize=8)
    ax_lc.grid(alpha=0.2)
    ax_lc.axvline(0, color="gray", ls=":", alpha=0.4)
    if row == 2:
        ax_lc.set_xlabel("Time \u2212 T\u2080 (s)", fontsize=10)

    # ─── 中列: 散点 全景 ───
    ax_sc = axes[row, 1]
    draw_sat(ax_sc, bname, VIEW_FULL)

    ax_sc.scatter(t1k, c1k, s=0.15, c="black", alpha=0.12, label="1K", rasterized=True)
    ax_sc.scatter(t1b, c1b_arr, s=0.15, c=c1b, alpha=0.08, label="1B", rasterized=True)
    ax_sc.set_xlim(*VIEW_FULL)
    ax_sc.set_ylim(-5, 260)
    ax_sc.set_ylabel("Channel", fontsize=10)
    ax_sc.set_title(
        f"Box {bname} \u2014 Scatter (full, SAT shaded)", fontsize=12, fontweight="bold"
    )
    ax_sc.legend(loc="upper right", fontsize=8, markerscale=20)
    ax_sc.axvline(0, color="gray", ls=":", alpha=0.4)

    # zoom 区域框
    rect = Rectangle(
        (VIEW_ZOOM[0], -5),
        VIEW_ZOOM[1] - VIEW_ZOOM[0],
        265,
        lw=1.2,
        edgecolor="red",
        facecolor="none",
        ls="--",
        alpha=0.6,
    )
    ax_sc.add_patch(rect)
    if row == 2:
        ax_sc.set_xlabel("Time \u2212 T\u2080 (s)", fontsize=10)

    # ─── 右列: 散点 zoom ───
    ax_zm = axes[row, 2]
    draw_sat(ax_zm, bname, VIEW_ZOOM)

    m1k_z = (t1k >= VIEW_ZOOM[0]) & (t1k <= VIEW_ZOOM[1])
    m1b_z = (t1b >= VIEW_ZOOM[0]) & (t1b <= VIEW_ZOOM[1])
    nz1k = int(m1k_z.sum())
    nz1b = int(m1b_z.sum())

    ax_zm.scatter(
        t1k[m1k_z],
        c1k[m1k_z],
        s=0.5,
        c="black",
        alpha=0.2,
        label=f"1K ({nz1k:,})",
        rasterized=True,
    )
    ax_zm.scatter(
        t1b[m1b_z],
        c1b_arr[m1b_z],
        s=0.5,
        c=c1b,
        alpha=0.15,
        label=f"1B ({nz1b:,})",
        rasterized=True,
    )
    ax_zm.set_xlim(*VIEW_ZOOM)
    ax_zm.set_ylim(-5, 260)
    ax_zm.set_ylabel("Channel", fontsize=10)
    ax_zm.set_title(
        f"Box {bname} \u2014 Zoom T+{VIEW_ZOOM[0]}\u2013{VIEW_ZOOM[1]}s",
        fontsize=12,
        fontweight="bold",
    )
    ax_zm.legend(loc="upper right", fontsize=8, markerscale=15)

    dz = nz1b - nz1k
    pz = (nz1b / max(nz1k, 1) - 1) * 100
    ax_zm.text(
        0.02,
        0.95,
        f"\u0394 = {dz:+,} ({pz:+.1f}%)",
        transform=ax_zm.transAxes,
        fontsize=9,
        va="top",
        bbox=dict(boxstyle="round,pad=0.3", fc="white", alpha=0.8),
    )
    if row == 2:
        ax_zm.set_xlabel("Time \u2212 T\u2080 (s)", fontsize=10)

fig.suptitle(
    "GRB 260226A \u2014 Per-Box Event Scatter: 1K (black) vs 1B (color)\n"
    f"T\u2080 = {burst_utc.iso}  |  MET = {burst_met:.3f}",
    fontsize=14,
    fontweight="bold",
    y=0.995,
)

plt.tight_layout(rect=[0, 0, 1, 0.97])
outfile = "scatter_per_box_1k_vs_1b.png"
plt.savefig(outfile, dpi=180, bbox_inches="tight")
print(f"\nSaved: {outfile}")

# ═══════════════ 5. 统计摘要 ═══════════════
print("\n" + "=" * 60)
print("统计摘要")
print("=" * 60)
fmt = "  Box {b}:  1K={n1k:>8,}  1B={n1b:>8,}  \u0394={d:>+7,} ({p:>+6.1f}%)"
for b in ["A", "B", "C"]:
    n1k_ = len(ev1k[b]["t"])
    n1b_ = len(ev1b[b]["t"])
    d_ = n1b_ - n1k_
    p_ = (n1b_ / max(n1k_, 1) - 1) * 100
    print(fmt.format(b=b, n1k=n1k_, n1b=n1b_, d=d_, p=p_))

    # SAT 覆盖率
    if sat[b]:
        total_sat = sum(
            e - s for s, e in sat[b] if e > VIEW_FULL[0] and s < VIEW_FULL[1]
        )
        window = VIEW_FULL[1] - VIEW_FULL[0]
        print(
            f"         SAT coverage: {total_sat:.1f}s / {window}s = {total_sat / window * 100:.1f}%"
        )

total_1k = sum(len(ev1k[b]["t"]) for b in box_cfg)
total_1b = sum(len(ev1b[b]["t"]) for b in box_cfg)
print(
    f"  {'Total':5s}: 1K={total_1k:>8,}  1B={total_1b:>8,}  "
    f"\u0394={total_1b - total_1k:>+7,} ({(total_1b / max(total_1k, 1) - 1) * 100:>+6.1f}%)"
)
