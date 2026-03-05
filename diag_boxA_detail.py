#!/usr/bin/env python3
"""画 Box A 问题区域的逐事例对比图，标出包边界"""

import numpy as np
from astropy.io import fits
from astropy.time import Time
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches

# --- Burst 参数 ---
burst_utc = Time("2026-02-26T10:37:55", scale="utc")
t0 = Time("2012-01-01T00:00:00", scale="utc")
burst_met = (burst_utc - t0).sec
print(f"Burst MET = {burst_met:.3f}")

# 问题区域：T+18.7-19.3s 对应 MET
t_rel_min, t_rel_max = 18.7, 19.3
met_min = burst_met + t_rel_min
met_max = burst_met + t_rel_max

# --- 读取 1B 数据 ---
print("读取 1B 数据...")

boxes_1b = []
pkts_1b = []
mets_1b = []

with open("dump_A_18.7_19.3_v2.txt") as f:
    for line in f:
        if line.startswith("#"):
            continue
        parts = line.strip().split(",")
        boxes_1b.append(parts[0])
        pkts_1b.append(int(parts[1]))
        mets_1b.append(float(parts[5]))

mets_1b = np.array(mets_1b)
pkts_1b = np.array(pkts_1b)
print(f"  1B: {len(mets_1b)} events")
print(
    f"  Packets: {pkts_1b.min()} - {pkts_1b.max()} ({len(np.unique(pkts_1b))} packets)"
)

# --- 读取 1K 数据 ---
print("读取 1K 数据...")
with fits.open(
    "data/1K/Y202602/20260226-3179/HXMT_20260226T10_HE-Evt_FFFFFF_V1_1K.FITS"
) as f:
    time_1k = f[1].data["Time"]
    det_id_1k = f[1].data["Det_ID"]

# Box A: Det_ID 0-5
m_box_a = (det_id_1k >= 0) & (det_id_1k <= 5)
m_window = (time_1k >= met_min) & (time_1k <= met_max)
m = m_box_a & m_window
time_1k_box_a = time_1k[m]
print(f"  1K Box A: {len(time_1k_box_a)} events in window")

# --- 画图 ---
fig, axes = plt.subplots(3, 1, figsize=(16, 12), sharex=True)

# 子图1: 1ms bins 的光变曲线对比
ax1 = axes[0]
bins = np.arange(met_min, met_max + 0.001, 0.001)
ax1.hist(
    time_1k_box_a,
    bins=bins,
    histtype="step",
    color="black",
    lw=0.8,
    label=f"1K Box A (n={len(time_1k_box_a)})",
    alpha=0.8,
)
ax1.hist(
    mets_1b,
    bins=bins,
    histtype="step",
    color="blue",
    lw=0.8,
    label=f"1B Box A (n={len(mets_1b)})",
    alpha=0.8,
)

# 标出包边界
pkt_indices = np.unique(pkts_1b)
for pkt in pkt_indices:
    m_pkt = pkts_1b == pkt
    pkt_min = mets_1b[m_pkt].min()
    ax1.axvline(pkt_min, color="red", lw=0.3, alpha=0.3)

ax1.set_ylabel("Counts / 1ms", fontsize=10)
ax1.set_title(f"Box A: T+{t_rel_min:.1f}s - T+{t_rel_max:.1f}s (1ms bins)", fontsize=12)
ax1.legend(loc="upper right", fontsize=9)
ax1.set_xlim(met_min, met_max)

# 添加包边界图例
red_patch = mpatches.Patch(color="red", alpha=0.3, label="Packet boundary")
ax1.legend(
    handles=[
        ax1.get_legend_handles_labels()[0][0],
        ax1.get_legend_handles_labels()[0][1],
        red_patch,
    ],
    loc="upper right",
    fontsize=9,
)

# 子图2: 0.1ms bins 放大
ax2 = axes[1]
bins_fine = np.arange(met_min, met_max + 0.0001, 0.0001)
ax2.hist(
    time_1k_box_a,
    bins=bins_fine,
    histtype="step",
    color="black",
    lw=0.5,
    label="1K",
    alpha=0.8,
)
ax2.hist(
    mets_1b,
    bins=bins_fine,
    histtype="step",
    color="blue",
    lw=0.5,
    label="1B",
    alpha=0.8,
)

for pkt in pkt_indices:
    m_pkt = pkts_1b == pkt
    pkt_min = mets_1b[m_pkt].min()
    ax2.axvline(pkt_min, color="red", lw=0.3, alpha=0.3)

ax2.set_ylabel("Counts / 0.1ms", fontsize=10)
ax2.set_title("0.1ms bins (finer detail)", fontsize=11)
ax2.legend(loc="upper right", fontsize=9)

# 子图3: 逐事例散点图
ax3 = axes[2]

# 1K 事例 (黑点)
ax3.scatter(
    time_1k_box_a, [0.8] * len(time_1k_box_a), s=2, c="black", alpha=0.5, label="1K"
)

# 1B 事例 (蓝点)
ax3.scatter(mets_1b, [1.2] * len(mets_1b), s=2, c="blue", alpha=0.3, label="1B")

# 标出包边界
for pkt in pkt_indices:
    m_pkt = pkts_1b == pkt
    pkt_min = mets_1b[m_pkt].min()
    ax3.axvline(pkt_min, color="red", lw=0.5, alpha=0.5)

ax3.set_ylabel("Source (0=1K, 1=1B)", fontsize=10)
ax3.set_xlabel("MET (s)", fontsize=10)
ax3.set_title("Event-by-event view", fontsize=11)
ax3.set_ylim(0, 2)
ax3.set_yticks([0.8, 1.2])
ax3.set_yticklabels(["1K", "1B"])
ax3.legend(loc="upper right", fontsize=9)

plt.tight_layout()
plt.savefig("diag_boxA_T18.7_19.3.png", dpi=200)
print(f"Saved: diag_boxA_T18.7_19.3.png")

# --- 打印统计信息 ---
print("\n=== 统计 ===")
print(f"1K Box A: {len(time_1k_box_a)} events")
print(f"1B Box A: {len(mets_1b)} events")
print(
    f"差异: {len(mets_1b) - len(time_1k_box_a)} events ({(len(mets_1b) / len(time_1k_box_a) - 1) * 100:.1f}%)"
)
