#!/usr/bin/env python3
"""Plot 1B light curve with 1K reference and saturation intervals.

Calls Rust CLI via subprocess (no intermediate files).

Usage:
    python3 scripts/plot_solve.py 2020-04-15T08 --trigger 2020-04-15T08:48:08 --before 5 --after 10
    python3 scripts/plot_solve.py 2022-10-09T13 --trigger 2022-10-09T13:17:02 --before 50 --after 600 --bin 1
"""

import argparse
import subprocess
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from datetime import datetime, timezone, timedelta
import os
import sys

MET_EPOCH = datetime(2012, 1, 1, tzinfo=timezone.utc)


def parse_met_or_utc(s):
    try:
        return float(s)
    except ValueError:
        pass
    for fmt in ["%Y-%m-%dT%H:%M:%S.%f", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%dT%H"]:
        try:
            dt = datetime.strptime(s, fmt).replace(tzinfo=timezone.utc)
            return (dt - MET_EPOCH).total_seconds()
        except ValueError:
            continue
    raise ValueError(f"Cannot parse time: {s}")


def met_to_utc(met):
    dt = MET_EPOCH + timedelta(seconds=met)
    return dt.strftime("%Y-%m-%dT%H:%M:%S")


def run_cli(epoch, subcmd, trigger=None, before=10, after=100, box_filter=None):
    """Call blink_cli sat <subcmd> and parse event output via pipe."""
    cmd = ["./target/release/blink_cli", "sat", epoch]
    if box_filter:
        cmd.extend(["--box", box_filter])
    cmd.append(subcmd)
    if trigger:
        cmd.append(trigger)
        cmd.extend(["--before", str(before), "--after", str(after)])

    env = os.environ.copy()
    if "HXMT_1B_DIR" not in env:
        env["HXMT_1B_DIR"] = "data/1B"
    if "HXMT_1K_DIR" not in env:
        env["HXMT_1K_DIR"] = "data/1K"

    print(f"  Running: {' '.join(cmd)}", file=sys.stderr)
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, env=env)

    boxes = {}
    for line in proc.stdout:
        parts = line.strip().split(",")
        if len(parts) < 3 or parts[0] == "box":
            continue
        box, typ, met = parts[0], parts[1], float(parts[2])
        if typ != "EVT":
            continue
        if box not in boxes:
            boxes[box] = []
        boxes[box].append(met)

    stderr = proc.stderr.read()
    proc.wait()
    if stderr:
        for line in stderr.strip().split("\n"):
            print(f"  {line}", file=sys.stderr)

    for box in boxes:
        boxes[box] = np.array(boxes[box])
    return boxes


def run_detect(epoch, trigger=None, before=10, after=100):
    """Call blink_cli sat detect and parse saturation intervals."""
    cmd = ["./target/release/blink_cli", "sat", epoch, "detect"]
    if trigger:
        cmd.append(trigger)
        cmd.extend(["--before", str(before), "--after", str(after)])

    env = os.environ.copy()
    if "HXMT_1B_DIR" not in env:
        env["HXMT_1B_DIR"] = "data/1B"

    print(f"  Running: {' '.join(cmd)}", file=sys.stderr)
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, env=env)

    fifo_resets = {}   # box -> list of (start, stop)
    silent_drops = {}  # box -> list of (start, stop)

    for line in proc.stdout:
        parts = line.strip().split(",")
        if len(parts) < 5 or parts[0] == "box":
            continue
        box, typ = parts[0], parts[1]
        start, stop = float(parts[2]), float(parts[3])
        if typ == "FifoReset":
            fifo_resets.setdefault(box, []).append((start, stop))
        elif typ == "SilentDrop":
            silent_drops.setdefault(box, []).append((start, stop))

    stderr = proc.stderr.read()
    proc.wait()
    if stderr:
        for line in stderr.strip().split("\n"):
            print(f"  {line}", file=sys.stderr)

    return fifo_resets, silent_drops


