#!/usr/bin/env python3
"""Scan engineering data vs 1K science events to compute k and beta across all observations.

Output: CSV with per-hour k, beta, Dead/PHO for Box A/B/C.
"""
import numpy as np
from astropy.io import fits
import glob, os, sys, csv

BASE_1B = "/hxmtfs/data/Archive_tmp/1B"
BASE_1K = "/hxmt/work/HXMT-DATA/1K"
OUTFILE = os.environ.get("OUTFILE", "/scratchfs/guohx/eng_k_scan.csv")

# Box config: (eng_code, sci_code, det_offset)
BOXES = {
    "A": ("0766", "0642", 0),
    "B": ("1009", "0922", 6),
    "C": ("1781", "1686", 12),
}


def find_1k_file(date_str, hour):
    """Find 1K event file for given date and hour."""
    year = date_str[:4]
    month = date_str[4:6]
    ym = f"Y{year}{month}"
    ym_dir = os.path.join(BASE_1K, ym)
    if not os.path.isdir(ym_dir):
        return None
    # Find observation directory for this date
    for d in os.listdir(ym_dir):
        if d.startswith(date_str):
            obs_dir = os.path.join(ym_dir, d)
            # Find highest version HE-Evt file for this hour
            pattern = f"HXMT_{date_str}T{hour:02d}_HE-Evt_FFFFFF_V*_1K.FITS"
            files = sorted(glob.glob(os.path.join(obs_dir, pattern)))
            if files:
                return files[-1]  # highest version
    return None


def process_hour(date_str, hour, eng_files):
    """Process one hour: compute k for each box."""
    results = []

    # Find 1K file
    evt_file = find_1k_file(date_str, hour)
    if evt_file is None:
        return results

    # Read 1K events
    try:
        fk = fits.open(evt_file, memmap=True)
        evts = fk[1].data
        met_all = evts["Time"]
        det_all = evts["Det_ID"]
        fk.close()
    except Exception:
        return results

    for box, (eng_code, sci_code, det_off) in BOXES.items():
        if eng_code not in eng_files:
            continue
        efile = eng_files[eng_code]

        try:
            fe = fits.open(efile, memmap=True)
            d = fe["HE_Eng"].data
            offset = d["UTC_Last_Bdc"][0] - d["sTime_Last_Bdc"][0]
            met_eng = d["Time"].astype(float) + offset
            length = d["Length_Time_Cycle"].astype(float)
            length_s = length * 16e-6

            pho = sum(d[f"Cnt_PHODet_{det_off+i}"].astype(float) for i in range(6))
            csi = sum(d[f"Cnt_CsI_PHODet_{det_off+i}"].astype(float) for i in range(6))
            large = sum(d[f"Cnt_LargeEvt_{det_off+i}"].astype(float) for i in range(6))
            dead = sum(d[f"DeadTime_PHODet_{det_off+i}"].astype(float) for i in range(6))
            eng = pho - csi - large
            fe.close()
        except Exception:
            continue

        # Box events from 1K
        box_mask = (det_all >= det_off) & (det_all < det_off + 6)
        met_box = met_all[box_mask]

        # Count events in each engineering Length window using searchsorted
        obs_094 = np.zeros(len(met_eng))
        met_box_sorted = np.sort(met_box)
        for i in range(len(met_eng)):
            t0 = met_eng[i]
            t1 = t0 + length_s[i]
            lo = np.searchsorted(met_box_sorted, t0, side="left")
            hi = np.searchsorted(met_box_sorted, t1, side="left")
            obs_094[i] = hi - lo

        # Non-saturated filter
        ns = (obs_094 > 50) & (eng > 50) & (dead > 0)
        if ns.sum() < 20:
            continue

        # Check ratio to exclude saturated seconds
        ratio = np.where(eng > 0, obs_094 / eng, 0)
        ns &= (ratio > 0.6)
        if ns.sum() < 20:
            continue

        e = eng[ns]
        o = obs_094[ns]
        dd = dead[ns]
        pp = pho[ns]

        # Simple ratio k
        k_simple = np.median(e / o)

        # Dead/PHO
        dp = np.median(dd / pp)

        # Large/PHO
        lp = np.median(large[ns] / pp)

        # Find optimal beta (minimize spread across rate bins)
        best_beta = 0.0
        best_spread = 1e9
        for beta in np.arange(0.0, 0.4, 0.005):
            est = e - beta * dd
            k_vals = est / o
            medians = []
            for lo_r, hi_r in [(2000, 5000), (5000, 8000), (8000, 15000)]:
                m = (e >= lo_r) & (e < hi_r)
                if m.sum() >= 5:
                    medians.append(np.median(k_vals[m]))
            if len(medians) >= 2:
                spread = max(medians) - min(medians)
                if spread < best_spread:
                    best_spread = spread
                    best_beta = beta

        # k with optimal beta
        corrected = e - best_beta * dd
        k_beta = np.median(corrected / o)

        # Mean Length
        mean_length_s = np.mean(length_s[ns])

        results.append({
            "date": date_str,
            "hour": hour,
            "box": box,
            "n_good": int(ns.sum()),
            "k_simple": k_simple,
            "k_beta": k_beta,
            "beta": best_beta,
            "spread": best_spread,
            "dead_over_pho": dp,
            "large_over_pho": lp,
            "pho_median": np.median(pp),
            "length_s": mean_length_s,
        })

    return results


def main():
    writer = csv.DictWriter(
        open(OUTFILE, "w", newline=""),
        fieldnames=[
            "date", "hour", "box", "n_good",
            "k_simple", "k_beta", "beta", "spread",
            "dead_over_pho", "large_over_pho", "pho_median", "length_s",
        ],
    )
    writer.writeheader()

    n_processed = 0

    # Scan years 2017-2026, sample ~2 dates per month
    for year in range(2017, 2027):
        year_dir = os.path.join(BASE_1B, str(year))
        if not os.path.isdir(year_dir):
            continue

        dates = sorted(os.listdir(year_dir))
        # Sample: ~2 per month = ~24 per year
        step = max(1, len(dates) // 24)
        sample_dates = dates[::step]

        for date_dir in sample_dates:
            date_path = os.path.join(year_dir, date_dir)
            if not os.path.isdir(date_path):
                continue

            # Find engineering files for all boxes
            eng_files = {}
            for box, (eng_code, _, _) in BOXES.items():
                code_dir = os.path.join(date_path, eng_code)
                if not os.path.isdir(code_dir):
                    continue
                files = sorted(glob.glob(os.path.join(code_dir, "*.fits")))
                if files:
                    eng_files[eng_code] = files[0]  # first hour

            if not eng_files:
                continue

            # Extract hour from filename: HXMT_1B_0766_20200415T080000_...
            fname = os.path.basename(list(eng_files.values())[0])
            try:
                # Find the T in the date portion
                t_idx = fname.index("T", 15)  # skip "HXMT_1B_0766_"
                hour = int(fname[t_idx+1:t_idx+3])
            except (ValueError, IndexError):
                continue

            results = process_hour(date_dir, hour, eng_files)
            for r in results:
                writer.writerow(r)

            n_processed += 1
            if n_processed % 10 == 0:
                print(f"  processed {n_processed} dates...", file=sys.stderr, flush=True)

    print(f"Done: {n_processed} dates, output: {OUTFILE}", file=sys.stderr)


if __name__ == "__main__":
    main()
