"""全局视图：整个小时的事件率 + 所有 FIFO Reset 饱和区间

用法:
    HXMT_1B_DIR=data/1B cargo run --release -- 2020-04-15T08 --dump-packets 261564488.564 1800 > dump_packets_full.csv
    HXMT_1B_DIR=data/1B cargo run --release -- 2020-04-15T08 --detect-saturation > detect_sat.csv
    python3 plot_saturation_overview.py
"""

import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from matplotlib.lines import Line2D

# ===== 读取包位置 =====
boxes = {}
with open("dump_packets_full.csv") as f:
    for line in f:
        line = line.strip()
        if (
            line.startswith("box,")
            or line.startswith("#")
            or line.startswith("Loading")
        ):
            continue
        parts = line.split(",")
        if len(parts) < 5:
            continue
        box_name = parts[0]
        min_t = float(parts[2])
        max_t = float(parts[3])
        n_events = int(parts[4])
        boxes.setdefault(box_name, []).append((min_t, max_t, n_events))

# ===== 读取饱和区间 =====
sat_by_box = {"A": [], "B": [], "C": []}
with open("detect_sat.csv") as f:
    for line in f:
        line = line.strip()
        if line.startswith("box,") or line.startswith("#"):
            continue
        parts = line.split(",")
        if len(parts) < 7:
            continue
        box_name = parts[0]
        start_met = float(parts[2])
        stop_met = float(parts[3])
        gap_s = float(parts[4])
        sat_by_box.setdefault(box_name, []).append((start_met, stop_met, gap_s))

CENTER_MET = 261564488.564

all_min = min(t[0] for pkts in boxes.values() for t in pkts)
all_max = max(t[1] for pkts in boxes.values() for t in pkts)

bin_width = 1.0
t_edges = np.arange(all_min, all_max + bin_width, bin_width)

colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}

fig, axes = plt.subplots(3, 1, figsize=(20, 10), sharex=True)

for ax_idx, box_name in enumerate(["A", "B", "C"]):
    ax = axes[ax_idx]
    pkts = boxes.get(box_name, [])

    rates = np.zeros(len(t_edges) - 1)
    for min_t, max_t, n_events in pkts:
        mid = (min_t + max_t) / 2
        idx = int((mid - all_min) / bin_width)
        if 0 <= idx < len(rates):
            span = max_t - min_t
            if span > 1e-6:
                rates[idx] += n_events

    t_centers = (t_edges[:-1] + t_edges[1:]) / 2 - CENTER_MET
    ax.fill_between(t_centers, rates, color=colors[box_name], alpha=0.5)
    ax.plot(t_centers, rates, color=colors[box_name], linewidth=0.5)

    for start_met, stop_met, gap_s in sat_by_box.get(box_name, []):
        mid = (start_met + stop_met) / 2 - CENTER_MET
        ax.axvline(mid, color="red", linewidth=1.5, alpha=0.8, zorder=10)
        ax.annotate(
            f"{gap_s * 1000:.1f}ms",
            xy=(mid, ax.get_ylim()[1] if ax.get_ylim()[1] > 0 else 1),
            xytext=(0, 5),
            textcoords="offset points",
            fontsize=6,
            ha="center",
            va="bottom",
            color="red",
            rotation=90,
        )

    ax.set_ylabel(f"Box {box_name}\nevents/s")
    ax.grid(True, alpha=0.3)

    n_sat = len(sat_by_box.get(box_name, []))
    legend_elements = [
        Line2D(
            [0],
            [0],
            color=colors[box_name],
            linewidth=3,
            label=f"Box {box_name} event rate",
        ),
        Line2D([0], [0], color="red", linewidth=1.5, label=f"FIFO Reset ({n_sat})"),
    ]
    ax.legend(handles=legend_elements, fontsize=8, loc="upper right")

axes[-1].set_xlabel("Time - burst (s)")
axes[0].set_title("GRB 200415A — Event Rate + FIFO Reset Saturation (full hour)")

plt.tight_layout()
plt.savefig("plot_saturation_overview.png", dpi=150)
print("Saved: plot_saturation_overview.png")
