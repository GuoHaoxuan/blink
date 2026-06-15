#!/usr/bin/env python3
"""Compare multiplicative vs additive dead time correction, per detector.
Real per-detector science events. Full hour.

Each detector: light curve + two ratio panels (mult and add).
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

fig = plt.figure(figsize=(20, 36))
outer_gs = fig.add_gridspec(6, 3, hspace=0.4, wspace=0.2)

for col, (box_name, eng_code, sci_csv, det_off) in enumerate(BOXES):
    eng_file = f"data/1B/2022/20221009/{eng_code}/HXMT_1B_{eng_code}_20221009T130000_G046601_000_004.fits"
    fe = fits.open(eng_file, memmap=True)
    d = fe["HE_Eng"].data
    offset = d["UTC_Last_Bdc"][0] - d["sTime_Last_Bdc"][0]
    met_eng = d["Time"].astype(float) + offset + MET_CORRECTION
    length = d["Length_Time_Cycle"].astype(float)
    length_s = length * 16e-6

    det_evts = {i: [] for i in range(6)}
    with open(sci_csv) as f:
        reader = csv.DictReader(f)
        for r in reader:
            if r["type"] == "EVT":
                det_evts[int(r["det_id"])].append(float(r["met"]))
    for k in det_evts:
        det_evts[k] = np.sort(np.array(det_evts[k]))

    t_min = (met_eng - TRIGGER_MET) / 60.0

    for row in range(6):
        det_id = det_off + row

        pho_d = d[f"Cnt_PHODet_{det_id}"].astype(float)
        csi_d = d[f"Cnt_CsI_PHODet_{det_id}"].astype(float)
        dead_d = d[f"DeadTime_PHODet_{det_id}"].astype(float)
        large_d = unwrap_large(pho_d, d[f"Cnt_LargeEvt_{det_id}"].astype(float))
        eng_raw = pho_d - csi_d - large_d

        # Per-detector science
        met_det = det_evts[row]
        sci_d = np.zeros(len(met_eng))
        for i in range(len(met_eng)):
            t0 = met_eng[i]
            t1 = t0 + length_s[i]
            sci_d[i] = np.searchsorted(met_det, t1) - np.searchsorted(met_det, t0)

        has_data = (sci_d > 5) & (dead_d > 0)
        quiet = has_data & (eng_raw > 100) & (eng_raw < 1500)

        # Multiplicative: PHO*(1-D/L) - CsI - Large
        live_frac = (length - dead_d) / length
        eng_mult = pho_d * live_frac - csi_d - large_d
        k_mult = np.nanmedian(eng_mult[quiet] / sci_d[quiet]) if quiet.sum() > 20 else np.nan
        ratio_mult = np.where(has_data, eng_mult / k_mult / sci_d, np.nan)

        # Additive: PHO - CsI - Large - k*Dead
        k_add = np.nanmedian((eng_raw[quiet] - sci_d[quiet]) / dead_d[quiet]) if quiet.sum() > 20 else 0.1
        eng_add = eng_raw - k_add * dead_d
        ratio_add = np.where(has_data, eng_add / sci_d, np.nan)

        # 3 sub-panels: lc, ratio_mult, ratio_add
        inner = outer_gs[row, col].subgridspec(3, 1, height_ratios=[3, 1, 1], hspace=0.05)
        ax_lc = fig.add_subplot(inner[0])
        ax_m = fig.add_subplot(inner[1], sharex=ax_lc)
        ax_a = fig.add_subplot(inner[2], sharex=ax_lc)

        # Light curve
        ax_lc.step(t_min, eng_raw, where="post", color="gray", lw=0.5, alpha=0.5,
                   label="PHO−CsI−Large")
        ax_lc.step(t_min, sci_d, where="post", color="blue", lw=0.7, alpha=0.9,
                   label="Sci EVT")
        ax_lc.set_ylim(bottom=0)
        ax_lc.set_ylabel("Counts", fontsize=7)
        ax_lc.tick_params(labelsize=6, labelbottom=False)
        title = f"Box {box_name} Det {det_id}" if row == 0 else f"Det {det_id}"
        ax_lc.set_title(title, fontsize=9, fontweight="bold", loc="left")
        if row == 0 and col == 0:
            ax_lc.legend(fontsize=6, loc="upper right")

        # Multiplicative ratio
        ax_m.step(t_min[has_data], ratio_mult[has_data], where="post", color="darkred", lw=0.5)
        ax_m.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_m.set_ylim(0.85, 1.15)
        ax_m.set_ylabel("Mult", fontsize=6, color="darkred")
        ax_m.tick_params(labelsize=6, labelbottom=False)
        mad_m = np.nanmedian(np.abs(ratio_mult[has_data] - 1))
        ax_m.text(0.98, 0.85, f"k={k_mult:.3f} MAD={mad_m*100:.1f}%",
                  transform=ax_m.transAxes, fontsize=6, ha="right", va="top", color="darkred")

        # Additive ratio
        ax_a.step(t_min[has_data], ratio_add[has_data], where="post", color="darkblue", lw=0.5)
        ax_a.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_a.set_ylim(0.85, 1.15)
        ax_a.set_ylabel("Add", fontsize=6, color="darkblue")
        ax_a.tick_params(labelsize=6)
        mad_a = np.nanmedian(np.abs(ratio_add[has_data] - 1))
        ax_a.text(0.98, 0.85, f"k={k_add:.3f} MAD={mad_a*100:.1f}%",
                  transform=ax_a.transAxes, fontsize=6, ha="right", va="top", color="darkblue")

        if row == 5:
            ax_a.set_xlabel("Time since trigger (min)", fontsize=7)

        print(f"Det {det_id:2d}: Mult k={k_mult:.3f} MAD={mad_m*100:.2f}%  |  Add k={k_add:.3f} MAD={mad_a*100:.2f}%")

    fe.close()

fig.suptitle("Multiplicative vs Additive dead time correction (real det_id)", fontsize=14, y=0.998)
plt.savefig("plots/eng_vs_sci_compare.png", dpi=150, bbox_inches="tight")
print("\nSaved: plots/eng_vs_sci_compare.png")
plt.close()
