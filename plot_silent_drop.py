"""静默丢数可疑位置可视化。

三面板布局：
  Panel 1  Overview ±100ms ：包 bar + 事件 tick，红色阴影标注异常间隔
  Panel 2  Zoom ±(span+3ms)：聚焦可疑包，事件 tick 逐个可见
  Panel 3  包内事件间隔柱状图

用法: HXMT_1B_DIR=data/1B python3 plot_silent_drop.py
"""

import subprocess
import os
import numpy as np
import matplotlib.pyplot as plt

BLINK_CLI = "./target/release/blink_cli"
OBS_ID    = "2022-10-09T13"
DATA_DIR  = os.environ.get("HXMT_1B_DIR", "/Users/skyair/Developer/ihep/blink/data/1B")

# 用于第一步大范围查找目标包的参考中心
SEARCH_CENTER = 339945304.0

LOG_P_THRESHOLD = -10.0  # log10(p) < -10 判为异常
colors = {"A": "#1f77b4", "B": "#ff7f0e", "C": "#2ca02c"}

# (box, pkt_idx, gap_evt_idx, gap_dt_us)
# gap_evt_idx：可疑间隔在 时间排序后 的位置，即 sorted_events[gap_evt_idx] 到 sorted_events[gap_evt_idx+1]
suspects = [
    ("A", 38896, 107, 5764.0),
    ("B", 39288, 107, 5886.0),
    ("B", 39645,   0,  454.0),
    ("C", 37420, 107, 3552.0),
    ("C", 39011,   0, 1802.0),
]

os.makedirs("silent_drop_plots", exist_ok=True)


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
               gap_start, gap_stop, zoom=False):
    """
    在一个 Axes 上画：
      - 每个 CCSDS 包：y=0 的水平条（跨度 >20ms 的饱和包跳过，避免遮挡）
      - 每个事件：y=-0.15 处的 | tick，颜色按所属包
      - 可疑包和它的事件用红色
      - gap 区域红色阴影 + 边界虚线 + 标注
    """
    c = colors[box]
    lw_norm = 5 if zoom else 3          # 正常包 bar 粗细
    lw_susp = 8 if zoom else 5          # 可疑包 bar 粗细
    tick_sz = 10 if zoom else 5         # 事件 tick 大小（markersize）
    label_fs = 6 if zoom else 5

    # ── 红色阴影标注 gap ──────────────────────────────────────────────────
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

    # ── 包 bar ───────────────────────────────────────────────────────────
    # 同时记录"实际画出了 bar 的包"，事件 tick 只画这些包的事件
    rendered_pkts: set[int] = set()
    for idx, t0_abs, t1_abs, n in packets:
        span = t1_abs - t0_abs
        # 跨度过大（饱和巨包）且不是可疑包 → 跳过，bar 和 tick 都不画
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

    # ── 事件 tick：只画 bar 已渲染的包的事件 ──────────────────────────────
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

    # ── gap 端点特别标注（倒三角） ────────────────────────────────────────
    for met in (gap_start, gap_stop):
        x = met - center
        if xlim[0] <= x <= xlim[1]:
            ax.plot(x, -0.24, "v", color="red",
                    markersize=8 if zoom else 5, zorder=5)

    ax.set_xlim(*xlim)
    ax.set_ylim(-0.38, 0.42)
    ax.set_yticks([])
    ax.grid(axis="x", alpha=0.3)


# ── 主循环 ──────────────────────────────────────────────────────────────────

for si, (box, pkt_idx, gap_evt_idx, dt_us_expect) in enumerate(suspects):

    # 1. 大窗口找目标包的 MET 范围
    pkts_all = fetch_packets(SEARCH_CENTER, 1800, box)
    info = next((r for r in pkts_all if r[0] == pkt_idx), None)
    if info is None:
        print(f"[{si+1}] Box {box} pkt {pkt_idx}: not found, skip")
        continue
    _, pkt_min, pkt_max, _ = info
    center = (pkt_min + pkt_max) / 2     # 图的时间原点

    # 2. 以 center 为中心拉取 ±0.5s 的包和事件
    packets = fetch_packets(center, 0.5, box)
    events  = fetch_events(center, 0.5, box)

    # 3. 可疑包内事件（时间排序）
    susp_evts = sorted(met for pid, met in events if pid == pkt_idx)
    if len(susp_evts) < 2:
        print(f"[{si+1}] Box {box} pkt {pkt_idx}: too few events, skip")
        continue

    gap_i = min(gap_evt_idx, len(susp_evts) - 2)
    gap_start = susp_evts[gap_i]
    gap_stop  = susp_evts[gap_i + 1]
    dt_us_actual = (gap_stop - gap_start) * 1e6

    # 估算可疑包事件率（排除 gap 本身）
    normal_ivs = np.diff(susp_evts)
    normal_ivs_filt = normal_ivs[normal_ivs < 1e-3]   # 排除 gap
    lam = 1.0 / np.mean(normal_ivs_filt) if len(normal_ivs_filt) > 0 else 1.0
    log_p = -lam * (gap_stop - gap_start) / np.log(10)

    print(f"[{si+1}/{len(suspects)}] Box {box} pkt={pkt_idx}  "
          f"gap_evt={gap_evt_idx}  dt={dt_us_actual:.1f}μs  log10p={log_p:.1f}")

    # 4. 画图
    fig, axes = plt.subplots(
        3, 1, figsize=(16, 10),
        gridspec_kw={"height_ratios": [1.2, 1.2, 1.5]},
    )

    # Panel 1：Overview ±100ms
    half_ov = 0.100
    draw_panel(axes[0], packets, events, center,
               (-half_ov, half_ov), box, pkt_idx,
               gap_start, gap_stop, zoom=False)
    axes[0].set_xlabel(f"Time − {center:.3f}  (s)")
    axes[0].set_title("Overview  ±100 ms")

    # Panel 2：Zoom — 以可疑包跨度为基准，两侧各留 3ms
    pkt_span = pkt_max - pkt_min
    half_zm  = pkt_span / 2 + 0.003
    draw_panel(axes[1], packets, events, center,
               (-half_zm, half_zm), box, pkt_idx,
               gap_start, gap_stop, zoom=True)
    axes[1].set_xlabel(f"Time − {center:.3f}  (s)")
    axes[1].set_title(f"Zoom  ±{half_zm*1e3:.1f} ms  (event ticks visible)")

    # Panel 3：包内事件间隔柱状图
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

    fig.suptitle(
        f"#{si+1}  Box {box}  pkt {pkt_idx}  —  Silent Drop suspect\n"
        f"gap between evt[{gap_i}] and evt[{gap_i+1}]:  "
        f"{dt_us_actual:.0f} μs,  log₁₀p = {log_p:.1f}",
        fontsize=12, fontweight="bold",
    )
    plt.tight_layout()
    fname = f"silent_drop_plots/sd_{si+1:02d}_box{box}_pkt{pkt_idx}.png"
    plt.savefig(fname, dpi=150)
    plt.close()
    print(f"  → {fname}")

print(f"\nDone.")
