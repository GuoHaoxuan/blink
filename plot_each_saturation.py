"""为每个 FIFO Reset 饱和区间画一张放大图，展示附近的包和包内事例。

用法:
    python3 plot_each_saturation.py 200415a
    python3 plot_each_saturation.py 221009a [--max N]
    python3 plot_each_saturation.py 260226a
"""

import subprocess
import os
import sys
import matplotlib.pyplot as plt
import numpy as np

GRB_CONFIG = {
    "200415a": {"obs_id": "2020-04-15T08", "label": "GRB 200415A"},
    "221009a": {"obs_id": "2022-10-09T13", "label": "GRB 221009A"},
    "260226a": {"obs_id": "2026-02-26T10", "label": "GRB 260226A"},
}

grb = sys.argv[1].lower() if len(sys.argv) > 1 else "200415a"
cfg = GRB_CONFIG[grb]

max_plots = None
if "--max" in sys.argv:
    idx = sys.argv.index("--max")
    max_plots = int(sys.argv[idx + 1])

BLINK_CLI = "./target/release/blink_cli"
OBS_ID = cfg["obs_id"]
DATA_DIR = os.environ.get("HXMT_1B_DIR", "data/1B")

colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}
box_y = {"A": 0, "B": 1, "C": 2}

all_sat_intervals = []
with open(f"detect_sat_{grb}.csv") as f:
    for line in f:
        line = line.strip()
        if line.startswith("box,") or line.startswith("#"):
            continue
        parts = line.split(",")
        if len(parts) < 7:
            continue
        all_sat_intervals.append(
            {
                "box": parts[0],
                "type": parts[1],
                "start": float(parts[2]),
                "stop": float(parts[3]),
                "gap_s": float(parts[4]),
                "prev_pkt": int(parts[5]),
                "next_pkt": int(parts[6]),
            }
        )

# 用于画图的子集（--max 限制画哪些图，但标注用完整列表）
sat_intervals = list(all_sat_intervals)
if max_plots and len(sat_intervals) > max_plots:
    # 按 gap 大小排序，取最大的 N 个
    sat_intervals.sort(key=lambda x: x["gap_s"], reverse=True)
    sat_intervals = sat_intervals[:max_plots]
    sat_intervals.sort(key=lambda x: x["start"])

print(f"Found {len(sat_intervals)} saturation intervals for {cfg['label']}")

out_dir = f"sat_plots_{grb}"
os.makedirs(out_dir, exist_ok=True)


def run_cli(*extra_args):
    env = os.environ.copy()
    env["HXMT_1B_DIR"] = DATA_DIR
    cmd = [BLINK_CLI, OBS_ID] + list(extra_args)
    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    return result.stdout


def parse_packets(text, target_box):
    packets = []
    for line in text.strip().split("\n"):
        if line.startswith("box,") or line.startswith("#") or line.startswith("SEC,"):
            continue
        parts = line.split(",")
        if len(parts) < 5:
            continue
        bx, pkt_idx, min_t, max_t, n = (
            parts[0],
            int(parts[1]),
            float(parts[2]),
            float(parts[3]),
            int(parts[4]),
        )
        if bx == target_box:
            packets.append((pkt_idx, min_t, max_t, n))
    return packets


def parse_events(text, target_box):
    events = []
    for line in text.strip().split("\n"):
        if line.startswith("#") or not line:
            continue
        parts = line.split(",")
        if len(parts) < 6:
            continue
        bx = parts[0]
        if bx != target_box:
            continue
        pkt_idx = int(parts[1])
        is_sec = parts[3] == "SEC"
        met = float(parts[5])
        events.append((pkt_idx, is_sec, met))
    return events


