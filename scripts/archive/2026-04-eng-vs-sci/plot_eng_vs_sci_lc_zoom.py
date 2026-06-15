#!/usr/bin/env python3
"""Plot engineering vs science light curves per detector (18 panels) for GRB 221009A.
5 curves per panel:
  - PHO (red)
  - PHO − CsI (orange)
  - PHO − CsI − Large (dark red)
  - Sci EVT/6 in Length window (blue)
  - Sci EVT/6 full second (cyan dashed)
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
    # --- Engineering ---
    eng_file = f"data/1B/2022/20221009/{eng_code}/HXMT_1B_{eng_code}_20221009T130000_G046601_000_004.fits"
    fe = fits.open(eng_file, memmap=True)
    d = fe["HE_Eng"].data
    offset = d["UTC_Last_Bdc"][0] - d["sTime_Last_Bdc"][0]
    met_eng = d["Time"].astype(float) + offset + MET_CORRECTION
    length_s = d["Length_Time_Cycle"].astype(float) * 16e-6

    # --- Science EVT (box total) ---
    met_evt = []
    with open(sci_csv) as f:
        reader = csv.DictReader(f)
        for r in reader:
            if r["type"] == "EVT":
                met_evt.append(float(r["met"]))
    met_evt = np.sort(np.array(met_evt))

    # Bin science: Length window
    sci_length = np.zeros(len(met_eng))
    # Bin science: full second [met_eng[i], met_eng[i]+1)
    sci_full = np.zeros(len(met_eng))
    for i in range(len(met_eng)):
        t0 = met_eng[i]
        sci_length[i] = np.searchsorted(met_evt, t0 + length_s[i]) - np.searchsorted(met_evt, t0)
        sci_full[i] = np.searchsorted(met_evt, t0 + 1.0) - np.searchsorted(met_evt, t0)

    sci_length_det = sci_length / 6.0
    sci_full_det = sci_full / 6.0

    t_min = (met_eng - TRIGGER_MET) / 60.0

    for row in range(6):
        det_id = det_off + row

        pho_d = d[f"Cnt_PHODet_{det_id}"].astype(float)
        csi_d = d[f"Cnt_CsI_PHODet_{det_id}"].astype(float)
        large_raw = d[f"Cnt_LargeEvt_{det_id}"].astype(float)
        large_d = unwrap_large(pho_d, large_raw)
        eng_pho = pho_d
        eng_all = pho_d - csi_d - large_d

        sel = (t_min >= 10) & (t_min <= 30)

        inner = outer_gs[row, col].subgridspec(2, 1, height_ratios=[3, 1], hspace=0.05)
        ax_lc = fig.add_subplot(inner[0])
        ax_ratio = fig.add_subplot(inner[1], sharex=ax_lc)

        # Light curves - main y axis
        ax_lc.step(t_min[sel], eng_pho[sel], where="post", color="red", lw=0.7, alpha=0.8,
                   label="PHO")
        ax_lc.step(t_min[sel], eng_all[sel], where="post", color="darkred", lw=0.7, alpha=0.9,
                   label="PHO−CsI−Large")
        ax_lc.step(t_min[sel], sci_length_det[sel], where="post", color="blue", lw=0.7, alpha=0.9,
                   label="Sci/6 (Length)")
        ax_lc.step(t_min[sel], sci_full_det[sel], where="post", color="deepskyblue", lw=0.7, alpha=0.8,
                   ls="--", label="Sci/6 (1s)")

        # CsI and Large on same axis
        ax_lc.step(t_min[sel], csi_d[sel], where="post", color="limegreen", lw=0.8,
                   label="CsI")
        ax_lc.step(t_min[sel], large_d[sel], where="post", color="purple", lw=0.8,
                   label="Large")

        ax_lc.set_xlim(10, 30)
        if sel.sum() > 0:
            ax_lc.set_ylim(bottom=0, top=eng_pho[sel].max() * 1.15)
        ax_lc.set_ylabel("Counts", fontsize=8)
        ax_lc.tick_params(labelsize=7, labelbottom=False)
        title = f"Box {box_name} Det {det_id}" if row == 0 else f"Det {det_id}"
        ax_lc.set_title(title, fontsize=9, fontweight="bold", loc="left")
        if row == 0 and col == 0:
            ax_lc.legend(fontsize=5, loc="upper right", ncol=3)

        # Ratios: all eng variants / sci_length_det
        has_data = sci_length_det > 5
        sel_data = has_data & sel
        r_all = np.where(sel_data, eng_all / sci_length_det, np.nan)
        r_pho = np.where(sel_data, eng_pho / sci_length_det, np.nan)

        ax_ratio.step(t_min[sel_data], r_pho[sel_data], where="post", color="red", lw=0.4, alpha=0.6)
        ax_ratio.step(t_min[sel_data], r_all[sel_data], where="post", color="darkred", lw=0.5, alpha=0.9)

        k_val = np.nanmedian(r_all[sel_data]) if sel_data.sum() > 5 else np.nan
        ax_ratio.axhline(k_val, color="darkred", ls=":", lw=0.8)
        ax_ratio.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_ratio.set_ylim(0.9, 1.6)
        ax_ratio.set_ylabel("Eng/Sci", fontsize=7)
        ax_ratio.tick_params(labelsize=7)
        ax_ratio.text(0.98, 0.85, f"k = {k_val:.3f}", transform=ax_ratio.transAxes,
                      fontsize=8, ha="right", va="top", color="darkred")
        if row == 5:
            ax_ratio.set_xlabel("Time since trigger (min)", fontsize=8)

        print(f"Det {det_id:2d} (Box {box_name}): k = {k_val:.3f}")

    fe.close()

fig.suptitle("GRB 221009A: Engineering vs Science per detector (10–30 min)", fontsize=14, y=0.995)
plt.savefig("plots/eng_vs_sci_18det_zoom.png", dpi=150, bbox_inches="tight")
print("\nSaved: plots/eng_vs_sci_18det_zoom.png")
plt.close()
