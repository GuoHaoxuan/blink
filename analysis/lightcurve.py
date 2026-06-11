import matplotlib.pyplot as plt
import numpy as np
import csv
import sys
from astropy.io import fits
from astropy.time import Time
from pathlib import Path


def main():
    if len(sys.argv) < 3:
        print("Usage: python lightcurve.py <center_isot> <evt_fits> [sat_csv] [tmin] [tmax]")
        print("Example: python lightcurve.py 2022-10-09T13:17:00 data/.../Evt.FITS sat.csv -500 2000")
        sys.exit(1)

    center_isot = sys.argv[1]
    evt_file = sys.argv[2]
    sat_file = sys.argv[3] if len(sys.argv) > 3 else "saturation.csv"
    tmin = float(sys.argv[4]) if len(sys.argv) > 4 else -5.0
    tmax = float(sys.argv[5]) if len(sys.argv) > 5 else 5.0
    title = sys.argv[6] if len(sys.argv) > 6 else f"HXMT/HE Light Curve — {center_isot}"

    ref_time = "2012-01-01T00:00:00"
    ref = Time(ref_time, format="isot", scale="utc")
    center_met = (Time(center_isot, format="isot", scale="utc") - ref).sec
    start_met = center_met + tmin
    stop_met = center_met + tmax

    # --- 读取 1K 事件数据 ---
    with fits.open(evt_file, memmap=True) as f:
        time = f[1].data["Time"]
        mask = (time >= start_met) & (time <= stop_met)
        time_window = time[mask] - center_met

    # --- 绘制光变曲线 ---
    fig, ax = plt.subplots(figsize=(12, 5))
    ax.hist(time_window, bins=2000, histtype="step", color="black", linewidth=0.5)
    ax.set_xlabel(f"Time relative to {center_isot} (s)")
    ax.set_ylabel("Counts")
    ax.set_title(title)

    # --- 读取饱和区间并叠加 ---
    if Path(sat_file).exists():
        with open(sat_file) as f:
            reader = csv.reader(f)
            next(reader)
            for row in reader:
                s, e = float(row[0]), float(row[1])
                if e <= s:
                    continue
                s_rel = s - center_met
                e_rel = e - center_met
                if e_rel >= tmin and s_rel <= tmax:
                    s_c = max(s_rel, tmin)
                    e_c = min(e_rel, tmax)
                    ax.axvspan(s_c, e_c, alpha=0.3, color="red", label=None)

        from matplotlib.patches import Patch
        ax.legend(handles=[Patch(facecolor="red", alpha=0.3, label="Saturation")], loc="upper right")

    ax.set_xlim(tmin, tmax)
    plt.tight_layout()
    output = "lightcurve.png"
    plt.savefig(output, dpi=200)
    print(f"Saved to {output}")


if __name__ == "__main__":
    main()
