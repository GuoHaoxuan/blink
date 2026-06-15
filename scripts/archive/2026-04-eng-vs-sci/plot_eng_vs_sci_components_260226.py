#!/usr/bin/env python3
"""GRB 260226A: all components + multiplicative vs additive comparison, full hour."""
import numpy as np
import matplotlib.pyplot as plt
from astropy.io import fits
import csv
from unwrap_large import unwrap_large

TRIGGER_MET = 446726278.0  # 10:37:55 UTC
MET_CORRECTION = 4.0

BOXES = [
    ("A", "0766", "/tmp/260226_boxA_full.csv", 0),
    ("B", "1009", "/tmp/260226_boxB_full.csv", 6),
    ("C", "1781", "/tmp/260226_boxC_full.csv", 12),
]

fig = plt.figure(figsize=(20, 40))
outer_gs = fig.add_gridspec(6, 3, hspace=0.4, wspace=0.2)

for col, (box_name, eng_code, sci_csv, det_off) in enumerate(BOXES):
    eng_file = f"data/1B/2026/20260226/{eng_code}/HXMT_1B_{eng_code}_20260226T100000_G076262_000_004.fits"
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

        met_det = det_evts[row]
        sci_d = np.zeros(len(met_eng))
        sci_full = np.zeros(len(met_eng))
        for i in range(len(met_eng)):
            t0 = met_eng[i]
            sci_d[i] = np.searchsorted(met_det, t0 + length_s[i]) - np.searchsorted(met_det, t0)
            sci_full[i] = np.searchsorted(met_det, t0 + 1.0) - np.searchsorted(met_det, t0)

        has_data = (sci_d > 5) & (dead_d > 0)
        quiet = has_data & (t_min >= -5) & (t_min <= -1)

        # Multiplicative
        live_frac = (length - dead_d) / length
        eng_mult = pho_d * live_frac - csi_d - large_d
        k_mult = np.nanmedian(eng_mult[quiet] / sci_d[quiet]) if quiet.sum() > 20 else np.nan
        ratio_mult = np.where(has_data, eng_mult / k_mult / sci_d, np.nan)

        # Additive
        k_add = np.nanmedian((eng_raw[quiet] - sci_d[quiet]) / dead_d[quiet]) if quiet.sum() > 20 else 0.1
        eng_add = eng_raw - k_add * dead_d
        ratio_add = np.where(has_data, eng_add / sci_d, np.nan)

        # 4 sub-panels: components, mult ratio, add ratio
        inner = outer_gs[row, col].subgridspec(4, 1, height_ratios=[4, 1, 1, 1], hspace=0.05)
        ax_lc = fig.add_subplot(inner[0])
        ax_raw = fig.add_subplot(inner[1], sharex=ax_lc)
        ax_m = fig.add_subplot(inner[2], sharex=ax_lc)
        ax_a = fig.add_subplot(inner[3], sharex=ax_lc)

        # Components
        ax_lc.step(t_min, pho_d, where="post", color="red", lw=0.6, alpha=0.8, label="PHO")
        ax_lc.step(t_min, eng_raw, where="post", color="darkred", lw=0.6, alpha=0.9, label="PHO−CsI−Large")
        ax_lc.step(t_min, sci_d, where="post", color="blue", lw=0.7, alpha=0.9, label="Sci (Length)")
        ax_lc.step(t_min, sci_full, where="post", color="deepskyblue", lw=0.6, alpha=0.7, label="Sci (1s)")
        ax_lc.step(t_min, csi_d, where="post", color="limegreen", lw=0.7, label="CsI")
        ax_lc.step(t_min, large_d, where="post", color="purple", lw=0.7, label="Large")
        ax_lc.step(t_min, dead_d, where="post", color="orange", lw=0.6, alpha=0.7, label="Dead")
        ax_lc.set_xlim(-1, 3)
        ax_lc.set_ylim(bottom=0)
        ax_lc.set_ylabel("Counts", fontsize=7)
        ax_lc.tick_params(labelsize=6, labelbottom=False)
        title = f"Box {box_name} Det {det_id}" if row == 0 else f"Det {det_id}"
        ax_lc.set_title(title, fontsize=9, fontweight="bold", loc="left")
        ax_lc.legend(fontsize=4.5, loc="upper left", ncol=4)

        # Raw ratio (no dead time correction)
        r_raw = np.where(has_data, eng_raw / sci_d, np.nan)
        ax_raw.step(t_min[has_data], r_raw[has_data], where="post", color="gray", lw=0.5)
        ax_raw.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_raw.set_ylim(0.85, 1.5)
        ax_raw.set_ylabel("Raw", fontsize=6, color="gray")
        ax_raw.tick_params(labelsize=5, labelbottom=False)
        k_raw = np.nanmedian(r_raw[quiet]) if quiet.sum() > 5 else np.nan
        ax_raw.axhline(k_raw, color="gray", ls=":", lw=0.6)
        ax_raw.text(0.98, 0.85, f"k={k_raw:.3f}", transform=ax_raw.transAxes,
                    fontsize=5, ha="right", va="top", color="gray")

        # Mult ratio
        ax_m.step(t_min[has_data], ratio_mult[has_data], where="post", color="darkred", lw=0.5)
        ax_m.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_m.set_ylim(0.85, 1.15)
        ax_m.set_ylabel("Mult", fontsize=6, color="darkred")
        ax_m.tick_params(labelsize=5, labelbottom=False)
        mad_m = np.nanmedian(np.abs(ratio_mult[has_data] - 1))
        ax_m.text(0.98, 0.85, f"k={k_mult:.3f} MAD={mad_m*100:.1f}%",
                  transform=ax_m.transAxes, fontsize=5, ha="right", va="top", color="darkred")

        # Add ratio
        ax_a.step(t_min[has_data], ratio_add[has_data], where="post", color="darkblue", lw=0.5)
        ax_a.axhline(1.0, color="gray", ls=":", lw=0.4)
        ax_a.set_ylim(0.85, 1.15)
        ax_a.set_ylabel("Add", fontsize=6, color="darkblue")
        ax_a.tick_params(labelsize=5)
        mad_a = np.nanmedian(np.abs(ratio_add[has_data] - 1))
        ax_a.text(0.98, 0.85, f"k={k_add:.3f} MAD={mad_a*100:.1f}%",
                  transform=ax_a.transAxes, fontsize=5, ha="right", va="top", color="darkblue")

        if row == 5:
            ax_a.set_xlabel("Time since trigger (min)", fontsize=7)

        print(f"Det {det_id:2d}: Raw k={k_raw:.3f} | Mult k={k_mult:.3f} MAD={mad_m*100:.1f}% | Add k={k_add:.3f} MAD={mad_a*100:.1f}%")

    fe.close()

fig.suptitle("GRB 260226A: Components + Mult vs Add correction (burst region, real det_id)",
             fontsize=14, y=0.998)
plt.savefig("plots/eng_vs_sci_components_260226.png", dpi=150, bbox_inches="tight")
print("\nSaved: plots/eng_vs_sci_components_260226.png")
plt.close()
