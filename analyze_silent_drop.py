"""分析包内事件间隔分布，验证泊松假设，寻找静默丢数信号。

思路：
- 泊松过程的事件间隔服从指数分布 P(Δt) = λ·exp(-λΔt)
- 如果包内某个间隔 Δt 的生存概率 P(X > Δt) = exp(-λΔt) 极小，说明中间可能丢了事件
- 只看事件率 > 15797 evt/s 的包（低于此值 FIFO 不可能满）

用法:
    HXMT_1B_DIR=data/1B cargo run --release -- 2020-04-15T08 --dump-events CENTER HALF_WINDOW --box A > events_A.csv
    python3 analyze_silent_drop.py
"""

import subprocess
import os
import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

BLINK_CLI = "./target/release/blink_cli"
OBS_ID = "2022-10-09T13"
DATA_DIR = os.environ.get("HXMT_1B_DIR", "/Users/skyair/Developer/ihep/blink/data/1B")

MCU_READ_RATE = 15797.0  # evt/s


def run_cli(*extra_args):
    env = os.environ.copy()
    env["HXMT_1B_DIR"] = DATA_DIR
    cmd = [BLINK_CLI, OBS_ID] + list(extra_args)
    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    return result.stdout


# 导出 burst 附近 ±2s 的所有事例
CENTER = 339945304.0
HALF = 1800.0

print("Dumping events for all boxes...")
all_events = {}
for box in ["A", "B", "C"]:
    text = run_cli("--box", box, "--dump-events", f"{CENTER:.6f}", f"{HALF:.6f}")
    events = []
    for line in text.strip().split("\n"):
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split(",")
        if len(parts) < 6:
            continue
        if parts[0] != box:
            continue
        pkt_idx = int(parts[1])
        met = float(parts[5])
        events.append((pkt_idx, met))
    all_events[box] = events
    print(f"  Box {box}: {len(events)} events")

# 按包分组，计算包内事件间隔
for box in ["A", "B", "C"]:
    events = all_events[box]
    if not events:
        continue

    pkts = defaultdict(list)
    for pkt_idx, met in events:
        pkts[pkt_idx].append(met)

    # 对每个包：排序事件时间，计算间隔，估算事件率
    pkt_stats = []
    all_intervals_normal = []
    all_intervals_high = []
    suspicious = []

    for pkt_idx in sorted(pkts.keys()):
        times = sorted(pkts[pkt_idx])
        n = len(times)
        if n < 2:
            continue
        span = times[-1] - times[0]
        if span < 1e-9:
            continue
        rate = n / span
        intervals = np.diff(times)

        pkt_stats.append((pkt_idx, n, span, rate, intervals))

        if rate > MCU_READ_RATE:
            all_intervals_high.extend(intervals)
            # 用包内平均间隔估算 λ
            lam = rate
            for j, dt in enumerate(intervals):
                # 生存概率 P(X > dt) = exp(-λ * dt)
                log_p = -lam * dt
                if log_p < -23.0:  # p < 1e-10
                    suspicious.append((pkt_idx, j, dt, rate, log_p))
        else:
            all_intervals_normal.extend(intervals)

    print(f"\n=== Box {box} ===")
    print(f"  Total packets: {len(pkt_stats)}")
    n_high = sum(1 for _, _, _, r, _ in pkt_stats if r > MCU_READ_RATE)
    print(f"  High-rate packets (>{MCU_READ_RATE:.0f} evt/s): {n_high}")
    print(f"  Suspicious intervals (p < 1e-6): {len(suspicious)}")
    for pkt_idx, j, dt, rate, log_p in suspicious[:20]:
        print(
            f"    pkt={pkt_idx} evt_gap_idx={j} dt={dt * 1e6:.1f}μs rate={rate:.0f} log10(p)={log_p / np.log(10):.1f}"
        )

# 画图：高事件率包的间隔分布 vs 指数分布拟合
fig, axes = plt.subplots(1, 3, figsize=(18, 5))
for ax_idx, box in enumerate(["A", "B", "C"]):
    ax = axes[ax_idx]
    events = all_events[box]
    pkts = defaultdict(list)
    for pkt_idx, met in events:
        pkts[pkt_idx].append(met)

    intervals_high = []
    for pkt_idx in sorted(pkts.keys()):
        times = sorted(pkts[pkt_idx])
        if len(times) < 2:
            continue
        span = times[-1] - times[0]
        if span < 1e-9:
            continue
        rate = len(times) / span
        if rate > MCU_READ_RATE:
            intervals_high.extend(np.diff(times))

    if not intervals_high:
        ax.set_title(f"Box {box}: no high-rate packets")
        continue

    intervals_high = np.array(intervals_high)
    lam = 1.0 / np.mean(intervals_high)

    ax.hist(intervals_high * 1e6, bins=100, density=True, alpha=0.7, label="observed")
    x = np.linspace(0, np.percentile(intervals_high, 99.9) * 1e6, 200)
    ax.plot(
        x,
        lam * 1e-6 * np.exp(-lam * x * 1e-6),
        "r-",
        linewidth=2,
        label=f"exp(λ={lam:.0f}/s)",
    )
    ax.set_xlabel("Event interval (μs)")
    ax.set_ylabel("Density")
    ax.set_title(f"Box {box} — high-rate packets ({len(intervals_high)} intervals)")
    ax.legend()
    ax.set_yscale("log")

plt.tight_layout()
plt.savefig("silent_drop_intervals.png", dpi=150)
print("\nSaved: silent_drop_intervals.png")
