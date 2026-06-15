#!/usr/bin/env python3
"""Plot engineering vs science per detector, full hour.
Uses real per-detector science events (det_id from 1B solve).
Dead time correction: PHO * (Length - Dead) / Length - CsI - Large_unwrapped
"""
import numpy as np
import matplotlib.pyplot as plt
from astropy.io import fits
import csv
from unwrap_large import unwrap_large

TRIGGER_MET = 339945419.0
MET_CORRECTION = 4.0

BOXES = [
    ("A", "0766", "/tmp/221009a_boxA_full.csv", 0),
    ("B", "1009", "/tmp/221009a_boxB_full.csv", 6),
    ("C", "1781", "/tmp/221009a_boxC_full.csv", 12),
]

fig = plt.figure(figsize=(20, 28))
outer_gs = fig.add_gridspec(6, 3, hspace=0.35, wspace=0.2)

for col, (box_name, eng_code, sci_csv, det_off) in enumerate(BOXES):
    eng_file = f"data/1B/2022/20221009/{eng_code}/HXMT_1B_{eng_code}_20221009T130000_G046601_000_004.fits"
    fe = fits.open(eng_file, memmap=True)
    d = fe["HE_Eng"].data
    offset = d["UTC_Last_Bdc"][0] - d["sTime_Last_Bdc"][0]
    met_eng = d["Time"].astype(float) + offset + MET_CORRECTION
    length = d["Length_Time_Cycle"].astype(float)
    length_s = length * 16e-6

    # Read science EVT with det_id
    det_evts = {i: [] for i in range(6)}  # local det 0-5
    with open(sci_csv) as f:
        reader = csv.DictReader(f)
        for r in reader:
            if r["type"] == "EVT":
                local_det = int(r["det_id"])
                det_evts[local_det].append(float(r["met"]))
    for k in det_evts:
        det_evts[k] = np.sort(np.array(det_evts[k]))
        print(f"  Box {box_name} det {k}: {len(det_evts[k])} events")

    t_min = (met_eng - TRIGGER_MET) / 60.0

    for row in range(6):
        det_id = det_off + row
        local_det = row

        pho_d = d[f"Cnt_PHODet_{det_id}"].astype(float)
        csi_d = d[f"Cnt_CsI_PHODet_{det_id}"].astype(float)
        dead_d = d[f"DeadTime_PHODet_{det_id}"].astype(float)
        large_d = unwrap_large(pho_d, d[f"Cnt_LargeEvt_{det_id}"].astype(float))

        live_frac = (length - dead_d) / length
        eng_d = pho_d * live_frac - csi_d - large_d

        # Per-detector science events binned into Length windows
        met_det = det_evts[local_det]
        sci_d = np.zeros(len(met_eng))
        for i in range(len(met_eng)):
            t0 = met_eng[i]
            t1 = t0 + length_s[i]
            sci_d[i] = np.searchsorted(met_det, t1) - np.searchsorted(met_det, t0)

        has_data = sci_d > 5
        ratio = np.where(has_data, eng_d / sci_d, np.nan)
        quiet = has_data & (eng_d < 1500) & (eng_d > 100)
        k_val = np.nanmedian(ratio[quiet]) if quiet.sum() > 20 else np.nan

        inner = outer_gs[row, col].subgridspec(2, 1, height_ratios=[3, 1], hspace=0.05)
        ax_lc = fig.add_subplot(inner[0])
        ax_ratio = fig.add_subplot(inner[1], sharex=ax_lc)

        ax_lc.step(t_min, eng_d, where="post", color="red", lw=0.7, alpha=0.9,
                   label="PHO×(1−dead)−CsI−Large")
        ax_lc.step(t_min, sci_d, where="post", color="blue", lw=0.7, alpha=0.9,
                   label="Sci EVT (per det)")
        ax_lc.set_ylim(bottom=0)
        ax_lc.set_ylabel("Counts", fontsize=8)
        ax_lc.tick_params(labelsize=7, labelbottom=False)
        title = f"Box {box_name} Det {det_id}" if row == 0 else f"Det {det_id}"
        ax_lc.set_title(title, fontsize=9, fontweight="bold", loc="left")
        if row == 0 and col == 0:
            ax_lc.legend(fontsize=7, loc="upper right")

        ax_ratio.step(t_min[has_data], ratio[has_data], where="post", color="black", lw=0.5)
        ax_ratio.axhline(k_val, color="red", ls=":", lw=0.8)
        ax_ratio.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_ratio.set_ylim(0.9, 1.4)
        ax_ratio.set_ylabel("Corr/Sci", fontsize=7)
        ax_ratio.tick_params(labelsize=7)
        ax_ratio.text(0.98, 0.85, f"k = {k_val:.3f}", transform=ax_ratio.transAxes,
                      fontsize=8, ha="right", va="top", color="red")
        if row == 5:
            ax_ratio.set_xlabel("Time since trigger (min)", fontsize=8)

        print(f"Det {det_id:2d} (Box {box_name}): k = {k_val:.3f}")

    fe.close()

fig.suptitle("GRB 221009A: Corrected Eng vs Science per detector (real det_id)",
             fontsize=14, y=0.995)
plt.savefig("plots/eng_vs_sci_18det_full.png", dpi=150, bbox_inches="tight")
print("\nSaved: plots/eng_vs_sci_18det_full.png")
plt.close()
