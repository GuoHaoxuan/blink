import json
import math
import sqlite3
from dataclasses import dataclass
from datetime import datetime

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse
from matplotlib.ticker import MultipleLocator


@dataclass
class Signal:
    start: datetime
    stop: datetime
    fp_year: float
    longitude: float
    latitude: float
    altitude: float
    events: str
    lightnings: str
    satellite: str
    detector: str

    def __init__(self, row):
        self.start = parse(row[0])
        self.stop = parse(row[1])
        self.fp_year = min(-math.log10(row[2]) if row[2] > 0 else 100, 14.5)
        self.longitude = float(row[3])
        self.latitude = float(row[4])
        self.altitude = float(row[5])
        self.events = row[6]
        self.lightnings = False
        lightnings_json = json.loads(row[7])
        for lightning in lightnings_json:
            if lightning["is_associated"]:
                self.lightnings = True
                break
        self.satellite = row[8]
        self.detector = row[9]


def plot_map(ax_drop, signals):
    ax_drop.set_extent([-180, 180, -43, 43], crs=ccrs.PlateCarree())
    ax_drop.coastlines(linewidth=1.5)

    # 添加经纬度边缘标注（不绘制网格线）
    gl = ax_drop.gridlines(
        crs=ccrs.PlateCarree(),
        draw_labels=True,
        linewidth=0,
        color="gray",
        alpha=0,
    )
    gl.top_labels = False  # 不在顶部显示经度标签
    gl.right_labels = False  # 不在右侧显示纬度标签
    gl.xlines = False  # 不显示经线网格
    gl.ylines = False  # 不显示纬线网格
    gl.xlocator = plt.MultipleLocator(30)  # 每30度放置一个经度标注
    gl.ylocator = plt.MultipleLocator(15)  # 每15度放置一个纬度标注
    gl.xformatter = plt.FuncFormatter(
        lambda v, pos: f"{int(v)}°E" if v > 0 else f"{-int(v)}°W" if v < 0 else "0°"
    )
    gl.yformatter = plt.FuncFormatter(
        lambda v, pos: f"{int(v)}°N" if v > 0 else f"{-int(v)}°S" if v < 0 else "0°"
    )
    gl.xlabel_style = {"size": 10}
    gl.ylabel_style = {"size": 10}

    # 绘制无闪电信号点
    signals_no_lightning = [signal for signal in signals if not signal.lightnings]
    len_signals = len(signals_no_lightning)
    ax_drop.scatter(
        [signal.longitude for signal in signals_no_lightning],
        [signal.latitude for signal in signals_no_lightning],
        s=1,
        c="C0",
        transform=ccrs.PlateCarree(),
        label=f"Signal ({len_signals})",
    )

    # 绘制有闪电信号点
    signals_with_lightning = [signal for signal in signals if signal.lightnings]
    len_signals = len(signals_with_lightning)
    ax_drop.scatter(
        [signal.longitude for signal in signals_with_lightning],
        [signal.latitude for signal in signals_with_lightning],
        s=1,
        c="C1",
        transform=ccrs.PlateCarree(),
        label=f"Signal with Lightnings ({len_signals})",
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
        color="black",
        linewidth=1,
        linestyle="--",
        label="SAA",
        transform=ccrs.PlateCarree(),
    )
    ax_drop.legend(
        loc="lower right",
        markerscale=5,
    )


def get_signals():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT start, stop, fp_year, longitude, latitude, altitude, events, lightnings, satellite, detector
        FROM signal
        WHERE start < "2025-01-01"
        AND fp_year < 1e-3
        AND duration > 200e-6
        AND duration < 3e-3
        """
    )
    data = cursor.fetchall()
    conn.close()
    signals = [Signal(row) for row in data]
    return signals


fig = plt.figure(figsize=(16, 9), dpi=1200, facecolor="none")  # 设置图形的背景为透明
fig.patch.set_alpha(0.0)  # 设置图形背景的透明度
ax_drop = fig.add_subplot(1, 1, 1, projection=ccrs.PlateCarree(central_longitude=150))
ax_drop.set_facecolor("none")  # 设置坐标区域的背景为透明
signals = get_signals()
plot_map(ax_drop, signals)
# plt.savefig("signals_map.svg", bbox_inches="tight", transparent=True)  # 保存为透明背景
# plt.savefig("signals_map.png", bbox_inches="tight", transparent=True)  # 保存为透明背景
plt.savefig("signals_map.emf", bbox_inches="tight", transparent=True)  # 保存为透明背景
