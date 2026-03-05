#!/usr/bin/env python3
"""
GRB260226A: engineering vs 1K light curve overlay (time-aligned, raw counts).
"""
import numpy as np
import matplotlib.pyplot as plt
from astropy.io import fits

BASE = "data/1B/2026/20260226"
eng_info = {
    "A": ("0766", range(0, 6)),
    "B": ("1009", range(6, 12)),
    "C": ("1781", range(12, 18)),
}

# ── Read eng data per box ──────────────────────────────────────────────────
box_met, box_cnt = {}, {}
for box, (apid, dets) in eng_info.items():
    fpath = f"{BASE}/{apid}/HXMT_1B_{apid}_20260226T100000_G076262_000_004.fits"
    with fits.open(fpath) as hdul:
        hdu = hdul["HE_Eng"]
        stime = hdu.data["Time"].astype(np.float64)
        offset = float(hdu.data["UTC_Last_Bdc"][0]) - float(hdu.data["sTime_Last_Bdc"][0])
        met = stime + offset
        cnt = np.zeros(len(stime), dtype=np.int64)
        for d in dets:
            cnt += hdu.data[f"Cnt_PHODet_{d}"].astype(np.int64)
        box_met[box] = met
        box_cnt[box] = cnt
        print(f"Box {box}: offset={offset:.0f}, met[0]={met[0]:.0f}")

# Total: interpolate B, C onto A's time grid
met = box_met["A"]
total_eng = box_cnt["A"].copy()
for b in ["B", "C"]:
    total_eng += np.interp(met, box_met[b], box_cnt[b]).astype(np.int64)

t0 = met[np.argmax(total_eng)]
print(f"T0={t0:.0f}, peak={total_eng.max()}")

# ── Read 1K CSV (10ms) → rebin to 1s ──────────────────────────────────────
m10, c10 = [], []
with open("hist_260226a_1k.csv") as f:
    for line in f:
        if line.startswith("#") or not line.strip():
            continue
        a, b = line.split(",")
        m10.append(float(a)); c10.append(int(b))
m10 = np.array(m10); c10 = np.array(c10)

# Histogram into integer-aligned 1s bins
edges = np.arange(np.floor(m10[0]), np.ceil(m10[-1]) + 1, 1.0)
c1s, _ = np.histogram(m10, bins=edges, weights=c10)
m1s = 0.5 * (edges[:-1] + edges[1:])  # centers at .5
# Shift by -0.5 so centers match integer METs (same as eng)
m1s -= 0.5

dt_e = met - t0
dt_k = m1s - t0

# ── Plot ──────────────────────────────────────────────────────────────────
fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 9), sharex=False,
                                gridspec_kw={"height_ratios": [3, 1.5]})

ax1.step(dt_e, total_eng, where="mid", color="k", lw=1.5, label="Eng (Cnt_PHODet)")
ax1.step(dt_k, c1s,       where="mid", color="#9b59b6", lw=1.5, alpha=0.85, label="1K Science")
ax1.set_ylabel("Counts / s", fontsize=13)
ax1.set_title("GRB 260226A — Eng vs 1K (raw counts, aligned)", fontsize=15, fontweight="bold")
ax1.legend(fontsize=11, loc="upper right")
ax1.grid(alpha=0.3)
ax1.set_xlim(dt_k[0]-1, dt_k[-1]+1)

ax2.step(dt_e, total_eng, where="mid", color="k", lw=1.5, label="Eng")
ax2.step(dt_k, c1s,       where="mid", color="#9b59b6", lw=1.5, alpha=0.85, label="1K")
ax2.set_ylabel("Counts / s", fontsize=13)
ax2.set_xlabel(f"Time − T0 (s)  [T0 = MET {t0:.0f}]", fontsize=13)
ax2.set_xlim(-10, 30)
ax2.grid(alpha=0.3)
ax2.legend(fontsize=10, loc="upper right")

plt.tight_layout()
plt.savefig("lightcurve_grb260226a_eng.png", dpi=150, bbox_inches="tight")
print("Saved lightcurve_grb260226a_eng.png")
