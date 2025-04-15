import json
import math
import sqlite3
from dataclasses import dataclass
from datetime import datetime

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse

acd = np.loadtxt("src/hxmt/acd.txt")


def interpolate_point(lon, lat):
    """
    Interpolate a point in the given data set.

    Parameters:
    - data: numpy array, the data set to interpolate from
    - lon: float, the longitude of the point
    - lat: float, the latitude of the point

    Returns:
    - interpolated_value: float, the interpolated value at the given point
    """
    acdx = acd[0:180, :]
    acdy = acd[180:360, :]
    data1 = acd[360::]
    data = np.nan_to_num(data1)
    lon_indices = np.searchsorted(acdx[0, :], lon)
    lat_indices = np.searchsorted(acdy[:, 0], lat)

    lon_index = min(lon_indices, len(acdx[0, :]) - 1)
    lat_index = min(lat_indices, len(acdy[:, 0]) - 1)

    lon_fraction = (lon - acdx[0, lon_index - 1]) / (
        acdx[0, lon_index] - acdx[0, lon_index - 1]
    )
    lat_fraction = (lat - acdy[lat_index - 1, 0]) / (
        acdy[lat_index, 0] - acdy[lat_index - 1, 0]
    )

    interpolated_value = (
        (1 - lon_fraction) * (1 - lat_fraction) * data[lat_index - 1, lon_index - 1]
        + lon_fraction * (1 - lat_fraction) * data[lat_index - 1, lon_index]
        + (1 - lon_fraction) * lat_fraction * data[lat_index, lon_index - 1]
        + lon_fraction * lat_fraction * data[lat_index, lon_index]
    )

    return interpolated_value


