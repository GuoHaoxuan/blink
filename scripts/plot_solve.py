#!/usr/bin/env python3
"""Plot light curve from `blink_cli sat <EPOCH> solve` output.

Usage:
    blink_cli sat 2020-04-15T08 solve > events.csv
    python3 scripts/plot_solve.py events.csv
    python3 scripts/plot_solve.py events.csv --center 261564488.564 --window 60 --bin 0.1
    python3 scripts/plot_solve.py events.csv --bin 1.0 -o lightcurve.png
"""

import argparse
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from datetime import datetime, timezone, timedelta

# HXMT MET epoch
MET_EPOCH = datetime(2012, 1, 1, tzinfo=timezone.utc)


def parse_met_or_utc(s):
    """Parse a time string as MET (float) or UTC (datetime). Returns MET float."""
    try:
        return float(s)
    except ValueError:
        pass
    # Try ISO formats
    for fmt in ["%Y-%m-%dT%H:%M:%S.%f", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%dT%H"]:
        try:
            dt = datetime.strptime(s, fmt).replace(tzinfo=timezone.utc)
            met = (dt - MET_EPOCH).total_seconds()
            print(f"  UTC {s} -> MET {met:.6f}")
            return met
        except ValueError:
            continue
    raise ValueError(f"Cannot parse time: {s}. Use MET number or UTC (e.g. 2020-04-15T08:48:08)")


def load_events(path):
    """Load solve CSV, return dict of box -> array of MET times."""
    boxes = {}
    with open(path) as f:
        header = f.readline()  # box,type,met,channel,pkt_idx,evt_idx
        for line in f:
            parts = line.strip().split(",")
            if len(parts) < 3:
                continue
            box, typ, met = parts[0], parts[1], float(parts[2])
            if typ != "EVT":
                continue
            if box not in boxes:
                boxes[box] = []
            boxes[box].append(met)
    for box in boxes:
        boxes[box] = np.array(boxes[box])
    return boxes


def plot_lightcurve(boxes, center=None, window=None, bin_width=1.0, output=None):
    # Determine time range
    all_times = np.concatenate(list(boxes.values()))
    if center is not None and window is not None:
        t_min = center - window
        t_max = center + window
    else:
        t_min = all_times.min()
        t_max = all_times.max()

    # Reference time for x-axis
    t_ref = center if center else (t_min + t_max) / 2

    edges = np.arange(t_min, t_max + bin_width, bin_width)
    colors = {"A": "#2166AC", "B": "#D6604D", "C": "#1B7837"}
    fill_colors = {"A": "#92C5DE", "B": "#F4A582", "C": "#A6D96A"}

    box_names = sorted(boxes.keys())
    fig, axes = plt.subplots(len(box_names), 1, figsize=(16, 4 * len(box_names)),
                              sharex=True)
    if len(box_names) == 1:
        axes = [axes]

    for i, box in enumerate(box_names):
        ax = axes[i]
        times = boxes[box]
        mask = (times >= t_min) & (times < t_max)
        counts, _ = np.histogram(times[mask], bins=edges)
        rates = counts / bin_width

        plot_edges = edges - t_ref
        ax.stairs(rates, plot_edges, fill=True, color=fill_colors.get(box, "#AAAAAA"),
                  alpha=0.6)
        ax.stairs(rates, plot_edges, color=colors.get(box, "#333333"), lw=0.5)

        ax.set_ylabel(f"Box {box}\n(evt/s)", fontsize=12)
        ax.grid(alpha=0.15)
        ax.set_xlim(plot_edges[0], plot_edges[-1])
        ax.set_ylim(bottom=0)

        n_total = mask.sum()
        ax.text(0.98, 0.95, f"n = {n_total:,}", transform=ax.transAxes,
                ha="right", va="top", fontsize=10,
                bbox=dict(boxstyle="round", facecolor="white", alpha=0.8))

    axes[-1].set_xlabel(f"Time - {t_ref:.3f} (s)", fontsize=12)
    fig.suptitle(f"Light Curve ({bin_width}s bins)", fontsize=14, fontweight="bold")
    plt.tight_layout()

    out = output or "lightcurve.png"
    plt.savefig(out, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"Saved: {out}")


def main():
    parser = argparse.ArgumentParser(description="Plot light curve from solve CSV")
    parser.add_argument("csv", help="Input CSV from blink_cli sat solve")
    parser.add_argument("--center", type=str, default=None,
                        help="Center time (MET number or UTC, e.g. 261564488.564 or 2020-04-15T08:48:08)")
    parser.add_argument("--window", type=float, default=None,
                        help="Half window size (seconds)")
    parser.add_argument("--bin", type=float, default=1.0,
                        help="Bin width (seconds)")
    parser.add_argument("-o", "--output", default=None,
                        help="Output file (default: lightcurve.png)")
    args = parser.parse_args()

    boxes = load_events(args.csv)
    print(f"Loaded: {', '.join(f'Box {k}: {len(v):,} events' for k, v in sorted(boxes.items()))}")

    center = parse_met_or_utc(args.center) if args.center else None
    plot_lightcurve(boxes, center=center, window=args.window,
                    bin_width=args.bin, output=args.output)


if __name__ == "__main__":
    main()
