import json
import math
import os
import sqlite3
from dataclasses import dataclass
from datetime import datetime, timedelta

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse


def wgs84_to_lonlatalt(x, y, z):
    """
    Convert WGS84 coordinates to longitude, latitude, and altitude.
    """
    lon = np.arctan2(y, x) * 180 / np.pi
    hyp = np.sqrt(x**2 + y**2)
    lat = np.arctan2(z, hyp) * 180 / np.pi
    alt = np.sqrt(x**2 + y**2 + z**2) - 6371000.0
    return lon, lat, alt


@dataclass
class Signal:
    start: datetime
    stop: datetime
    fp_year: float
    events: json
    position: json
    lightnings: json


conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute("SELECT start, stop, fp_year, events, position, lightnings FROM signals")

data = cursor.fetchall()
conn.close()

signals = [
    Signal(
        start=parse(row[0]),
        stop=parse(row[1]),
        fp_year=float(row[2]),
        events=json.loads(row[3]),
        position=json.loads(row[4]),
        lightnings=json.loads(row[5]),
    )
    for row in data
    if row[4] is not None
]

fig = plt.figure(figsize=(18, 4.6), dpi=300)
map = plt.axes(projection=ccrs.PlateCarree())
map.set_extent([-180, 180, -43, 43], crs=ccrs.PlateCarree())
map.coastlines(linewidth=1.5)

# 收集所有数据点
lons = []
lats = []
fp_values = []

for signal in signals:
    if signal.position is not None:
        lat = signal.position["sc_lat"]
        lon = signal.position["sc_lon"]
        lon1, lat1, alt1 = wgs84_to_lonlatalt(
            signal.position["pos"][0],
            signal.position["pos"][1],
            signal.position["pos"][2],
        )
        print(alt1)

        # 安全计算对数，处理零值和负值
        if signal.fp_year > 0:
            fp_year = -math.log(signal.fp_year)
        else:
            continue

        if fp_year < 3:
            continue
        lons.append(lon)
        lats.append(lat)
        fp_values.append(fp_year)

# 使用scatter绘制点，使用colormap为点着色
scatter = map.scatter(
    lons,
    lats,
    c=fp_values,
    cmap="viridis",
    s=1,  # 将点的大小从10减小到3
    marker="o",
    alpha=0.7,
    transform=ccrs.PlateCarree(),
)

# 添加colorbar
cbar = plt.colorbar(scatter, orientation="horizontal", pad=0.05, shrink=0.8)
cbar.set_label("-log(False Positive Rate per Year)")

plt.savefig(os.path.join("output", "map.png"), bbox_inches="tight", dpi=300)
