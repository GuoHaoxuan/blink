#!/usr/bin/env python3
"""Fig 4 (recovery_221009a): 3-panel light-curve comparison for GRB
221009A across the burst peak T_0+70 to T_0+100 s.

Each panel (Box A, B, C):
  • 1B LIS time reconstruction (this work, no gap-fill applied) — coloured
  • 1K standard pipeline                                         — grey

Headline: in Box C between T_0+82 and T_0+90 s the 1K pipeline reports
zero events for ~8 s during the brightest phase of the burst, whereas
the 1B LIS reconstruction places events at amplitudes consistent with
the surrounding 1B rate and with the simultaneous A/B light curves.

Output: figures/f4_solve_vs_1k_221009a.pdf in the paper repo.
"""
import os
import sys
import subprocess
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from datetime import datetime, timezone

EPOCH    = "2022-10-09T13"
TRIGGER  = "2022-10-09T13:17:00"
BEFORE   = 10.0
AFTER    = 290.0
BIN      = 1.0
ZOOM_LO  = 250.0
ZOOM_HI  = 272.0  # extends past 1B's burst end (T+269.9) to capture
                  # 1K's wrap-shifted tail in Box A (1K max ~T+271.0)
SHADE_C_LO = 263.0   # Box C: 1K=0 for ~7 s while 1B places ~12 k evt/s
SHADE_C_HI = 270.0
SHADE_A_LO = 251.0   # Box A: 1K wrap mis-select (+1.05 s = 1 ptime wrap)
SHADE_A_HI = 253.0
MET_EPOCH = datetime(2012, 1, 1, tzinfo=timezone.utc)


def parse_met(s):
    dt = datetime.strptime(s, "%Y-%m-%dT%H:%M:%S").replace(tzinfo=timezone.utc)
    return (dt - MET_EPOCH).total_seconds()


def run_cli(source):
    """source: '1b' (LIS reconstruction) or '1k' (standard pipeline)."""
    cmd = ["./target/release/blink", "sat", "extract",
           "--source", source,
           "--before", str(BEFORE), "--after", str(AFTER), TRIGGER]
    env = os.environ.copy()
    env.setdefault("HXMT_1B_DIR", "data/1B")
    env.setdefault("HXMT_1K_DIR", "data/1K")
    print(f"  $ {' '.join(cmd)}", file=sys.stderr)
    proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
    boxes = {}
    for line in proc.stdout.strip().split("\n"):
        parts = line.split(",")
        if len(parts) < 3 or parts[0] == "box":
            continue
        box, typ, met = parts[0], parts[1], float(parts[2])
        if typ == "SEC":
            continue
        boxes.setdefault(box, []).append(met)
    return {b: np.array(v) for b, v in boxes.items()}


def main():
    t_ref = parse_met(TRIGGER)
    print("Loading 1B (LIS-reconstructed)...", file=sys.stderr)
    b1 = run_cli("1b")
    print("Loading 1K...", file=sys.stderr)
    k1 = run_cli("1k")

    edges = np.arange(t_ref + ZOOM_LO, t_ref + ZOOM_HI + BIN / 2, BIN)
    x = edges[:-1] - t_ref

    fig, axes = plt.subplots(3, 1, figsize=(10, 8), sharex=True,
                             gridspec_kw={"hspace": 0.10})
    colors_1b = {"A": "#1F4E9E", "B": "#E89028", "C": "#1B7837"}
    fill_1b   = {"A": "#A6C8E0", "B": "#F6CB8A", "C": "#A6D96A"}

    for ax, box in zip(axes, ["A", "B", "C"]):
        evt_1b = b1.get(box, np.array([]))
        evt_1k = k1.get(box, np.array([]))
        win_1b = evt_1b[(evt_1b >= edges[0]) & (evt_1b < edges[-1])]
        win_1k = evt_1k[(evt_1k >= edges[0]) & (evt_1k < edges[-1])]
        r_1k = np.histogram(win_1k, bins=edges)[0] / BIN
        r_1b = np.histogram(win_1b, bins=edges)[0] / BIN

        if box == "C":
            ax.axvspan(SHADE_C_LO, SHADE_C_HI, color="#D62728", alpha=0.10,
                       zorder=0)
        if box == "A":
            ax.axvspan(SHADE_A_LO, SHADE_A_HI, color="#D62728", alpha=0.10,
                       zorder=0)

        ax.fill_between(x, r_1k, step="post", color="#CCCCCC",
                        alpha=0.85, edgecolor="none", zorder=1)
        ax.step(x, r_1k, where="post", color="#666666", lw=1.0,
                label=f"1K standard pipeline ({len(win_1k):,})", zorder=2)
        ax.fill_between(x, r_1b, step="post", color=fill_1b[box],
                        alpha=0.45, edgecolor="none", zorder=3)
        ax.step(x, r_1b, where="post", color=colors_1b[box], lw=1.3,
                label=f"1B LIS reconstruction ({len(win_1b):,})", zorder=4)

        ax.set_ylabel(f"Box {box}\nevt/s", fontsize=12)
        ax.legend(loc="lower right", fontsize=10, framealpha=0.95)
        ax.grid(alpha=0.15)
        ax.set_ylim(bottom=0, top=15500)
        ax.tick_params(labelsize=10)

        if box == "C":
            ax.text((SHADE_C_LO + SHADE_C_HI) / 2, 14200,
                    r"1K $=$ 0 evt/s for ${\sim}7$ s",
                    ha="center", va="top", fontsize=11, color="#7A1212",
                    fontweight="bold")
        if box == "A":
            ax.text((SHADE_A_LO + SHADE_A_HI) / 2, 14200,
                    r"1K $+1$ wrap shift" + "\n" + r"(${\sim}1.05$ s late)",
                    ha="center", va="top", fontsize=9.5, color="#7A1212",
                    fontweight="bold")

    axes[-1].set_xlabel(
        r"Time $-$ $T_0$ (s)    "
        f"[$T_0=$ {TRIGGER} UTC; bin $=$ {BIN:.0f} s]",
        fontsize=12)
    axes[0].set_xlim(ZOOM_LO, ZOOM_HI)
    fig.suptitle("GRB 221009A: 1B LIS reconstruction vs 1K pipeline, "
                 r"$T_0+%.0f$ to $T_0+%.0f$ s" % (ZOOM_LO, ZOOM_HI),
                 fontsize=13, fontweight="bold", y=0.995)
    plt.tight_layout()
    out_pdf = "/Users/skyair/Developer/ihep/paper-hxmt-saturation/figures/f4_solve_vs_1k_221009a.pdf"
    out_png = "/Users/skyair/Developer/ihep/blink/plots/fig4_solve_vs_1k_221009a_preview.png"
    plt.savefig(out_pdf)
    plt.savefig(out_png, dpi=150)
    plt.close()
    print(f"Saved: {out_pdf}")
    print(f"Saved: {out_png}")


if __name__ == "__main__":
    main()
