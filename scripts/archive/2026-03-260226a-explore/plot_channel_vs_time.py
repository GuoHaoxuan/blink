#!/usr/bin/env python3
"""Time vs channel scatter plot for GRB 260226A Box A — overlay 1K and 1B."""

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from astropy.io import fits
from datetime import datetime, timezone, timedelta

MET_EPOCH = datetime(2012, 1, 1, tzinfo=timezone.utc)

def parse_met(s):
    dt = datetime.strptime(s, "%Y-%m-%dT%H:%M:%S").replace(tzinfo=timezone.utc)
    return (dt - MET_EPOCH).total_seconds()

def met_to_utc(met):
    return (MET_EPOCH + timedelta(seconds=met)).strftime("%Y-%m-%dT%H:%M:%S")

trigger = "2026-02-26T10:37:53"
t_ref = parse_met(trigger)
t_lo, t_hi = 20, 25  # zoom range

# ── Load 1B events ──
t_1b, ch_1b = [], []
with open("/tmp/grb260226a_boxa_events.csv") as f:
    for line in f:
        parts = line.strip().split(",")
        if parts[0] == "box" or parts[1] == "SEC":
            continue
        t = float(parts[2]) - t_ref
        if t_lo <= t <= t_hi:
            t_1b.append(t)
            ch_1b.append(int(parts[3]))
t_1b, ch_1b = np.array(t_1b), np.array(ch_1b)

# ── Load 1K events (Box A: det_id 0~5) ──
with fits.open("data/1K/Y202602/20260226-3179/HXMT_20260226T10_HE-Evt_FFFFFF_V1_1K.FITS", memmap=True) as h:
    d = h[1].data
    mask = (d["Det_ID"] >= 0) & (d["Det_ID"] <= 5) & \
           (d["Time"] >= t_ref + t_lo) & (d["Time"] <= t_ref + t_hi)
    t_1k = d["Time"][mask] - t_ref
    # Try Channel first, fallback to PI
    col = "Channel" if "Channel" in d.columns.names else "PI"
    ch_1k = d[col][mask]

print(f"1B: {len(t_1b)} events, 1K: {len(t_1k)} events in T+{t_lo}~{t_hi}")

# ── Single overlay plot ──
fig, ax = plt.subplots(figsize=(24, 10))

# 1K first (gray, behind)
ax.scatter(t_1k, ch_1k, s=0.3, alpha=0.4, color="#AAAAAA", rasterized=True, label=f"1K ({len(t_1k):,})")
# 1B on top (blue)
ax.scatter(t_1b, ch_1b, s=0.3, alpha=0.4, color="#2166AC", rasterized=True, label=f"1B ({len(t_1b):,})")

ax.set_xlabel(f"Time − T₀ (s)", fontsize=14)
ax.set_ylabel("Channel", fontsize=14)
ax.set_title(f"GRB 260226A  Box A  T+{t_lo}~{t_hi}s  1K (gray) vs 1B (blue)    [T₀ = {met_to_utc(t_ref)} UTC]",
             fontsize=15, fontweight="bold")
ax.legend(loc="upper right", fontsize=12, markerscale=20)
ax.set_xlim(t_lo, t_hi)
ax.grid(alpha=0.15)

plt.savefig("grb260226a_boxa_channel_vs_time_zoom.png", dpi=150, bbox_inches="tight")
print("Saved grb260226a_boxa_channel_vs_time_zoom.png")
