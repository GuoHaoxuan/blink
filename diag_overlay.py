#!/usr/bin/env python3
"""叠加 1K (FITS) + 1B (Rust hist CSV) 光变+饱和图。

用法:
    python3 diag_overlay.py hist_221009a.csv data/1K/.../XXX.FITS <center_met> [grb_name]
"""
import sys
import numpy as np
from astropy.io import fits
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec
from matplotlib.patches import Patch

hist_csv = sys.argv[1]
fits_path = sys.argv[2]
center_met = float(sys.argv[3])
grb_name = sys.argv[4] if len(sys.argv) > 4 else "GRB"

# 1. 读 Rust histogram
bin_starts_1b = []
counts_1b = []
sat_by_box = {"A": [], "B": [], "C": []}
bin_width = 0.01

with open(hist_csv) as f:
    in_hist = False
    for line in f:
        line = line.strip()
        if line.startswith("# bin_width="):
            bin_width = float(line.split("=")[1])
        elif line == "# HIST":
            in_hist = True; continue
        elif line == "# SAT":
            in_hist = False; continue
        elif line.startswith("SAT,"):
            p = line.split(",")
            sat_by_box[p[1]].append((float(p[2]), float(p[3])))
        elif line.startswith("#"):
            continue
        elif in_hist:
            t, c = line.split(",")
            bin_starts_1b.append(float(t))
            counts_1b.append(int(c))

bin_starts_1b = np.array(bin_starts_1b)
counts_1b = np.array(counts_1b)
met_min = bin_starts_1b[0]
met_max = bin_starts_1b[-1] + bin_width

# 2. 读 1K FITS 并用相同 bins 做 histogram
with fits.open(fits_path) as f:
    t1k_all = f[1].data["Time"]
m = (t1k_all >= met_min) & (t1k_all < met_max)
t1k = t1k_all[m]
bins_edge = np.arange(met_min, met_max + bin_width, bin_width)
counts_1k, _ = np.histogram(t1k, bins=bins_edge)
# 确保长度一致
n = min(len(counts_1b), len(counts_1k))
counts_1b = counts_1b[:n]
counts_1k = counts_1k[:n]
bin_starts_1b = bin_starts_1b[:n]

t_rel = bin_starts_1b - center_met
print(f"1B: {counts_1b.sum()} events, 1K: {counts_1k.sum()} events, bins={n}")

sat_colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}
half_w = (met_max - met_min) / 2

views = [
    (-half_w, half_w, f"Full range ({bin_width*1000:.0f}ms bins)"),
    (0, half_w, f"After trigger ({bin_width*1000:.0f}ms bins)"),
]
# Peak region
peak_idx = np.argmax(counts_1k)
pk = t_rel[peak_idx]
views.append((pk - 15, pk + 15, f"Peak (t≈{pk:.0f}s)"))

fig = plt.figure(figsize=(18, 14))
gs = gridspec.GridSpec(len(views)*2, 1, height_ratios=[6,1]*len(views), hspace=0.05)

for vi, (tmin, tmax, title) in enumerate(views):
    ax_lc = fig.add_subplot(gs[vi*2])
    ax_sat = fig.add_subplot(gs[vi*2+1], sharex=ax_lc)

    m_v = (t_rel >= tmin) & (t_rel < tmax)
    view_n = int(m_v.sum())

    # rebinning if needed
    if view_n > 2000:
        rebin = max(1, view_n // 1000)
        idx = np.where(m_v)[0]
        n_use = (len(idx) // rebin) * rebin
        idx = idx[:n_use]
        rb_1k = counts_1k[idx].reshape(-1, rebin).sum(axis=1)
        rb_1b = counts_1b[idx].reshape(-1, rebin).sum(axis=1)
        rb_t = t_rel[idx].reshape(-1, rebin)[:, 0]
        bw_eff = bin_width * rebin
        ax_lc.step(rb_t, rb_1k, where='post', color='black', lw=1.0,
                   label=f'1K (bw={bw_eff*1000:.0f}ms)')
        ax_lc.step(rb_t, rb_1b, where='post', color='blue', lw=0.8, alpha=0.8,
                   label=f'1B (bw={bw_eff*1000:.0f}ms)')
    else:
        ax_lc.step(t_rel[m_v], counts_1k[m_v], where='post', color='black', lw=1.0,
                   label=f'1K ({bin_width*1000:.0f}ms bins)')
        ax_lc.step(t_rel[m_v], counts_1b[m_v], where='post', color='blue', lw=0.8, alpha=0.8,
                   label=f'1B ({bin_width*1000:.0f}ms bins)')

    for bn in ["A","B","C"]:
        for s,e in sat_by_box[bn]:
            sr, er = s-center_met, e-center_met
            if er > tmin and sr < tmax:
                ax_lc.axvspan(max(sr,tmin), min(er,tmax), alpha=0.08, color=sat_colors[bn], zorder=0)

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

out = hist_csv.replace(".csv", "_overlay.png")
plt.tight_layout()
plt.savefig(out, dpi=150)
print(f"Saved: {out}")
