#!/usr/bin/env python3
"""画每个 Box 的 1K vs 1B 对比图，散点图用 channel 作为 Y 轴"""

import numpy as np
from astropy.io import fits
from astropy.time import Time
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from collections import defaultdict

# --- Burst 参数 ---
burst_utc = Time("2026-02-26T10:37:55", scale="utc")
t0 = Time("2012-01-01T00:00:00", scale="utc")
burst_met = (burst_utc - t0).sec
print(f"Burst MET = {burst_met:.3f}")

# 问题区域：T+18.7-19.3s
t_rel_min, t_rel_max = 18.7, 19.3
met_min = burst_met + t_rel_min
met_max = burst_met + t_rel_max

# Box 到 Det_ID 的映射
box_info = {
    "A": {"det_ids": (0, 5), "color": "#1f77b4"},
    "B": {"det_ids": (6, 11), "color": "#ff7f0e"},
    "C": {"det_ids": (12, 17), "color": "#2ca02c"},
}


def load_1b_dump(filepath):
    """加载 1B dump 文件"""
    boxes = []
    pkts = []
    channels = []
    mets = []

    with open(filepath) as f:
        for line in f:
            if line.startswith("#"):
                continue
            parts = line.strip().split(",")
            boxes.append(parts[0])
            pkts.append(int(parts[1]))
            channels.append(int(parts[4]))
            mets.append(float(parts[5]))

    return (np.array(boxes), np.array(pkts), np.array(channels), np.array(mets))


def load_1k_data(box_name, det_min, det_max):
    """加载 1K 数据中特定 Box 的数据"""
    with fits.open(
        "data/1K/Y202602/20260226-3179/HXMT_20260226T10_HE-Evt_FFFFFF_V1_1K.FITS"
    ) as f:
        time_1k = f[1].data["Time"]
        det_id_1k = f[1].data["Det_ID"]
        channel_1k = f[1].data["Channel"]

    m_box = (det_id_1k >= det_min) & (det_id_1k <= det_max)
    m_window = (time_1k >= met_min) & (time_1k <= met_max)
    m = m_box & m_window

    return time_1k[m], channel_1k[m]


def get_packet_boundaries(pkts, mets):
    """获取每个包的起始和结束时间"""
    pkt_bounds = {}
    for i in range(len(pkts)):
        pkt = pkts[i]
        met = mets[i]
        if pkt not in pkt_bounds:
            pkt_bounds[pkt] = {"min": met, "max": met}
        else:
            pkt_bounds[pkt]["min"] = min(pkt_bounds[pkt]["min"], met)
            pkt_bounds[pkt]["max"] = max(pkt_bounds[pkt]["max"], met)
    return pkt_bounds


# 加载 1K 数据
print("读取 1K 数据...")
data_1k = {}
for box_name, info in box_info.items():
    det_min, det_max = info["det_ids"]
    times, channels = load_1k_data(box_name, det_min, det_max)
    data_1k[box_name] = {"time": times, "channel": channels}
    print(f"  1K Box {box_name}: {len(times)} events")

