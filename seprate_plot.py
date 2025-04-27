import math
import sqlite3
from dataclasses import dataclass
from datetime import datetime

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse


@dataclass
class Signal:
    start: datetime
    stop: datetime
    fp_year: float
    longitude: float
    latitude: float
    altitude: float
    lightnings: str
    coincidence_probability: float
    acd_rate: float

    def __init__(self, row):
        self.start = parse(row[0])
        self.stop = parse(row[1])
        self.fp_year = min(-math.log10(row[2]) if row[2] > 0 else 100, 14.5)
        self.longitude = float(row[3])
        self.latitude = float(row[4])
        self.altitude = float(row[5])
        self.lightnings = row[7]
        self.coincidence_probability = float(row[8])


def plot_map(ax_drop, signals):
    ax_drop.set_extent([-180, 180, -43, 43], crs=ccrs.PlateCarree())
    ax_drop.coastlines(linewidth=1.5)
    ax_drop.scatter(
        [signal.longitude for signal in signals if signal.lightnings == "[]"],
        [signal.latitude for signal in signals if signal.lightnings == "[]"],
        s=1,
        c="C0",
        transform=ccrs.PlateCarree(),
        label="Signal",
    )
    ax_drop.scatter(
        [signal.longitude for signal in signals if signal.lightnings != "[]"],
        [signal.latitude for signal in signals if signal.lightnings != "[]"],
        s=1,
        c="C1",
        transform=ccrs.PlateCarree(),
        label="Signal with Lightnings",
    )
    ax_drop.legend(
        loc=(0.25, 0.05),
        markerscale=5,
    )
    SAA_Lon_ARR_Raw = np.array(
        [-74.3, -88.2, -96, -92, -70, -45, -33, -15, 0.8, 18.2, 31, 27.3, 22, -74.3]
    )
    SAA_Lat_ARR_Raw = np.array(
        [-45, -28, -13, -9, -2.5, 3, 2.1, -15, -18.8, -23, -31, -39, -45, -45]
    )
    ax_drop.plot(
        SAA_Lon_ARR_Raw,
        SAA_Lat_ARR_Raw,
        color="#4B5361",
        linewidth=1,
        linestyle="--",
        label="SAA",
        transform=ccrs.PlateCarree(),
    )


def plot_distribution(ax_fp_year, bins, all_fp_year, filtered_fp_year, log=False):
    ax_fp_year.hist(
        all_fp_year,
        bins=bins,
        facecolor="none",
        edgecolor="#005D9B",
        label="All",
        hatch="/",
    )
    ax_fp_year.hist(
        filtered_fp_year,
        bins=bins,
        facecolor="none",
        edgecolor="#9A131A",
        label="Lightnings",
        hatch="/",
    )
    ax_fp_year.legend(loc="upper right")
    ax_fp_year.set_yscale("log")
    ax_fp_year.set_ylabel("Number")
    percentage = np.zeros(len(bins) - 1)
    for j in range(len(bins) - 1):
        percentage[j] = np.sum(
            (filtered_fp_year >= bins[j]) & (filtered_fp_year < bins[j + 1])
        ) / np.sum((all_fp_year >= bins[j]) & (all_fp_year < bins[j + 1]))
    ax2 = ax_fp_year.twinx()
    ax2.plot(
        bins[:-1] + (bins[1] - bins[0]) / 2
        if not log
        else np.exp(np.log(bins[:-1]) + (np.log(bins[1]) - np.log(bins[0])) / 2),
        percentage,
        color="#4B5361",
        marker="x",
    )
    ax2.set_ylabel("Percentage")
    if log:
        ax_fp_year.set_xscale("log")


def get_signals():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT start, stop, fp_year, longitude, latitude, altitude, events, lightnings, coincidence_probability
        FROM signals
        WHERE start < '2025-01-01'
        """
    )
    data = cursor.fetchall()
    conn.close()
    signals = [Signal(row) for row in data]
    return signals


signals = get_signals()

fig = plt.figure(figsize=(16, 9), dpi=300)
gs = fig.add_gridspec(5, 1)


bins = np.linspace(-1.5, 15, 34)
ax_fp_year = fig.add_subplot(gs[4, :])
all_fp_year = np.array([signal.fp_year for signal in signals])
filtered_fp_year = np.array(
    [signal.fp_year for signal in signals if signal.lightnings != "[]"]
)
plot_distribution(ax_fp_year, bins, all_fp_year, filtered_fp_year)
ax_fp_year.set_title("-log10(FP year)")
fp_year_threshold = 4
ax_fp_year.axvspan(-1.5, fp_year_threshold, alpha=0.2, color="red")
ax_fp_year.axvspan(fp_year_threshold, 15, alpha=0.2, color="green")

signals_take = []
signals_drop = []

for signal in signals:
    if signal.fp_year >= fp_year_threshold:
        signals_take.append(signal)
    else:
        signals_drop.append(signal)

ax_take = fig.add_subplot(gs[0:2, 0], projection=ccrs.PlateCarree())
plot_map(ax_take, signals_take)
ax_take.set_title(
    "Take {} signals, {}({:.2f}%) lightnings, {:.2f} accident coincidence total".format(
        len(signals_take),
        len([s for s in signals_take if s.lightnings != "[]"]),
        len([s for s in signals_take if s.lightnings != "[]"])
        / len(signals_take)
        * 100,
        sum([signal.coincidence_probability for signal in signals_take]),
    )
)

ax_drop = fig.add_subplot(gs[2:4, 0], projection=ccrs.PlateCarree())
plot_map(ax_drop, signals_drop)
ax_drop.set_title(
    "Drop {} signals, {}({:.2f}%) lightnings, {:.2f} accident coincidence".format(
        len(signals_drop),
        len([s for s in signals_drop if s.lightnings != "[]"]),
        len([s for s in signals_drop if s.lightnings != "[]"])
        / len(signals_take)
        * 100,
        sum([signal.coincidence_probability for signal in signals_drop]),
    )
)

plt.tight_layout()
plt.savefig("green.png")
