#!/usr/bin/env python3
"""Plot engineering vs science per detector with ACD fraction overlay, full hour."""
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

# --- Read 1K data for ACD info ---
f1k = fits.open("data/1K/Y202210/20221009-1943/HXMT_20221009T13_HE-Evt_FFFFFF_V1_1K.FITS", memmap=True)
d1k = f1k[1].data
met_1k = d1k["Time"]
det_1k = d1k["Det_ID"]
acd_1k = d1k["ACD"]  # bool
print(f"1K: {len(met_1k)} events, ACD rate = {acd_1k.sum()/len(acd_1k)*100:.1f}%")

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

    # Science EVT from 1B
    met_evt = []
    with open(sci_csv) as f:
        reader = csv.DictReader(f)
        for r in reader:
            if r["type"] == "EVT":
                met_evt.append(float(r["met"]))
    met_evt = np.sort(np.array(met_evt))

    sci_box = np.zeros(len(met_eng))
    for i in range(len(met_eng)):
        t0 = met_eng[i]
        t1 = t0 + length_s[i]
        sci_box[i] = np.searchsorted(met_evt, t1) - np.searchsorted(met_evt, t0)
    sci_per_det = sci_box / 6.0

    t_min = (met_eng - TRIGGER_MET) / 60.0

    for row in range(6):
        det_id = det_off + row

        pho_d = d[f"Cnt_PHODet_{det_id}"].astype(float)
        csi_d = d[f"Cnt_CsI_PHODet_{det_id}"].astype(float)
        dead_d = d[f"DeadTime_PHODet_{det_id}"].astype(float)
        large_d = unwrap_large(pho_d, d[f"Cnt_LargeEvt_{det_id}"].astype(float))

        live_frac = (length - dead_d) / length
        eng_d = pho_d * live_frac - csi_d - large_d

        has_data = sci_per_det > 5
        ratio = np.where(has_data, eng_d / sci_per_det, np.nan)
        quiet = has_data & (eng_d < 1500) & (eng_d > 100)
        k_val = np.nanmedian(ratio[quiet]) if quiet.sum() > 20 else np.nan

        # ACD fraction per second for this detector (from 1K)
        det_mask = det_1k == det_id
        met_det = met_1k[det_mask]
        acd_det = acd_1k[det_mask]
        idx_sort = np.argsort(met_det)
        met_det = met_det[idx_sort]
        acd_det = acd_det[idx_sort]

        acd_frac = np.zeros(len(met_eng))
        total_det = np.zeros(len(met_eng))
        for i in range(len(met_eng)):
            t0 = met_eng[i]
            t1 = t0 + length_s[i]
            lo = np.searchsorted(met_det, t0)
            hi = np.searchsorted(met_det, t1)
            n = hi - lo
            if n > 10:
                acd_frac[i] = acd_det[lo:hi].sum() / n
                total_det[i] = n
            else:
                acd_frac[i] = np.nan

        # --- Plot ---
        inner = outer_gs[row, col].subgridspec(2, 1, height_ratios=[3, 1], hspace=0.05)
        ax_lc = fig.add_subplot(inner[0])
        ax_ratio = fig.add_subplot(inner[1], sharex=ax_lc)

        ax_lc.step(t_min, eng_d, where="post", color="red", lw=0.7, alpha=0.9,
                   label="PHO×(1−dead)−CsI−Large")
        ax_lc.step(t_min, sci_per_det, where="post", color="blue", lw=0.7, alpha=0.9,
                   label="Sci EVT/6")
        ax_lc.set_ylim(bottom=0)
        ax_lc.set_ylabel("Counts", fontsize=8)
        ax_lc.tick_params(labelsize=7, labelbottom=False)
        title = f"Box {box_name} Det {det_id}" if row == 0 else f"Det {det_id}"
        ax_lc.set_title(title, fontsize=9, fontweight="bold", loc="left")
        if row == 0 and col == 0:
            ax_lc.legend(fontsize=6, loc="upper right")

        # Ratio + ACD fraction
        ax_ratio.step(t_min[has_data], ratio[has_data], where="post", color="black", lw=0.5,
                      label=f"Corr/Sci (k={k_val:.3f})")
        ax_ratio.axhline(k_val, color="red", ls=":", lw=0.8)
        ax_ratio.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_ratio.set_ylim(0.8, 1.6)
        ax_ratio.set_ylabel("Corr/Sci", fontsize=7)
        ax_ratio.tick_params(labelsize=7)

        # ACD on secondary axis
        ax_acd = ax_ratio.twinx()
        valid_acd = ~np.isnan(acd_frac)
        ax_acd.step(t_min[valid_acd], acd_frac[valid_acd], where="post",
                    color="green", lw=0.5, alpha=0.7, label="ACD frac")
        ax_acd.set_ylim(0, 1.1)
        ax_acd.set_ylabel("ACD%", fontsize=6, color="green")
        ax_acd.tick_params(labelsize=6, colors="green")

        if row == 0 and col == 0:
            h1, l1 = ax_ratio.get_legend_handles_labels()
            h2, l2 = ax_acd.get_legend_handles_labels()
            ax_ratio.legend(h1+h2, l1+l2, fontsize=5, loc="upper left")

        if row == 5:
            ax_ratio.set_xlabel("Time since trigger (min)", fontsize=8)

        print(f"Det {det_id:2d} (Box {box_name}): k = {k_val:.3f}")

    fe.close()

f1k.close()
fig.suptitle("GRB 221009A: Corrected Eng vs Science + ACD fraction (full hour)",
             fontsize=14, y=0.995)
plt.savefig("plots/eng_vs_sci_acd_18det.png", dpi=150, bbox_inches="tight")
print("\nSaved: plots/eng_vs_sci_acd_18det.png")
plt.close()
