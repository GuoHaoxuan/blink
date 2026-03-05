#!/usr/bin/env python3
"""从 Rust --dump-hist 输出画光变+饱和图。数据已预分好bin，Python只做画图。

用法:
    # 先用 Rust 生成 histogram:
    # HXMT_1B_DIR=data/1B cargo run -- 2022-10-09T13 --dump-hist 339945422.990 300 0.01 > hist_221009a.csv
    # 然后画图:
    python3 diag_plot_hist.py hist_221009a.csv [grb_name]
"""
import sys
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec
from matplotlib.patches import Patch

csv_file = sys.argv[1]
grb_name = sys.argv[2] if len(sys.argv) > 2 else csv_file.replace("hist_","").replace(".csv","")

# 解析
bin_starts = []
counts = []
sat_by_box = {"A": [], "B": [], "C": []}
center_met = 0.0
bin_width = 0.01

with open(csv_file) as f:
    in_hist = False
    for line in f:
        line = line.strip()
        if line.startswith("# center_met="):
            center_met = float(line.split("=")[1])
        elif line.startswith("# bin_width="):
            bin_width = float(line.split("=")[1])
        elif line == "# HIST":
            in_hist = True
            continue
        elif line == "# SAT":
            in_hist = False
            continue
        elif line.startswith("SAT,"):
            parts = line.split(",")
            sat_by_box[parts[1]].append((float(parts[2]), float(parts[3])))
        elif line.startswith("#"):
            continue
        elif in_hist:
            t, c = line.split(",")
            bin_starts.append(float(t))
            counts.append(int(c))

bin_starts = np.array(bin_starts)
counts = np.array(counts)
t_rel = bin_starts - center_met

print(f"Loaded: {len(counts)} bins, center={center_met}, bin_width={bin_width}")
print(f"SAT: A={len(sat_by_box['A'])}, B={len(sat_by_box['B'])}, C={len(sat_by_box['C'])}")

sat_colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}

# 自适应视图
total_range = t_rel[-1] - t_rel[0]
views = [
    (t_rel[0], t_rel[-1], "Full range"),
    (max(0, t_rel[0]), min(total_range/2, t_rel[-1]), "First half detail"),
]

# 找峰值区
peak_idx = np.argmax(counts)
peak_t = t_rel[peak_idx]
views.append((peak_t - 30, peak_t + 30, f"Peak region (t≈{peak_t:.0f}s)"))

fig = plt.figure(figsize=(18, 14))
gs = gridspec.GridSpec(len(views)*2, 1, height_ratios=[6,1]*len(views), hspace=0.05)

for vi, (tmin, tmax, title) in enumerate(views):
    ax_lc = fig.add_subplot(gs[vi*2])
    ax_sat = fig.add_subplot(gs[vi*2+1], sharex=ax_lc)

    # 选取视图范围内的bin，按视图宽度决定是否rebinning
    m = (t_rel >= tmin) & (t_rel < tmax)
    view_bins = int(m.sum())
    
    # 如果bin太多（>2000），做rebinning
    if view_bins > 2000:
        rebin = max(1, view_bins // 1000)
        # 截断到rebin整数倍
        idx = np.where(m)[0]
        n_use = (len(idx) // rebin) * rebin
        idx = idx[:n_use]
        rebinned = counts[idx].reshape(-1, rebin).sum(axis=1)
        rebinned_t = t_rel[idx].reshape(-1, rebin)[:, 0]
        effective_bw = bin_width * rebin
        ax_lc.step(rebinned_t, rebinned, where='post', color='blue', lw=0.8,
                   label=f'1B (rebinned {rebin}×, bw={effective_bw*1000:.0f}ms)')
    else:
        ax_lc.step(t_rel[m], counts[m], where='post', color='blue', lw=0.8,
                   label=f'1B ({bin_width*1000:.0f}ms bins)')

    # 饱和区间
    for bn in ["A","B","C"]:
        for s,e in sat_by_box[bn]:
            sr, er = s-center_met, e-center_met
            if er > tmin and sr < tmax:
                ax_lc.axvspan(max(sr,tmin), min(er,tmax), alpha=0.10, color=sat_colors[bn], zorder=0)

    ax_lc.set_title(f"{grb_name}: {title}", fontsize=11)
    handles, labels = ax_lc.get_legend_handles_labels()
    for b in ["A","B","C"]:
        handles.append(Patch(facecolor=sat_colors[b], alpha=0.3, label=f'SAT {b}'))
    ax_lc.legend(handles=handles, fontsize=7, loc='upper right')
    ax_lc.set_ylabel("Counts / bin")
    ax_lc.axvline(0, color='gray', ls=':', alpha=0.5)
    ax_lc.set_xlim(tmin, tmax)
    plt.setp(ax_lc.get_xticklabels(), visible=False)

    box_y = {"C":2,"B":1,"A":0}
    for bn in ["A","B","C"]:
        y = box_y[bn]
        for s,e in sat_by_box[bn]:
            sr,er = s-center_met, e-center_met
            if er > tmin and sr < tmax:
                ax_sat.barh(y, min(er,tmax)-max(sr,tmin), left=max(sr,tmin), height=0.8, color=sat_colors[bn], alpha=0.8)
    ax_sat.set_yticks([0,1,2])
    ax_sat.set_yticklabels(["A","B","C"], fontsize=7)
    ax_sat.set_ylim(-0.5, 2.5)
    ax_sat.set_xlim(tmin, tmax)
    if vi < len(views)-1:
        plt.setp(ax_sat.get_xticklabels(), visible=False)
    else:
        ax_sat.set_xlabel("Time - burst (s)")
    ax_sat.yaxis.set_label_position("right")
    ax_sat.set_ylabel("SAT", fontsize=8, rotation=0, labelpad=15, va='center')

out = csv_file.replace(".csv", ".png")
plt.tight_layout()
plt.savefig(out, dpi=150)
print(f"Saved: {out}")
