#!/usr/bin/env python3
"""Plot 1B vs 1K for the SEC congestion zone of GRB 260226A Box C."""

import subprocess, os
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from astropy.io import fits
from datetime import datetime, timezone, timedelta

MET_EPOCH = datetime(2012, 1, 1, tzinfo=timezone.utc)

def parse_met(s):
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%dT%H"]:
        try:
            dt = datetime.strptime(s, fmt).replace(tzinfo=timezone.utc)
            return (dt - MET_EPOCH).total_seconds()
        except ValueError:
            continue
    return float(s)

def met_to_utc(met):
    return (MET_EPOCH + timedelta(seconds=met)).strftime("%Y-%m-%dT%H:%M:%S")

def load_1b(epoch, trigger, before, after, box, max_gap):
    cmd = ["./target/release/blink_cli", "sat", epoch, "--box", box.lower(),
           "solve", trigger, "--before", str(before), "--after", str(after)]
    env = os.environ.copy()
    env["PASS1_ONLY"] = "1"
    env["HXMT_1B_DIR"] = "data/1B"
    env["HXMT_1K_DIR"] = "data/1K"
    env["MAX_SEC_GAP"] = str(max_gap)
    proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
    mets = []
    for line in proc.stdout.strip().split("\n"):
        parts = line.split(",")
        if len(parts) < 4 or parts[0] == "box" or parts[1] == "SEC":
            continue
        mets.append(float(parts[2]))
    return np.array(mets)

def load_1k(fits_path, box, t_ref, before, after):
    lo, hi = {"A": (0, 5), "B": (6, 11), "C": (12, 17)}[box]
    with fits.open(fits_path, memmap=True) as h:
        d = h[1].data
        mask = (d["Det_ID"] >= lo) & (d["Det_ID"] <= hi) & \
               (d["Time"] >= t_ref - before) & (d["Time"] <= t_ref + after)
        return d["Time"][mask].copy()

# ── Parameters ──
epoch = "2026-02-26T10"
trigger = "2026-02-26T10:37:53"
fits_path = "data/1K/Y202602/20260226-3179/HXMT_20260226T10_HE-Evt_FFFFFF_V1_1K.FITS"
box = "C"
before = 1900.0
after = 0.0
bin_width = 1.0

t_ref = parse_met(trigger)

# ── Load data ──
print("Loading 1B confident (MAX_SEC_GAP=1)...")
m_1s = load_1b(epoch, trigger, before, after, box, 1)
print(f"  {len(m_1s)} events")

print("Loading 1B all (MAX_SEC_GAP=999)...")
m_all = load_1b(epoch, trigger, before, after, box, 999)
print(f"  {len(m_all)} events")

print("Loading 1K...")
m_1k = load_1k(fits_path, box, t_ref, before, after)
print(f"  {len(m_1k)} events")

# Greedy-only
set_1s = set(np.round(m_1s, 7))
m_greedy = np.array([m for m in m_all if round(m, 7) not in set_1s])

# ── Histogram ──
edges = np.arange(t_ref - before, t_ref + after + bin_width, bin_width)
x = edges[:-1] - t_ref

r_1k = np.histogram(m_1k, bins=edges)[0] / bin_width
r_1s = np.histogram(m_1s, bins=edges)[0] / bin_width
r_greedy = np.histogram(m_greedy, bins=edges)[0] / bin_width
r_total = r_1s + r_greedy

# ── Plot ──
fig, (ax_lc, ax_res) = plt.subplots(2, 1, figsize=(24, 12), sharex=True,
    gridspec_kw={"height_ratios": [2, 1], "hspace": 0.06})

# 1K
ax_lc.fill_between(x, r_1k, step="post", color="#DDDDDD", alpha=0.9, zorder=1)
ax_lc.step(x, r_1k, where="post", color="#AAAAAA", lw=0.8, zorder=2)

# Greedy (orange)
ax_lc.fill_between(x, r_total, step="post", color="#F4A460", alpha=0.6, zorder=3)

# Confident (blue)
ax_lc.fill_between(x, r_1s, step="post", color="#92C5DE", alpha=0.8, zorder=4)
ax_lc.step(x, r_1s, where="post", color="#2166AC", lw=0.8, zorder=4)

# SEC congestion zone marker
sec_start = 31377285 + 415347184 - t_ref
sec_end = 31378613 + 415347184 - t_ref
ax_lc.axvspan(sec_start, sec_end, alpha=0.08, color='red', zorder=0)

legend_elements = [
    Patch(facecolor="#DDDDDD", edgecolor="#AAAAAA", label=f"1K ({len(m_1k):,})"),
    Patch(facecolor="#92C5DE", edgecolor="#2166AC", label=f"1B confident ({len(m_1s):,})"),
    Patch(facecolor="#F4A460", edgecolor="#D2691E", label=f"1B greedy ({len(m_greedy):,})"),
]
ax_lc.legend(handles=legend_elements, loc="upper right", fontsize=12)
ax_lc.set_ylabel("Count rate (evt/s)", fontsize=14)
ax_lc.set_ylim(bottom=0)
ax_lc.grid(alpha=0.15)

# Residual
residual = r_total - r_1k
res_pos = np.maximum(residual, 0)
res_neg = np.minimum(residual, 0)
ax_res.fill_between(x, res_pos, step="post", color="#2166AC", alpha=0.4, zorder=2)
ax_res.fill_between(x, res_neg, step="post", color="#D6604D", alpha=0.4, zorder=2)
ax_res.step(x, residual, where="post", color="#333333", lw=0.6, zorder=3)
ax_res.axhline(0, color="black", lw=0.5, ls="--", alpha=0.5)
ax_res.set_ylabel("1B − 1K (evt/s)", fontsize=14)
ax_res.grid(alpha=0.15)

ax_res.set_xlabel(f"Time − T₀ (s)    [T₀ = {met_to_utc(t_ref)} UTC]", fontsize=13)
ax_lc.set_xlim(-before, after)

fig.suptitle(f"GRB 260226A  Box C  SEC congestion zone  ({bin_width}s bins)",
             fontsize=15, fontweight="bold")
plt.tight_layout()
plt.savefig("grb260226a_congestion_zone.png", dpi=150, bbox_inches="tight")
print("Saved grb260226a_congestion_zone.png")
