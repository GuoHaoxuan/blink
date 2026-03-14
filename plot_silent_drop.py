"""静默丢数可疑位置可视化。

三面板布局：
  Panel 1  Overview ±100ms ：包 bar + 事件 tick，红色阴影标注异常间隔
  Panel 2  Zoom ±(span+3ms)：聚焦可疑包，事件 tick 逐个可见
  Panel 3  包内事件间隔柱状图

用法:
    python3 plot_silent_drop.py 200415a
    python3 plot_silent_drop.py 221009a
    python3 plot_silent_drop.py 260226a
"""

import subprocess
import os
import sys
import json
import numpy as np
import matplotlib.pyplot as plt

GRB_CONFIG = {
    "200415a": {"obs_id": "2020-04-15T08", "center": 261564488.564, "label": "GRB 200415A"},
    "221009a": {"obs_id": "2022-10-09T13", "center": 339945422.990, "label": "GRB 221009A"},
    "260226a": {"obs_id": "2026-02-26T10", "center": 446726278.000, "label": "GRB 260226A"},
}

grb = sys.argv[1].lower() if len(sys.argv) > 1 else "200415a"
cfg = GRB_CONFIG[grb]

BLINK_CLI = "./target/release/blink_cli"
OBS_ID = cfg["obs_id"]
DATA_DIR = os.environ.get("HXMT_1B_DIR", "data/1B")
SEARCH_CENTER = cfg["center"]

LOG_P_THRESHOLD = -10.0  # log10(p) < -10 判为异常
colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}

# 从 analyze_silent_drop.py 生成的 JSON 文件读取 suspects
suspects_file = f"silent_drop_suspects_{grb}.json"
if not os.path.exists(suspects_file):
    print(f"Error: {suspects_file} not found. Run analyze_silent_drop.py {grb} first.")
    sys.exit(1)

with open(suspects_file) as f:
    all_suspects = json.load(f)

# 转换为 (box, pkt_idx, gap_evt_idx, gap_dt_us) 元组
suspects = [(s["box"], s["pkt_idx"], s["gap_evt_idx"], s["gap_dt_us"]) for s in all_suspects]
print(f"Loaded {len(suspects)} suspects from {suspects_file}")

if not suspects:
    print("No suspects found, nothing to plot.")
    sys.exit(0)

out_dir = f"silent_drop_plots_{grb}"
os.makedirs(out_dir, exist_ok=True)


# ── CLI 封装 ────────────────────────────────────────────────────────────────

def run_cli(*args):
    env = os.environ.copy()
    env["HXMT_1B_DIR"] = DATA_DIR
    return subprocess.run(
        [BLINK_CLI, OBS_ID] + list(args),
        capture_output=True, text=True, env=env,
    ).stdout


def fetch_packets(center, half_window, box):
    """返回 [(pkt_idx, min_met, max_met, n_events), ...]"""
    text = run_cli("--box", box, "--dump-packets",
                   f"{center:.6f}", f"{half_window:.6f}")
    result = []
    for line in text.splitlines():
        if not line or line.startswith("#") or line.startswith("box,") or line.startswith("SEC,"):
            continue
        p = line.split(",")
        if len(p) < 5 or p[0] != box:
            continue
        try:
            result.append((int(p[1]), float(p[2]), float(p[3]), int(p[4])))
        except ValueError:
            continue
    return result


def fetch_events(center, half_window, box):
    """返回 [(pkt_idx, met), ...]，已按 met 升序排列"""
    text = run_cli("--box", box, "--dump-events",
                   f"{center:.6f}", f"{half_window:.6f}")
    result = []
    for line in text.splitlines():
        if not line or line.startswith("#"):
            continue
        p = line.split(",")
        if len(p) < 6 or p[0] != box:
            continue
        try:
            result.append((int(p[1]), float(p[5])))
        except ValueError:
            continue
    result.sort(key=lambda x: x[1])
    return result


# ── 绘图辅助 ────────────────────────────────────────────────────────────────