def plot_lightcurve(b1, k1, fifo_resets, silent_drops,
                    trigger_met=None, bin_width=1.0, output=None, epoch=""):
    all_b1 = np.concatenate(list(b1.values())) if b1 else np.array([])
    all_k1 = np.concatenate(list(k1.values())) if k1 else np.array([])
    all_times = np.concatenate([all_b1, all_k1])
    t_min = all_times.min()
    t_max = all_times.max()
    t_ref = trigger_met if trigger_met else (t_min + t_max) / 2

    edges = np.arange(t_min, t_max + bin_width, bin_width)
    plot_edges = edges - t_ref

    colors_1b = {"A": "#2166AC", "B": "#D6604D", "C": "#1B7837"}
    fill_1b = {"A": "#92C5DE", "B": "#F4A582", "C": "#A6D96A"}

    box_names = sorted(set(list(b1.keys()) + list(k1.keys())))
    # Each box: light curve + saturation strip below it, then merged + strip
    # height ratios: [lc, sat, lc, sat, lc, sat, lc_merged, sat_merged]
    n_groups = len(box_names) + 1  # 3 boxes + merged
    height_ratios = []
    for _ in range(n_groups):
        height_ratios.extend([1, 0.12])  # lc tall, sat strip thin

    fig, all_axes = plt.subplots(n_groups * 2, 1,
                                  figsize=(16, 3.5 * n_groups),
                                  sharex=True,
                                  gridspec_kw={"height_ratios": height_ratios,
                                               "hspace": 0.05})

    def draw_lc(ax, times_1k, times_1b, fill_color, line_color):
        if times_1k is not None and len(times_1k) > 0:
            counts_k, _ = np.histogram(times_1k, bins=edges)
            rates_k = counts_k / bin_width
            ax.fill_between(plot_edges[:-1], rates_k, step="post", color="#DDDDDD",
                      alpha=0.8, edgecolor="none", linewidth=0, zorder=1)
            ax.step(plot_edges[:-1], rates_k, where="post", color="#AAAAAA",
                    lw=0.5, label=f"1K (n={len(times_1k):,})", zorder=2)
        if times_1b is not None and len(times_1b) > 0:
            counts_b, _ = np.histogram(times_1b, bins=edges)
            rates_b = counts_b / bin_width
            ax.fill_between(plot_edges[:-1], rates_b, step="post",
                      color=fill_color, alpha=0.5,
                      edgecolor="none", linewidth=0, zorder=3)
            ax.step(plot_edges[:-1], rates_b, where="post",
                    color=line_color, lw=0.5,
                    label=f"1B (n={len(times_1b):,})", zorder=4)

    def draw_sat_strip(ax, box, color):
        """Draw saturation indicators as a thin strip."""
        for start, stop in fifo_resets.get(box, []):
            ax.axvspan(start - t_ref, stop - t_ref, color=color, alpha=0.7)
        for start, stop in silent_drops.get(box, []):
            mid = (start + stop) / 2 - t_ref
            ax.axvline(mid, color="purple", alpha=0.8, lw=0.8)
        ax.set_ylim(0, 1)
        ax.set_yticks([])
        ax.patch.set_facecolor("#F8F8F8")
        n_fr = len(fifo_resets.get(box, []))
        n_sd = len(silent_drops.get(box, []))
        if n_fr or n_sd:
            label = f"FR:{n_fr}"
            if n_sd:
                label += f" SD:{n_sd}"
            ax.text(0.99, 0.5, label, transform=ax.transAxes,
                    ha="right", va="center", fontsize=7, color="#666666")

    # Per-box panels
    for i, box in enumerate(box_names):
        ax_lc = all_axes[i * 2]
        ax_sat = all_axes[i * 2 + 1]

        draw_lc(ax_lc, k1.get(box), b1.get(box),
                fill_1b.get(box, "#88BBDD"),
                colors_1b.get(box, "#336699"))
        ax_lc.set_ylabel(f"Box {box}\n(evt/s)", fontsize=12)
        ax_lc.legend(loc="upper right", fontsize=8, framealpha=0.9)
        ax_lc.grid(alpha=0.12)
        ax_lc.set_ylim(bottom=0)

        draw_sat_strip(ax_sat, box, colors_1b.get(box, "#888888"))

    # Merged panel
    idx_merged = len(box_names) * 2
    ax_lc = all_axes[idx_merged]
    ax_sat = all_axes[idx_merged + 1]

    merged_k = np.concatenate(list(k1.values())) if k1 else None
    merged_b = np.concatenate(list(b1.values())) if b1 else None
    draw_lc(ax_lc, merged_k, merged_b, "#B0C4DE", "#4A6A8A")
    ax_lc.set_ylabel("A+B+C\n(evt/s)", fontsize=12)
    ax_lc.legend(loc="upper right", fontsize=8, framealpha=0.9)
    ax_lc.grid(alpha=0.12)
    ax_lc.set_ylim(bottom=0)

    # Merged saturation: show all boxes
    for box in box_names:
        for start, stop in fifo_resets.get(box, []):
            ax_sat.axvspan(start - t_ref, stop - t_ref,
                          color=colors_1b.get(box, "#888888"), alpha=0.5)
        for start, stop in silent_drops.get(box, []):
            mid = (start + stop) / 2 - t_ref
            ax_sat.axvline(mid, color="purple", alpha=0.6, lw=0.6)
    ax_sat.set_ylim(0, 1)
    ax_sat.set_yticks([])
    ax_sat.patch.set_facecolor("#F8F8F8")
    n_fr = sum(len(v) for v in fifo_resets.values())
    n_sd = sum(len(v) for v in silent_drops.values())
    ax_sat.text(0.99, 0.5, f"FR:{n_fr} SD:{n_sd}", transform=ax_sat.transAxes,
                ha="right", va="center", fontsize=7, color="#666666")

    if trigger_met:
        utc_str = met_to_utc(t_ref)
        ax_sat.set_xlabel(
            f"Time - T₀ (s)    [T₀ = {utc_str} UTC = MET {t_ref:.3f}]", fontsize=11)
    else:
        ax_sat.set_xlabel(f"Time - {t_ref:.3f} (s)", fontsize=11)

    all_axes[0].set_xlim(plot_edges[0], plot_edges[-1])
    fig.suptitle(f"{epoch}  Light Curve ({bin_width}s bins)", fontsize=14, fontweight="bold")
    plt.tight_layout()

    out = output or "lightcurve.png"
    plt.savefig(out, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"Saved: {out}")


