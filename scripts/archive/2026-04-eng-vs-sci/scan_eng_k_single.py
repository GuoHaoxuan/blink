#!/usr/bin/env python3
"""Process one year of engineering vs 1K data. Called with: python3 scan_eng_k_single.py <year>"""
import numpy as np
from astropy.io import fits
import glob, os, sys, csv

YEAR = int(sys.argv[1])
BASE_1B = "/hxmtfs/data/Archive_tmp/1B"
BASE_1K = "/hxmt/work/HXMT-DATA/1K"
WORKDIR = "/scratchfs/gecam/guohx/eng_k_scan"
OUTFILE = os.path.join(WORKDIR, f"result_{YEAR}.csv")

BOXES = {
    "A": ("0766", 0),
    "B": ("1009", 6),
    "C": ("1781", 12),
}


def find_1k_file(date_str, hour):
    ym = f"Y{date_str[:6]}"
    ym_dir = os.path.join(BASE_1K, ym)
    if not os.path.isdir(ym_dir):
        return None
    for d in os.listdir(ym_dir):
        if d.startswith(date_str):
            pattern = os.path.join(ym_dir, d, f"HXMT_{date_str}T{hour:02d}_HE-Evt_FFFFFF_V*_1K.FITS")
            files = sorted(glob.glob(pattern))
            if files:
                return files[-1]
    return None


def process_hour(date_str, hour, eng_files):
    results = []
    evt_file = find_1k_file(date_str, hour)
    if evt_file is None:
        return results

    try:
        fk = fits.open(evt_file, memmap=True)
        met_all = fk[1].data["Time"]
        det_all = fk[1].data["Det_ID"]
        fk.close()
    except Exception as e:
        print(f"  ERR reading {evt_file}: {e}", file=sys.stderr, flush=True)
        return results

    for box, (eng_code, det_off) in BOXES.items():
        if eng_code not in eng_files:
            continue
        try:
            fe = fits.open(eng_files[eng_code], memmap=True)
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

        box_mask = (det_all >= det_off) & (det_all < det_off + 6)
        met_box = np.sort(met_all[box_mask])

        obs_094 = np.zeros(len(met_eng))
        for i in range(len(met_eng)):
            lo = np.searchsorted(met_box, met_eng[i])
            hi = np.searchsorted(met_box, met_eng[i] + length_s[i])
            obs_094[i] = hi - lo

        ns = (obs_094 > 50) & (eng > 50) & (dead > 0)
        ratio = np.where(eng > 0, obs_094 / eng, 0)
        ns &= (ratio > 0.6)
        if ns.sum() < 20:
            continue

        e, o, dd, pp = eng[ns], obs_094[ns], dead[ns], pho[ns]

        k_simple = np.median(e / o)
        dp = np.median(dd / pp)
        lp = np.median(large[ns] / pp)

        best_beta, best_spread = 0.0, 1e9
        for beta in np.arange(0.0, 0.4, 0.005):
            k_vals = (e - beta * dd) / o
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

        k_beta = np.median((e - best_beta * dd) / o)

        results.append({
            "date": date_str, "hour": hour, "box": box,
            "n_good": int(ns.sum()),
            "k_simple": f"{k_simple:.6f}",
            "k_beta": f"{k_beta:.6f}",
            "beta": f"{best_beta:.3f}",
            "spread": f"{best_spread:.6f}",
            "dead_over_pho": f"{dp:.6f}",
            "large_over_pho": f"{lp:.6f}",
            "pho_median": f"{np.median(pp):.1f}",
            "length_s": f"{np.mean(length_s[ns]):.6f}",
        })
    return results


def main():
    os.makedirs(WORKDIR, exist_ok=True)
    year_dir = os.path.join(BASE_1B, str(YEAR))
    if not os.path.isdir(year_dir):
        print(f"No data for {YEAR}", file=sys.stderr)
        return

    dates = sorted(os.listdir(year_dir))
    step = max(1, len(dates) // 24)
    sample_dates = dates[::step]

    fields = ["date", "hour", "box", "n_good", "k_simple", "k_beta", "beta",
              "spread", "dead_over_pho", "large_over_pho", "pho_median", "length_s"]
    writer = csv.DictWriter(open(OUTFILE, "w", newline=""), fieldnames=fields)
    writer.writeheader()

    for idx, date_dir in enumerate(sample_dates):
        date_path = os.path.join(year_dir, date_dir)
        if not os.path.isdir(date_path):
            continue

        eng_files = {}
        for box, (eng_code, _) in BOXES.items():
            code_dir = os.path.join(date_path, eng_code)
            if os.path.isdir(code_dir):
                files = sorted(glob.glob(os.path.join(code_dir, "*.fits")))
                if files:
                    eng_files[eng_code] = files[0]
        if not eng_files:
            continue

        fname = os.path.basename(list(eng_files.values())[0])
        try:
            t_idx = fname.index("T", 15)
            hour = int(fname[t_idx+1:t_idx+3])
        except (ValueError, IndexError):
            continue

        print(f"  [{YEAR}] {idx+1}/{len(sample_dates)}: {date_dir} T{hour:02d}", file=sys.stderr, flush=True)
        for r in process_hour(date_dir, hour, eng_files):
            writer.writerow(r)

    print(f"Done: {YEAR} -> {OUTFILE}", file=sys.stderr)


if __name__ == "__main__":
    main()
