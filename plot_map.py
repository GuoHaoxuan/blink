import csv
import math
import os
import sqlite3
from dataclasses import dataclass
from datetime import datetime

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from dateutil import parser
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


conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    "SELECT start, stop, fp_year, longitude, latitude, altitude, lightnings FROM signals"
)

data = cursor.fetchall()
conn.close()

signals = [
    Signal(
        start=parse(row[0]),
        stop=parse(row[1]),
        fp_year=float(row[2]),
        longitude=float(row[3]),
        latitude=float(row[4]),
        altitude=float(row[5]),
        lightnings=row[6],
    )
    for row in data
]

# 收集所有数据点
lons = []
lats = []
fp_values = []
lightnings = []

for signal in signals:
    lon = signal.longitude
    lat = signal.latitude
    fp_year = -math.log(signal.fp_year) if signal.fp_year > 0 else 100
    lightning = signal.lightnings != "[]"

    lons.append(lon)
    lats.append(lat)
    fp_values.append(fp_year)
    lightnings.append(lightning)

lons = np.array(lons)
lats = np.array(lats)
fp_values = np.array(fp_values)
lightnings = np.array(lightnings)

fig = plt.figure(figsize=(16, 9), dpi=300)
gs = fig.add_gridspec(4, 1)

axes = [fig.add_subplot(gs[i], projection=ccrs.PlateCarree()) for i in range(4)]

ranges = [(-9999, 0), (0, 5), (5, 30), (30, 9999)]

for ax in axes:
    ax.set_extent([-180, 180, -25.6, 25.6], crs=ccrs.PlateCarree())
    ax.coastlines(linewidth=1.5)

    for i, (ax, (min_val, max_val)) in enumerate(zip(axes, ranges)):
        mask = (fp_values >= min_val) & (fp_values < max_val)
        scatter = ax.scatter(
            lons[mask],
            lats[mask],
            c=fp_values[mask],
            cmap="viridis",
            s=1,
            marker="o",
            transform=ccrs.PlateCarree(),
        )
        scatter = ax.scatter(
            lons[mask & lightnings],
            lats[mask & lightnings],
            c="red",
            s=3,
            marker="o",
            transform=ccrs.PlateCarree(),
        )

        # Add range title to the subplot
        range_title = f"Range: {min_val} to {max_val}"
        if max_val == 9999:
            range_title = "Too high to calculate"
        if min_val == -9999:
            range_title = f"Range: <{max_val}"
        ax.set_title(range_title)

plt.savefig(os.path.join("output", "map.png"), bbox_inches="tight", dpi=300)