@dataclass
class Signal:
    start: datetime
    stop: datetime
    fp_year: float
    longitude: float
    latitude: float
    altitude: float
    lightnings: str
    average: float
    bin_size_best: float
    bin_size_max: float
    bin_size_min: float
    count: int
    acd_ratio: float
    acd_rate: float

    def __init__(self, row):
        self.start = parse(row[0])
        self.stop = parse(row[1])
        self.fp_year = min(-math.log10(row[2]) if row[2] > 0 else 100, 14.5)
        self.longitude = float(row[3])
        self.latitude = float(row[4])
        self.altitude = float(row[5])
        self.lightnings = row[6]
        debugs = json.loads(row[7])
        self.average = debugs["average"]
        self.bin_size_best = debugs["bin_size_best"]["time"]
        self.bin_size_max = debugs["bin_size_max"]["time"]
        self.bin_size_min = debugs["bin_size_min"]["time"]
        self.count = debugs["count"]
        events = json.loads(row[8])
        acd_count = 0
        for evevt in events:
            if "true" in evevt["detector"]:
                acd_count += 1
        self.acd_ratio = acd_count / len(events)
        self.acd_rate = interpolate_point(self.longitude, self.latitude)


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
        SELECT start, stop, fp_year, longitude, latitude, altitude, lightnings, position_debug, events
        FROM signals
        """
    )
    data = cursor.fetchall()
    conn.close()
    signals = [Signal(row) for row in data]
    return signals


signals = get_signals()

fig = plt.figure(figsize=(16, 9), dpi=300)
gs = fig.add_gridspec(4, 2)


bins = np.linspace(-1.5, 15, 34)
ax_fp_year = fig.add_subplot(gs[1, :])
all_fp_year = np.array([signal.fp_year for signal in signals])
filtered_fp_year = np.array(
    [signal.fp_year for signal in signals if signal.lightnings != "[]"]
)
plot_distribution(ax_fp_year, bins, all_fp_year, filtered_fp_year)
ax_fp_year.set_title("-log10(FP year)")
fp_year_yellow = 1
fp_year_green = 3
ax_fp_year.axvspan(-1.5, fp_year_yellow, alpha=0.1, color="red")
ax_fp_year.axvspan(fp_year_yellow, fp_year_green, alpha=0.3, color="yellow")
ax_fp_year.axvspan(fp_year_green, 15, alpha=0.1, color="green")

signals_rough = [
    signal
    for signal in signals
    if signal.fp_year < fp_year_green and signal.fp_year >= fp_year_yellow
]

bins = np.linspace(0, 1, 21)
ax_acd_ratio = fig.add_subplot(gs[2, 0])
all_acd_ratio = np.array([signal.acd_ratio for signal in signals_rough])
filtered_acd_ratio = np.array(
    [signal.acd_ratio for signal in signals_rough if signal.lightnings != "[]"]
)
plot_distribution(ax_acd_ratio, bins, all_acd_ratio, filtered_acd_ratio)
ax_acd_ratio.set_title("ACD ratio")
acd_ratio_threshold = 0.2
ax_acd_ratio.axvspan(0, acd_ratio_threshold, alpha=0.2, color="green")
ax_acd_ratio.axvspan(acd_ratio_threshold, 1, alpha=0.2, color="red")

bins = np.array([7.5, 15, 30, 60, 120, 240, 480, 960]) * 1e-6
ax_duration = fig.add_subplot(gs[2, 1])
all_duration = np.array([signal.bin_size_best for signal in signals_rough])
filtered_duration = np.array(
    [signal.bin_size_best for signal in signals_rough if signal.lightnings != "[]"]
)
plot_distribution(ax_duration, bins, all_duration, filtered_duration, log=True)
ax_duration.set_xticks(
    np.array([10, 20, 40, 80, 160, 320, 640]) * 1e-6,
    ["10", "20", "40", "80", "160", "320", "640"],
)
ax_duration.set_title("Duration")
duration_threshold_l = 60e-6
duration_threshold_r = 240e-6
ax_duration.axvspan(7.5e-6, duration_threshold_l, alpha=0.2, color="red")
ax_duration.axvspan(
    duration_threshold_l, duration_threshold_r, alpha=0.2, color="green"
)
ax_duration.axvspan(duration_threshold_r, 960e-6, alpha=0.2, color="red")

bins = np.logspace(3.8, 5.6, 30)
ax_count_ratio = fig.add_subplot(gs[3, 0])
all_count_ratio = np.array(
    [
        min(signal.count / (signal.stop - signal.start).total_seconds(), 5e5)
        for signal in signals_rough
    ]
)
filtered_count_ratio = np.array(
    [
        min(signal.count / (signal.stop - signal.start).total_seconds(), 5e5)
        for signal in signals_rough
        if signal.lightnings != "[]"
    ]
)
plot_distribution(ax_count_ratio, bins, all_count_ratio, filtered_count_ratio, log=True)
ax_count_ratio.set_title("Count ratio")
count_ratio_threshold_l = 2e4
count_ratio_threshold_r = 7e4
ax_count_ratio.axvspan(10**3.8, count_ratio_threshold_l, alpha=0.2, color="red")
ax_count_ratio.axvspan(
    count_ratio_threshold_l, count_ratio_threshold_r, alpha=0.2, color="green"
)
ax_count_ratio.axvspan(count_ratio_threshold_r, 10**5.6, alpha=0.2, color="red")

bins = np.logspace(0, 4, 30)
ax_acd_rate = fig.add_subplot(gs[3, 1])
all_acd_rate = np.array([signal.acd_rate for signal in signals_rough])
filtered_acd_rate = np.array(
    [signal.acd_rate for signal in signals_rough if signal.lightnings != "[]"]
)
plot_distribution(ax_acd_rate, bins, all_acd_rate, filtered_acd_rate, log=True)
ax_acd_rate.set_title("ACD rate")
# acd_rate_threshold_l = 25
# acd_rate_threshold_r = 1000
# ax_acd_rate.axvspan(1, acd_rate_threshold_l, alpha=0.2, color="red")
# ax_acd_rate.axvspan(
#     acd_rate_threshold_l, acd_rate_threshold_r, alpha=0.2, color="green"
# )
# ax_acd_rate.axvspan(acd_rate_threshold_r, 10**4, alpha=0.2, color="red")


signals_take = []
signals_drop = []

for signal in signals_rough:
    if (
        signal.acd_ratio < acd_ratio_threshold
        and signal.bin_size_best > duration_threshold_l
        and signal.bin_size_best < duration_threshold_r
        and signal.count / (signal.stop - signal.start).total_seconds()
        > count_ratio_threshold_l
        and signal.count / (signal.stop - signal.start).total_seconds()
        < count_ratio_threshold_r
        # and signal.acd_rate > acd_rate_threshold_l
        # and signal.acd_rate < acd_rate_threshold_r
    ):
        signals_take.append(signal)
    else:
        signals_drop.append(signal)

ax_take = fig.add_subplot(gs[0, 0], projection=ccrs.PlateCarree())
plot_map(ax_take, signals_take)
ax_take.set_title(
    "Take {} signals, {}({:.2f}%) lightnings".format(
        len(signals_take),
        len([s for s in signals_take if s.lightnings != "[]"]),
        len([s for s in signals_take if s.lightnings != "[]"])
        / len(signals_take)
        * 100,
    )
)

ax_drop = fig.add_subplot(gs[0, 1], projection=ccrs.PlateCarree())
plot_map(ax_drop, signals_drop)
ax_drop.set_title(
    "Drop {} signals, {}({:.2f}%) lightnings".format(
        len(signals_drop),
        len([s for s in signals_drop if s.lightnings != "[]"]),
        len([s for s in signals_drop if s.lightnings != "[]"])
        / len(signals_take)
        * 100,
    )
)

plt.tight_layout()
plt.savefig("yellow.png")
