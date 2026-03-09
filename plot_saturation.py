"""绘制 CCSDS 包时间位置 + FIFO Reset 饱和区间标注

用法:
    python3 plot_saturation.py 200415a
    python3 plot_saturation.py 221009a
    python3 plot_saturation.py 260226a
"""

import sys
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from matplotlib.lines import Line2D

GRB_CONFIG = {
    "200415a": {"center_met": 261564488.564, "label": "GRB 200415A"},
    "221009a": {"center_met": 339945422.990, "label": "GRB 221009A"},
    "260226a": {"center_met": 446726278.000, "label": "GRB 260226A"},
}

grb = sys.argv[1].lower() if len(sys.argv) > 1 else "200415a"
cfg = GRB_CONFIG[grb]
CENTER_MET = cfg["center_met"]

# ===== 读取包位置 =====
boxes = {}
with open(f"dump_packets_{grb}.csv") as f:
    for line in f:
        line = line.strip()
        if line.startswith("box,") or line.startswith("#"):
            continue
        parts = line.split(",")
        if len(parts) < 5:
            continue
        box_name = parts[0]
        pkt_idx = int(parts[1])
        min_t = float(parts[2])
        max_t = float(parts[3])
        n_events = int(parts[4])
        boxes.setdefault(box_name, []).append((pkt_idx, min_t, max_t, n_events))

# ===== 读取饱和区间 =====
sat_by_box = {"A": [], "B": [], "C": []}
with open(f"detect_sat_{grb}.csv") as f:
    for line in f:
        line = line.strip()
        if line.startswith("box,") or line.startswith("#"):
            continue
        parts = line.split(",")
        if len(parts) < 7:
            continue
        box_name = parts[0]
        sat_type = parts[1]
        start_met = float(parts[2])
        stop_met = float(parts[3])
        gap_s = float(parts[4])
        sat_by_box.setdefault(box_name, []).append(
            (sat_type, start_met, stop_met, gap_s)
        )

for b in sorted(sat_by_box.keys()):
    n = len(sat_by_box[b])
    if n > 0:
        print(f"Box {b}: {n} FIFO reset intervals")
        for typ, s, e, g in sat_by_box[b]:
            print(
                f"  {typ}: {s - CENTER_MET:+.3f}s ~ {e - CENTER_MET:+.3f}s  gap={g * 1000:.1f}ms"
            )

colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}
box_y = {"A": 0, "B": 1, "C": 2}

fig, axes = plt.subplots(2, 1, figsize=(18, 8), gridspec_kw={"height_ratios": [1, 1]})

views = [
    ((-2.0, 2.0), "Overview: ±2s around burst"),
    ((-0.05, 0.15), "Zoom: -50ms ~ 150ms"),
]

for ax_idx, (xlim, title) in enumerate(views):
    ax = axes[ax_idx]

    for box_name in ["A", "B", "C"]:
        y = box_y[box_name]
        for typ, s, e, g in sat_by_box[box_name]:
            ts = s - CENTER_MET
            te = e - CENTER_MET
            if te < xlim[0] or ts > xlim[1]:
                continue
            ax.axvspan(
                ts, te, ymin=(y) / 3, ymax=(y + 1) / 3, color="red", alpha=0.3, zorder=5
            )
            mid = (ts + te) / 2
            if xlim[0] < mid < xlim[1] and ax_idx == 1:
                ax.annotate(
                    f"{g * 1000:.1f}ms",
                    xy=(mid, y + 0.35),
                    fontsize=7,
                    ha="center",
                    va="bottom",
                    color="red",
                )

    lw = 3 if ax_idx == 0 else 5
    for box_name in sorted(boxes.keys()):
        y = box_y[box_name]
        for pkt_idx, min_t, max_t, n in boxes[box_name]:
            t0 = min_t - CENTER_MET
            t1 = max_t - CENTER_MET
            if t1 < xlim[0] or t0 > xlim[1]:
                continue
            ax.plot(
                [t0, t1],
                [y, y],
                color=colors[box_name],
                linewidth=lw,
                alpha=0.7,
                solid_capstyle="butt",
            )

    ax.set_xlim(*xlim)
    ax.set_ylim(-0.5, 2.5)
    ax.set_yticks([0, 1, 2])
    ax.set_yticklabels(["Box A", "Box B", "Box C"])
    ax.set_xlabel("Time - burst (s)")
    ax.set_title(title)
    ax.axvline(0, color="gray", linestyle="--", alpha=0.5, label="t=0 (burst)")

    legend_elements = [
        Line2D([0], [0], color="gray", linestyle="--", label="t=0 (burst)"),
        Patch(facecolor="red", alpha=0.3, label="FIFO Reset (saturation)"),
    ]
    for b in ["A", "B", "C"]:
        legend_elements.append(
            Line2D([0], [0], color=colors[b], linewidth=3, label=f"Box {b} packets")
        )
    ax.legend(handles=legend_elements, fontsize=7, loc="upper right")

axes[0].set_title(f"{cfg['label']} — {views[0][1]}")
plt.tight_layout()
outfile = f"plot_saturation_{grb}.png"
plt.savefig(outfile, dpi=150)
print(f"Saved: {outfile}")