for i, sat in enumerate(sat_intervals):
    center = (sat["start"] + sat["stop"]) / 2
    gap_ms = sat["gap_s"] * 1000
    half_window = min(max(sat["gap_s"] * 10, 0.06), 2.5)

    print(
        f"[{i + 1}/{len(sat_intervals)}] Box {sat['box']} gap={gap_ms:.1f}ms center={center:.3f} window=±{half_window:.3f}s"
    )

    pkt_text = run_cli(
        "--box", sat["box"], "--dump-packets", f"{center:.6f}", f"{half_window:.6f}"
    )
    evt_text = run_cli(
        "--box", sat["box"], "--dump-events", f"{center:.6f}", f"{half_window:.6f}"
    )

    packets = parse_packets(pkt_text, sat["box"])
    events = parse_events(evt_text, sat["box"])

    if not packets:
        print(f"  No packets found, skipping")
        continue

    fig, axes = plt.subplots(
        2, 1, figsize=(16, 7), gridspec_kw={"height_ratios": [1, 1.2]}
    )

    gap_start = sat["start"] - center
    gap_stop = sat["stop"] - center

    nearby_sats = [
        s
        for s in all_sat_intervals
        if s["box"] == sat["box"]
        and s is not sat
        and s["stop"] > center - half_window
        and s["start"] < center + half_window
    ]

    for ax_idx, (xlim_factor, subtitle) in enumerate(
        [
            (1.0, "Overview"),
            (0.2, "Zoom"),
        ]
    ):
        ax = axes[ax_idx]
        xlim = (-half_window * xlim_factor, half_window * xlim_factor)

        ax.axvspan(gap_start, gap_stop, color="red", alpha=0.2, zorder=0)
        for ns in nearby_sats:
            ns_start = ns["start"] - center
            ns_stop = ns["stop"] - center
            if ns_stop >= xlim[0] and ns_start <= xlim[1]:
                ax.axvspan(ns_start, ns_stop, color="red", alpha=0.2, zorder=0)
                ns_mid = (ns_start + ns_stop) / 2
                ax.annotate(
                    f"{ns['gap_s'] * 1000:.1f}ms",
                    xy=(ns_mid, 0.05),
                    fontsize=7,
                    ha="center",
                    va="bottom",
                    color="red",
                    alpha=0.7,
                )

        for pkt_idx, min_t, max_t, n in packets:
            t0 = min_t - center
            t1 = max_t - center
            if t1 < xlim[0] or t0 > xlim[1]:
                continue
            lw = 4 if ax_idx == 0 else 6
            ax.plot(
                [t0, t1],
                [0, 0],
                color=colors[sat["box"]],
                linewidth=lw,
                alpha=0.6,
                solid_capstyle="butt",
            )
            edge_lw = 0.5 if ax_idx == 0 else 1.0
            ax.plot(
                [t0, t0], [-0.05, 0.05], color="black", linewidth=edge_lw, alpha=0.5
            )
            ax.plot(
                [t1, t1], [-0.05, 0.05], color="black", linewidth=edge_lw, alpha=0.5
            )

            mid = (t0 + t1) / 2
            if xlim[0] < mid < xlim[1]:
                fs = 5 if ax_idx == 0 else 6
                ax.text(
                    mid,
                    0.12,
                    f"#{pkt_idx}\n{n}evt",
                    fontsize=fs,
                    ha="center",
                    va="bottom",
                    color=colors[sat["box"]],
                    alpha=0.8,
                )

        for pkt_idx, is_sec, met in events:
            t = met - center
            if t < xlim[0] or t > xlim[1]:
                continue
            marker_color = "red" if is_sec else colors[sat["box"]]
            ax.plot(
                t,
                -0.15,
                "|",
                color=marker_color,
                markersize=6 if ax_idx == 1 else 3,
                alpha=0.6,
            )

        ax.axvline(gap_start, color="red", linewidth=0.8, linestyle="--", alpha=0.6)
        ax.axvline(gap_stop, color="red", linewidth=0.8, linestyle="--", alpha=0.6)

        mid_gap = (gap_start + gap_stop) / 2
        ax.annotate(
            f"{gap_ms:.1f}ms",
            xy=(mid_gap, 0.05),
            fontsize=9,
            ha="center",
            va="bottom",
            color="red",
            fontweight="bold",
        )

        ax.set_xlim(*xlim)
        ax.set_ylim(-0.4, 0.5)
        ax.set_yticks([])
        ax.set_xlabel(f"Time - {center:.3f} (s)")
        ax.set_title(subtitle)
        ax.grid(True, axis="x", alpha=0.3)

    fig.suptitle(
        f"#{i + 1} Box {sat['box']} — FIFO Reset gap={gap_ms:.1f}ms  "
        f"(pkt {sat['prev_pkt']}→{sat['next_pkt']})",
        fontsize=12,
        fontweight="bold",
    )
    plt.tight_layout()
    fname = f"{out_dir}/sat_{i + 1:02d}_box{sat['box']}_{gap_ms:.0f}ms.png"
    plt.savefig(fname, dpi=150)
    plt.close()
    print(f"  Saved: {fname}")

print(f"\nDone. {len(sat_intervals)} plots saved to {out_dir}/")