def main():
    parser = argparse.ArgumentParser(description="Plot 1B light curve with 1K ref + saturation")
    parser.add_argument("epoch", help="Epoch (e.g. 2020-04-15T08)")
    parser.add_argument("--trigger", type=str, default=None,
                        help="Trigger time (MET or UTC)")
    parser.add_argument("--before", type=float, default=10.0)
    parser.add_argument("--after", type=float, default=100.0)
    parser.add_argument("--bin", type=float, default=1.0, help="Bin width (seconds)")
    parser.add_argument("--box", type=str, default=None, dest="box_filter")
    parser.add_argument("--no-1k", action="store_true", help="Skip 1K reference")
    parser.add_argument("--no-sat", action="store_true", help="Skip saturation detection")
    parser.add_argument("-o", "--output", default=None)
    args = parser.parse_args()

    print("Loading 1B data...")
    b1 = run_cli(args.epoch, "solve", trigger=args.trigger,
                 before=args.before, after=args.after, box_filter=args.box_filter)

    k1 = {}
    if not args.no_1k:
        print("Loading 1K data...")
        try:
            k1 = run_cli(args.epoch, "solve1k", trigger=args.trigger,
                         before=args.before, after=args.after, box_filter=args.box_filter)
        except Exception as e:
            print(f"  1K not available: {e}", file=sys.stderr)

    fifo_resets, silent_drops = {}, {}
    if not args.no_sat:
        print("Detecting saturation...")
        try:
            fifo_resets, silent_drops = run_detect(
                args.epoch, trigger=args.trigger,
                before=args.before, after=args.after)
        except Exception as e:
            print(f"  Detection failed: {e}", file=sys.stderr)

    if not b1:
        print("No 1B events found!", file=sys.stderr)
        sys.exit(1)

    n_1b = sum(len(v) for v in b1.values())
    n_1k = sum(len(v) for v in k1.values())
    n_fr = sum(len(v) for v in fifo_resets.values())
    n_sd = sum(len(v) for v in silent_drops.values())
    print(f"1B: {n_1b:,}  |  1K: {n_1k:,}  |  FIFO Reset: {n_fr}  |  Silent Drop: {n_sd}")

    trigger_met = parse_met_or_utc(args.trigger) if args.trigger else None
    plot_lightcurve(b1, k1, fifo_resets, silent_drops,
                    trigger_met=trigger_met, bin_width=args.bin,
                    output=args.output, epoch=args.epoch)


if __name__ == "__main__":
    main()
