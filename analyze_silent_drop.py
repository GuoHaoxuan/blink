"""分析包内事件间隔分布，验证泊松假设，寻找静默丢数信号。

用法:
    python3 analyze_silent_drop.py 200415a
    python3 analyze_silent_drop.py 221009a
    python3 analyze_silent_drop.py 260226a
"""

import subprocess
import os
import sys
import json
import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

GRB_CONFIG = {
    "200415a": {"obs_id": "2020-04-15T08", "center": 261564488.564, "half": 1800.0,
                "label": "GRB 200415A"},
    "221009a": {"obs_id": "2022-10-09T13", "center": 339945422.990, "half": 1800.0,
                "label": "GRB 221009A"},
    "260226a": {"obs_id": "2026-02-26T10", "center": 446726278.000, "half": 1800.0,
                "label": "GRB 260226A"},
}

grb = sys.argv[1].lower() if len(sys.argv) > 1 else "200415a"
cfg = GRB_CONFIG[grb]

BLINK_CLI = "./target/release/blink_cli"
DATA_DIR = os.environ.get("HXMT_1B_DIR", "data/1B")

MCU_READ_RATE = 15797.0  # evt/s


def run_cli(*extra_args):
    env = os.environ.copy()
    env["HXMT_1B_DIR"] = DATA_DIR
    cmd = [BLINK_CLI, cfg["obs_id"]] + list(extra_args)
    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    return result.stdout


CENTER = cfg["center"]
HALF = cfg["half"]

print(f"Analyzing {cfg['label']} ...")
print(f"Dumping events for all boxes...")
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
all_suspects = []
for box in ["A", "B", "C"]:
    events = all_events[box]
    if not events:
        continue

    pkts = defaultdict(list)
    for pkt_idx, met in events:
        pkts[pkt_idx].append(met)

    pkt_stats = []
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
            # Use filtered intervals (< 1ms) for robust lambda estimate,
            # so large gaps don't drag down the rate and cause missed detections
            ivs_filt = intervals[intervals < 1e-3]
            lam = 1.0 / np.mean(ivs_filt) if len(ivs_filt) > 0 else rate
            for j, dt in enumerate(intervals):
                log_p = -lam * dt
                if log_p < -23.0:  # p < 1e-10
                    suspicious.append((pkt_idx, j, dt, lam, log_p))

    print(f"\n=== Box {box} ===")
    print(f"  Total packets: {len(pkt_stats)}")
    n_high = sum(1 for _, _, _, r, _ in pkt_stats if r > MCU_READ_RATE)
    print(f"  High-rate packets (>{MCU_READ_RATE:.0f} evt/s): {n_high}")
    print(f"  Suspicious intervals (p < 1e-10): {len(suspicious)}")
    for pkt_idx, j, dt, rate, log_p in suspicious[:20]:
        print(
            f"    pkt={pkt_idx} evt_gap_idx={j} dt={dt * 1e6:.1f}μs rate={rate:.0f} log10(p)={log_p / np.log(10):.1f}"
        )

    for pkt_idx, j, dt, rate, log_p in suspicious:
        all_suspects.append({
            "box": box, "pkt_idx": pkt_idx, "gap_evt_idx": j,
            "gap_dt_us": round(dt * 1e6, 1), "rate": round(rate, 0),
            "log10_p": round(log_p / np.log(10), 1),
        })

# 保存 suspects 到文件
suspects_file = f"silent_drop_suspects_{grb}.json"
with open(suspects_file, "w") as f:
    json.dump(all_suspects, f, indent=2)
print(f"\nSaved {len(all_suspects)} suspects to {suspects_file}")

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

fig.suptitle(f"{cfg['label']} — Intra-packet interval distribution", fontsize=14, fontweight="bold")
plt.tight_layout()
outfile = f"silent_drop_intervals_{grb}.png"
plt.savefig(outfile, dpi=150)
print(f"\nSaved: {outfile}")
