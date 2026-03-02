#!/usr/bin/env python3
"""画 GRB 200415A: 1K光变 + 1B光变(Rust) + per-box 饱和区间

用法:
    HXMT_1B_DIR=data/1B cargo run -- 2020-04-15T08 --dump-times 261564488.564 20 > dump_200415a.txt
    python3 diag_grb200415a_sat.py
"""

import numpy as np
from astropy.io import fits
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
import matplotlib.gridspec as gridspec

# --- 1. 读取 Rust 输出 ---
print("读取 Rust 输出...")
met_1b = []
sat_by_box = {"A": [], "B": [], "C": []}
center_met = None

with open("dump_200415a.txt") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        if line.startswith("# center_met="):
            center_met = float(line.split("=")[1])
        elif line.startswith("SAT,"):
            parts = line.split(",")
            box_name = parts[1]
            sat_by_box[box_name].append((float(parts[2]), float(parts[3])))
        elif line.startswith("#"):
            continue
        else:
            met_1b.append(float(line))

met_1b = np.array(met_1b)
total_sat = sum(len(v) for v in sat_by_box.values())
print(f"  1B: {len(met_1b)} events, center={center_met}")
print(f"  饱和区间: {total_sat} (A={len(sat_by_box['A'])}, B={len(sat_by_box['B'])}, C={len(sat_by_box['C'])})")

# --- 2. 读取 1K ---
print("读取 1K...")
with fits.open("data/1K/Y202004/20200415-1036/HXMT_20200415T08_HE-Evt_FFFFFF_V2_1K.FITS") as f:
    time_1k = f[1].data["Time"]
m = (time_1k >= center_met - 20) & (time_1k <= center_met + 20)
time_1k = time_1k[m]
print(f"  1K: {len(time_1k)} events in window")

# --- 3. 画图 ---
burst_met = center_met
t1k_rel = time_1k - burst_met
t1b_rel = met_1b - burst_met

sat_colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}

views = [
    (-10, 10, 0.05, "Wide view (50ms bins)"),
    (-1, 2, 0.005, "Burst region (5ms bins)"),
    (-0.05, 0.3, 0.001, "Burst peak (1ms bins)"),
]

# 每个 view 对应一个光变面板 + 一个饱和指示面板
fig = plt.figure(figsize=(16, 14))
gs = gridspec.GridSpec(len(views) * 2, 1,
                       height_ratios=[6, 1] * len(views),
                       hspace=0.05)

for vi, (tmin, tmax, bw, title) in enumerate(views):
    ax_lc = fig.add_subplot(gs[vi * 2])
    ax_sat = fig.add_subplot(gs[vi * 2 + 1], sharex=ax_lc)

    # 光变
    bins = np.arange(tmin, tmax + bw, bw)
    m1k = (t1k_rel >= tmin) & (t1k_rel < tmax)
    m1b = (t1b_rel >= tmin) & (t1b_rel < tmax)
    ax_lc.hist(t1k_rel[m1k], bins=bins, histtype='step', color='black', lw=1.0,
               label=f'1K (n={m1k.sum()})')
    ax_lc.hist(t1b_rel[m1b], bins=bins, histtype='step', color='blue', lw=0.8, alpha=0.8,
               label=f'1B Rust (n={m1b.sum()})')
    # 光变上也叠加淡色饱和区间
    for box_name in ["A", "B", "C"]:
        for start, stop in sat_by_box[box_name]:
            s_rel, e_rel = start - burst_met, stop - burst_met
            if e_rel > tmin and s_rel < tmax:
                ax_lc.axvspan(max(s_rel, tmin), min(e_rel, tmax), alpha=0.10,
                              color=sat_colors[box_name], zorder=0)

    ax_lc.set_title(f"GRB 200415A: {title}", fontsize=11)
    handles, labels = ax_lc.get_legend_handles_labels()
    for b in ["A", "B", "C"]:
        handles.append(Patch(facecolor=sat_colors[b], alpha=0.3, label=f'SAT {b}'))
    ax_lc.legend(handles=handles, fontsize=7, loc='upper right')
    ax_lc.set_ylabel("Counts / bin")
    ax_lc.axvline(0, color='gray', ls=':', alpha=0.5)
    ax_lc.set_xlim(tmin, tmax)
    plt.setp(ax_lc.get_xticklabels(), visible=False)

    # 饱和指示条 (每个 box 占一行)
    box_y = {"C": 2, "B": 1, "A": 0}
    for box_name in ["A", "B", "C"]:
        y = box_y[box_name]
        for start, stop in sat_by_box[box_name]:
            s_rel, e_rel = start - burst_met, stop - burst_met
            if e_rel > tmin and s_rel < tmax:
                ax_sat.barh(y, min(e_rel, tmax) - max(s_rel, tmin),
                            left=max(s_rel, tmin), height=0.8,
                            color=sat_colors[box_name], alpha=0.8)

    ax_sat.set_yticks([0, 1, 2])
    ax_sat.set_yticklabels(["A", "B", "C"], fontsize=7)
    ax_sat.set_ylim(-0.5, 2.5)
    ax_sat.set_xlim(tmin, tmax)
    ax_sat.tick_params(axis='x', labelsize=8)
    if vi < len(views) - 1:
        plt.setp(ax_sat.get_xticklabels(), visible=False)
    else:
        ax_sat.set_xlabel("Time - burst (s)")

    # 右侧标注
    ax_sat.yaxis.set_label_position("right")
    ax_sat.set_ylabel("SAT", fontsize=8, rotation=0, labelpad=15, va='center')

plt.tight_layout()
plt.savefig("diag_grb200415a_sat.png", dpi=150)
print(f"Saved: diag_grb200415a_sat.png")
