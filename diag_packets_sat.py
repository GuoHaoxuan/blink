"""绘制 burst 附近每个 CCSDS 包位置 + 分 Box 饱和区间 + 秒事例标记

用法:
    # 1. 先生成数据
    HXMT_1B_DIR=data/1B cargo run -- 2020-04-15T08 --dump-packets 261564488.564 2 > dump_packets.csv
    HXMT_1B_DIR=data/1B cargo run -- 2020-04-15T08 --dump-times 261564488.564 20 > dump_200415a.txt

    # 2. 画图
    python3 diag_packets_sat.py
"""
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from matplotlib.lines import Line2D

center_met = 261564488.564

# ===== 读取包 CSV =====
boxes = {}
sec_events = {}  # box -> [time, ...]
reading_seconds = False
with open("dump_packets.csv") as f:
    for line in f:
        line = line.strip()
        if line.startswith("# second_events"):
            reading_seconds = True
            continue
        if line.startswith("#") or line.startswith("box,"):
            continue
        if reading_seconds:
            if line.startswith("SEC,"):
                parts = line.split(",")
                box_name = parts[1]
                t = float(parts[2])
                sec_events.setdefault(box_name, []).append(t)
        else:
            parts = line.split(",")
            if len(parts) < 5:
                continue
            box_name = parts[0]
            pkt_idx = int(parts[1])
            min_t = float(parts[2])
            max_t = float(parts[3])
            n_events = int(parts[4])
            boxes.setdefault(box_name, []).append((pkt_idx, min_t, max_t, n_events))

# ===== 读取 per-box 饱和区间 =====
sat_by_box = {"A": [], "B": [], "C": []}
with open("dump_200415a.txt") as f:
    for line in f:
        if line.startswith("SAT,"):
            parts = line.strip().split(",")
            box_name = parts[1]
            sat_by_box[box_name].append((float(parts[2]), float(parts[3])))

# ===== 打印摘要 =====
for b in sorted(sat_by_box.keys()):
    for s, e in sat_by_box[b]:
        print(f"  Box {b}: SAT {s-center_met:.3f}s → {e-center_met:.3f}s ({(e-s)*1000:.1f}ms)")
for b in sorted(sec_events.keys()):
    print(f"  Box {b}: {len(sec_events[b])} second events")

# ===== 绘图 =====
colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}
sat_colors = {"A": "#4a90d9", "B": "#ffaa44", "C": "#5ac85a"}
box_y = {"A": 0, "B": 1, "C": 2}

fig, axes = plt.subplots(2, 1, figsize=(18, 9), gridspec_kw={"height_ratios": [3, 3]})

for ax_idx, (xlim, title) in enumerate([
    ((-1.5, 1.5), "CCSDS Packets + Saturation + Second events (-1.5s ~ 1.5s)"),
    ((-0.05, 0.15), "Zoom: -50ms ~ 150ms (numbers = events/packet)")
]):
    ax = axes[ax_idx]

    # 饱和区间
    for box_name in ["A", "B", "C"]:
        y = box_y[box_name]
        for s, e in sat_by_box[box_name]:
            ts = s - center_met
            te = e - center_met
            ax.fill_between([ts, te], y - 0.35, y + 0.35,
                            color=sat_colors[box_name], alpha=0.25, zorder=0)

    # 包线段
    lw = 4 if ax_idx == 0 else 6
    for box_name in sorted(boxes.keys()):
        y = box_y[box_name]
        for pkt_idx, min_t, max_t, n in boxes[box_name]:
            t0 = min_t - center_met
            t1 = max_t - center_met
            ax.plot([t0, t1], [y, y], color=colors[box_name],
                    linewidth=lw, alpha=0.8, solid_capstyle='butt')
            if ax_idx == 1:
                tc = (t0 + t1) / 2
                if xlim[0] < tc < xlim[1]:
                    ax.text(tc, y + 0.2, str(n), fontsize=6,
                            ha='center', va='bottom', color=colors[box_name])

    # 秒事例标记 (红色菱形)
    for box_name in sorted(sec_events.keys()):
        y = box_y[box_name]
        for t in sec_events[box_name]:
            ts = t - center_met
            if xlim[0] <= ts <= xlim[1]:
                ax.plot(ts, y, marker='D', color='red', markersize=8 if ax_idx == 1 else 5,
                        zorder=10, markeredgecolor='darkred', markeredgewidth=0.5)

    ax.set_xlim(*xlim)
    ax.set_yticks([0, 1, 2])
    ax.set_yticklabels(["Box A", "Box B", "Box C"])
    ax.set_xlabel("Time - burst (s)")
    ax.set_title(title)
    ax.axvline(0, color='gray', linestyle='--', alpha=0.5)

    legend_elements = [
        Line2D([0], [0], color='gray', linestyle='--', label='t=0 (burst)'),
        Line2D([0], [0], marker='D', color='red', linestyle='None',
               markersize=6, markeredgecolor='darkred', label='Second event'),
    ]
    for b in ["A", "B", "C"]:
        legend_elements.append(Patch(facecolor=sat_colors[b], alpha=0.3,
                                     label=f'Box {b} saturation'))
    ax.legend(handles=legend_elements, fontsize=7, loc='upper right')

plt.tight_layout()
plt.savefig("diag_packets_sat_perbox.png", dpi=150)
print("Saved: diag_packets_sat_perbox.png")