def draw_panel(ax, packets, events, center, xlim, box, susp_idx,
               gaps, zoom=False):
    """gaps: list of (gap_start, gap_stop) tuples"""
    c = colors[box]
    lw_norm = 5 if zoom else 3
    lw_susp = 8 if zoom else 5
    tick_sz = 10 if zoom else 5
    label_fs = 6 if zoom else 5

    for gap_start, gap_stop in gaps:
        g0 = gap_start - center
        g1 = gap_stop  - center
        ax.axvspan(max(g0, xlim[0]), min(g1, xlim[1]),
                   color="red", alpha=0.15, zorder=0)
        ax.axvline(g0, color="red", lw=0.9, ls="--", alpha=0.7, zorder=1)
        ax.axvline(g1, color="red", lw=0.9, ls="--", alpha=0.7, zorder=1)
        dt_us = (gap_stop - gap_start) * 1e6
        mid = (g0 + g1) / 2
        if xlim[0] < mid < xlim[1]:
            ax.text(mid, 0.10, f"{dt_us:.0f} μs",
                    ha="center", va="bottom", fontsize=9,
                    color="red", fontweight="bold")

    rendered_pkts: set[int] = set()
    for idx, t0_abs, t1_abs, n in packets:
        span = t1_abs - t0_abs
        if idx != susp_idx and span > 0.020:
            continue
        t0 = t0_abs - center
        t1 = t1_abs - center
        if t1 < xlim[0] or t0 > xlim[1]:
            continue
        rendered_pkts.add(idx)
        is_s = (idx == susp_idx)
        col  = "red" if is_s else c
        lw   = lw_susp if is_s else lw_norm
        alp  = 0.9 if is_s else 0.5
        ax.plot([t0, t1], [0, 0], color=col, lw=lw,
                alpha=alp, solid_capstyle="butt", zorder=2)
        for tx in (t0, t1):
            ax.plot([tx, tx], [-0.07, 0.07], color="black",
                    lw=0.8, alpha=0.35, zorder=3)
        mid_bar = (t0 + t1) / 2
        if xlim[0] < mid_bar < xlim[1]:
            rate = n / span if span > 1e-6 else 0
            ax.text(mid_bar, 0.13,
                    f"#{idx}\n{n}evt\n{span*1e3:.1f}ms\n{rate:.0f}/s",
                    ha="center", va="bottom", fontsize=label_fs,
                    color=col, alpha=0.9)

    for pid, met in events:
        if pid not in rendered_pkts:
            continue
        x = met - center
        if x < xlim[0] or x > xlim[1]:
            continue
        col = "red" if pid == susp_idx else c
        ax.plot(x, -0.15, "|", color=col,
                markersize=tick_sz, markeredgewidth=1.2,
                alpha=0.8, zorder=4)

    for gap_start, gap_stop in gaps:
        for met in (gap_start, gap_stop):
            x = met - center
            if xlim[0] <= x <= xlim[1]:
                ax.plot(x, -0.24, "v", color="red",
                        markersize=8 if zoom else 5, zorder=5)

    ax.set_xlim(*xlim)
    ax.set_ylim(-0.38, 0.42)
    ax.set_yticks([])
    ax.grid(axis="x", alpha=0.3)


# ── 按 (box, pkt_idx) 分组 ────────────────────────────────────────────────────

from collections import OrderedDict

grouped: OrderedDict[tuple[str, int], list[tuple[int, float]]] = OrderedDict()
for box, pkt_idx, gap_evt_idx, dt_us_expect in suspects:
    key = (box, pkt_idx)
    if key not in grouped:
        grouped[key] = []
    grouped[key].append((gap_evt_idx, dt_us_expect))

print(f"{len(suspects)} suspects → {len(grouped)} unique packets")

# ── 主循环 ──────────────────────────────────────────────────────────────────