# 为每个 Box 画图
for box_name in ["A", "B", "C"]:
    print(f"\n处理 Box {box_name}...")

    # 加载 1B 数据
    # Box A 需要用 _v2 版本（包含完整的问题区域）
    dump_file = (
        f"dump_{box_name}_18.7_19.3_v2.txt"
        if box_name == "A"
        else f"dump_{box_name}_18.7_19.3.txt"
    )
    boxes_1b, pkts_1b, channels_1b, mets_1b = load_1b_dump(dump_file)
    print(f"  1B Box {box_name}: {len(mets_1b)} events")

    # 获取包边界
    pkt_bounds = get_packet_boundaries(pkts_1b, mets_1b)
    pkts_sorted = sorted(pkt_bounds.keys())
    print(
        f"  Packets: {pkts_sorted[0]} - {pkts_sorted[-1]} ({len(pkts_sorted)} packets)"
    )

    # 获取 1K 数据
    time_1k = data_1k[box_name]["time"]
    channel_1k = data_1k[box_name]["channel"]

    # 创建图
    fig, axes = plt.subplots(3, 1, figsize=(18, 14))

    # ========== 子图1: 1ms bins 光变曲线 ==========
    ax1 = axes[0]
    bins = np.arange(met_min, met_max + 0.001, 0.001)
    ax1.hist(
        time_1k,
        bins=bins,
        histtype="step",
        color="black",
        lw=0.8,
        label=f"1K (n={len(time_1k)})",
        alpha=0.8,
    )
    ax1.hist(
        mets_1b,
        bins=bins,
        histtype="step",
        color="blue",
        lw=0.8,
        label=f"1B (n={len(mets_1b)})",
        alpha=0.8,
    )

    # 包边界 - 用半透明矩形表示包的时间跨度
    for pkt in pkts_sorted:
        ax1.axvline(pkt_bounds[pkt]["min"], color="red", lw=0.3, alpha=0.3)

    ax1.set_ylabel("Counts / 1ms", fontsize=10)
    ax1.set_title(
        f"Box {box_name}: T+{t_rel_min:.1f}s - T+{t_rel_max:.1f}s (1ms bins)",
        fontsize=12,
    )
    ax1.legend(loc="upper right", fontsize=9)
    ax1.set_xlim(met_min, met_max)

    red_patch = mpatches.Patch(color="red", alpha=0.3, label="Packet start")
    ax1.legend(
        handles=[
            ax1.get_legend_handles_labels()[0][0],
            ax1.get_legend_handles_labels()[0][1],
            red_patch,
        ],
        loc="upper right",
        fontsize=9,
    )

    # ========== 子图2: 散点图 - Time vs Channel ==========
    ax2 = axes[1]

    # 1K 数据点
    ax2.scatter(time_1k, channel_1k, s=3, c="black", alpha=0.4, label="1K", marker=".")

    # 1B 数据点
    ax2.scatter(mets_1b, channels_1b, s=3, c="blue", alpha=0.3, label="1B", marker=".")

    # 包边界 - 用红色竖线
    for pkt in pkts_sorted:
        ax2.axvline(pkt_bounds[pkt]["min"], color="red", lw=0.5, alpha=0.4)

    ax2.set_ylabel("Channel", fontsize=10)
    ax2.set_title(f"Time vs Channel (scatter)", fontsize=11)
    ax2.legend(loc="upper right", fontsize=9)
    ax2.set_xlim(met_min, met_max)
    ax2.set_ylim(0, 256)

    # ========== 子图3: 放大视图 - 找一个小区域 ==========
    ax3 = axes[2]

    # 选择一个小区域来放大，比如 T+19.0-19.1s
    zoom_min = burst_met + 19.0
    zoom_max = burst_met + 19.1

    # 1K 放大
    m_zoom_1k = (time_1k >= zoom_min) & (time_1k < zoom_max)
    ax3.scatter(
        time_1k[m_zoom_1k],
        channel_1k[m_zoom_1k],
        s=8,
        c="black",
        alpha=0.6,
        label="1K",
        marker=".",
    )

    # 1B 放大
    m_zoom_1b = (mets_1b >= zoom_min) & (mets_1b < zoom_max)
    ax3.scatter(
        mets_1b[m_zoom_1b],
        channels_1b[m_zoom_1b],
        s=8,
        c="blue",
        alpha=0.4,
        label="1B",
        marker=".",
    )

    # 包边界 - 放大视图
    for pkt in pkts_sorted:
        if (
            pkt_bounds[pkt]["min"] >= zoom_min - 0.01
            and pkt_bounds[pkt]["min"] <= zoom_max + 0.01
        ):
            ax3.axvline(pkt_bounds[pkt]["min"], color="red", lw=1, alpha=0.7)
            ax3.text(
                pkt_bounds[pkt]["min"],
                250,
                f"{pkt}",
                fontsize=7,
                rotation=90,
                va="top",
                color="red",
                alpha=0.7,
            )

    ax3.set_ylabel("Channel", fontsize=10)
    ax3.set_xlabel("MET (s)", fontsize=10)
    ax3.set_title(
        f"Zoom: T+{19.0:.1f}s - T+{19.1:.1f}s (packet boundaries labeled)", fontsize=11
    )
    ax3.legend(loc="upper right", fontsize=9)
    ax3.set_xlim(zoom_min, zoom_max)
    ax3.set_ylim(0, 256)

    plt.tight_layout()
    plt.savefig(f"diag_box{box_name}_T18.7_19.3.png", dpi=200)
    print(f"  Saved: diag_box{box_name}_T18.7_19.3.png")

    # 打印统计
    print(f"\n  === Box {box_name} 统计 ===")
    print(f"  1K: {len(time_1k)} events")
    print(f"  1B: {len(mets_1b)} events")
    print(
        f"  差异: {len(mets_1b) - len(time_1k)} events ({(len(mets_1b) / max(1, len(time_1k)) - 1) * 100:.1f}%)"
    )
