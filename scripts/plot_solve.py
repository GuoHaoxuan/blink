#!/usr/bin/env python3
"""Plot 1B light curve with 1K as reference background.

Calls Rust CLI via subprocess (no intermediate files).

Usage:
    python3 scripts/plot_solve.py 2020-04-15T08 --trigger 2020-04-15T08:48:08 --before 5 --after 10
    python3 scripts/plot_solve.py 2022-10-09T13 --trigger 2022-10-09T13:37:02 --before 100 --after 600 --bin 1
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
    """Call blink_cli sat <subcmd> and parse output via pipe."""
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


def plot_lightcurve(b1, k1, trigger_met=None, bin_width=1.0, output=None, epoch=""):
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
    n_panels = len(box_names) + 1

    fig, axes = plt.subplots(n_panels, 1, figsize=(16, 3.2 * n_panels), sharex=True)
    if n_panels == 1:
        axes = [axes]

    for i, box in enumerate(box_names):
        ax = axes[i]

        # 1K as background reference
        if box in k1 and len(k1[box]) > 0:
            counts_k, _ = np.histogram(k1[box], bins=edges)
            rates_k = counts_k / bin_width
            ax.stairs(rates_k, plot_edges, fill=True, color="#DDDDDD", alpha=0.8,
                      label=f"1K (n={len(k1[box]):,})", zorder=1)
            ax.stairs(rates_k, plot_edges, color="#AAAAAA", lw=0.4, zorder=2)

        # 1B on top
        if box in b1 and len(b1[box]) > 0:
            counts_b, _ = np.histogram(b1[box], bins=edges)
            rates_b = counts_b / bin_width
            ax.stairs(rates_b, plot_edges, fill=True, color=fill_1b.get(box, "#88BBDD"),
                      alpha=0.5, label=f"1B (n={len(b1[box]):,})", zorder=3)
            ax.stairs(rates_b, plot_edges, color=colors_1b.get(box, "#336699"),
                      lw=0.5, zorder=4)

        ax.set_ylabel(f"Box {box}\n(evt/s)", fontsize=12)
        ax.legend(loc="upper right", fontsize=9, framealpha=0.9)
        ax.grid(alpha=0.12)
        ax.set_ylim(bottom=0)

    # Merged panel
    ax = axes[-1]
    if k1:
        merged_k = np.concatenate(list(k1.values()))
        counts_k, _ = np.histogram(merged_k, bins=edges)
        rates_k = counts_k / bin_width
        ax.stairs(rates_k, plot_edges, fill=True, color="#DDDDDD", alpha=0.8,
                  label=f"1K (n={len(merged_k):,})", zorder=1)
        ax.stairs(rates_k, plot_edges, color="#AAAAAA", lw=0.4, zorder=2)

    if b1:
        merged_b = np.concatenate(list(b1.values()))
        counts_b, _ = np.histogram(merged_b, bins=edges)
        rates_b = counts_b / bin_width
        ax.stairs(rates_b, plot_edges, fill=True, color="#B0C4DE", alpha=0.5,
                  label=f"1B (n={len(merged_b):,})", zorder=3)
        ax.stairs(rates_b, plot_edges, color="#4A6A8A", lw=0.5, zorder=4)

    ax.set_ylabel("A+B+C\n(evt/s)", fontsize=12)
    ax.legend(loc="upper right", fontsize=9, framealpha=0.9)
    ax.grid(alpha=0.12)
    ax.set_ylim(bottom=0)

    if trigger_met:
        utc_str = met_to_utc(t_ref)
        axes[-1].set_xlabel(
            f"Time - T₀ (s)    [T₀ = {utc_str} UTC = MET {t_ref:.3f}]", fontsize=11)
    else:
        axes[-1].set_xlabel(f"Time - {t_ref:.3f} (s)", fontsize=11)

    axes[0].set_xlim(plot_edges[0], plot_edges[-1])
    fig.suptitle(f"{epoch}  Light Curve ({bin_width}s bins)  [1B + 1K ref]",
                 fontsize=14, fontweight="bold")
    plt.tight_layout()

    out = output or "lightcurve.png"
    plt.savefig(out, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"Saved: {out}")


def main():
    parser = argparse.ArgumentParser(description="Plot 1B light curve with 1K reference")
    parser.add_argument("epoch", help="Epoch (e.g. 2020-04-15T08)")
    parser.add_argument("--trigger", type=str, default=None,
                        help="Trigger time (MET or UTC)")
    parser.add_argument("--before", type=float, default=10.0)
    parser.add_argument("--after", type=float, default=100.0)
    parser.add_argument("--bin", type=float, default=1.0, help="Bin width (seconds)")
    parser.add_argument("--box", type=str, default=None, dest="box_filter")
    parser.add_argument("--no-1k", action="store_true", help="Skip 1K reference")
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

    if not b1:
        print("No 1B events found!", file=sys.stderr)
        sys.exit(1)

    n_1b = sum(len(v) for v in b1.values())
    n_1k = sum(len(v) for v in k1.values())
    print(f"1B: {n_1b:,} events  |  1K: {n_1k:,} events")

    trigger_met = parse_met_or_utc(args.trigger) if args.trigger else None
    plot_lightcurve(b1, k1, trigger_met=trigger_met, bin_width=args.bin,
                    output=args.output, epoch=args.epoch)


if __name__ == "__main__":
    main()