for gi, ((box, pkt_idx), gap_list) in enumerate(grouped.items()):

    pkts_all = fetch_packets(SEARCH_CENTER, 1800, box)
    info = next((r for r in pkts_all if r[0] == pkt_idx), None)
    if info is None:
        print(f"[{gi+1}] Box {box} pkt {pkt_idx}: not found, skip")
        continue
    _, pkt_min, pkt_max, _ = info
    center = (pkt_min + pkt_max) / 2

    packets = fetch_packets(center, 0.5, box)
    events  = fetch_events(center, 0.5, box)

    susp_evts = sorted(met for pid, met in events if pid == pkt_idx)
    if len(susp_evts) < 2:
        print(f"[{gi+1}] Box {box} pkt {pkt_idx}: too few events, skip")
        continue

    normal_ivs = np.diff(susp_evts)
    normal_ivs_filt = normal_ivs[normal_ivs < 1e-3]
    lam = 1.0 / np.mean(normal_ivs_filt) if len(normal_ivs_filt) > 0 else 1.0

    # Build all gaps for this packet
    gaps = []
    gap_descs = []
    for gap_evt_idx, _ in gap_list:
        gap_i = min(gap_evt_idx, len(susp_evts) - 2)
        gap_start = susp_evts[gap_i]
        gap_stop  = susp_evts[gap_i + 1]
        dt_us_actual = (gap_stop - gap_start) * 1e6
        log_p = -lam * (gap_stop - gap_start) / np.log(10)
        gaps.append((gap_start, gap_stop))
        gap_descs.append(f"evt[{gap_i}→{gap_i+1}] {dt_us_actual:.0f}μs p={log_p:.1f}")

    desc_str = ", ".join(gap_descs)
    print(f"[{gi+1}/{len(grouped)}] Box {box} pkt={pkt_idx}  {len(gaps)} gap(s): {desc_str}")

    fig, axes = plt.subplots(
        3, 1, figsize=(16, 10),
        gridspec_kw={"height_ratios": [1.2, 1.2, 1.5]},
    )

    half_ov = 0.100
    draw_panel(axes[0], packets, events, center,
               (-half_ov, half_ov), box, pkt_idx,
               gaps, zoom=False)
    axes[0].set_xlabel(f"Time − {center:.3f}  (s)")
    axes[0].set_title("Overview  ±100 ms")

    pkt_span = pkt_max - pkt_min
    half_zm  = pkt_span / 2 + 0.003
    draw_panel(axes[1], packets, events, center,
               (-half_zm, half_zm), box, pkt_idx,
               gaps, zoom=True)
    axes[1].set_xlabel(f"Time − {center:.3f}  (s)")
    axes[1].set_title(f"Zoom  ±{half_zm*1e3:.1f} ms  (event ticks visible)")

    ax3 = axes[2]
    intervals = np.diff(susp_evts)
    bar_colors = []
    for iv in intervals:
        lp = -lam * iv / np.log(10)
        bar_colors.append("red" if lp < LOG_P_THRESHOLD else colors[box])
    ax3.bar(range(len(intervals)), intervals * 1e6,
            color=bar_colors, alpha=0.75, width=0.9)
    mean_iv = np.mean(normal_ivs_filt) * 1e6 if len(normal_ivs_filt) > 0 else 0
    ax3.axhline(mean_iv, color="gray", ls="--", lw=1, alpha=0.6)
    ax3.text(len(intervals) * 0.98, mean_iv,
             f"mean = {mean_iv:.1f} μs",
             ha="right", va="bottom", fontsize=8, color="gray")
    for j, iv in enumerate(intervals):
        if iv * 1e6 > mean_iv * 5:
            lp = -lam * iv / np.log(10)
            ax3.annotate(
                f"  {iv*1e6:.0f} μs\n  log₁₀p = {lp:.1f}",
                xy=(j, iv * 1e6),
                xytext=(12, 0), textcoords="offset points",
                fontsize=8, color="red", fontweight="bold",
                arrowprops=dict(arrowstyle="->", color="red", lw=0.8),
            )
    ax3.set_xlabel(f"Event interval index within pkt #{pkt_idx}")
    ax3.set_ylabel("Interval (μs)")
    ax3.set_title(
        f"Intra-packet intervals  —  pkt #{pkt_idx}  "
        f"({len(susp_evts)} events,  λ ≈ {lam:.0f} evt/s)"
    )
    ax3.grid(axis="y", alpha=0.3)

    title_gaps = "  |  ".join(gap_descs)
    fig.suptitle(
        f"#{gi+1}  Box {box}  pkt {pkt_idx}  —  Silent Drop suspect  ({cfg['label']})\n"
        f"{len(gaps)} gap(s): {title_gaps}",
        fontsize=11, fontweight="bold",
    )
    plt.tight_layout()
    fname = f"{out_dir}/sd_{gi+1:02d}_box{box}_pkt{pkt_idx}.png"
    plt.savefig(fname, dpi=150)
    plt.close()
    print(f"  → {fname}")

print(f"\nDone.")
