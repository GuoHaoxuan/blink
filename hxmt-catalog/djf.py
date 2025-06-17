import sqlite3
from dataclasses import dataclass
from datetime import datetime

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from common import two_column_width
from dateutil.parser import parse


@dataclass
class Signal:
    start: datetime
    longitude: float
    latitude: float

    def __init__(self, row):
        self.start = parse(row[0])
        self.longitude = float(row[1])
        self.latitude = float(row[2])


def get_data():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT start, longitude, latitude
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
    return [
        row
        for row in signals
        if row.start.month == 12 or row.start.month == 1 or row.start.month == 2
    ]


def plot_map(ax_drop, signals):
    ax_drop.set_extent([-180, 180, -43, 43], crs=ccrs.PlateCarree())
    ax_drop.coastlines()

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
    gl.xlocator = plt.MultipleLocator(45)  # 每45度放置一个经度标注
    gl.ylocator = plt.MultipleLocator(20)  # 每20度放置一个纬度标注
    gl.xformatter = plt.FuncFormatter(
        lambda v, pos: f"{int(v)}°E" if v > 0 else f"{-int(v)}°W" if v < 0 else "0°"
    )
    gl.yformatter = plt.FuncFormatter(
        lambda v, pos: f"{int(v)}°N" if v > 0 else f"{-int(v)}°S" if v < 0 else "0°"
    )
    # gl.xlabel_style = {"size": 10}
    # gl.ylabel_style = {"size": 10}

    ax_drop.scatter(
        [signal.longitude for signal in signals],
        [signal.latitude for signal in signals],
        s=1,
        c="C0",
        transform=ccrs.PlateCarree(),
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
    # ax_drop.legend(
    #     bbox_to_anchor=(0.5, 1.00),
    #     loc="lower center",
    #     markerscale=5,
    #     ncol=3,
    #     frameon=False,
    # )


# 启用 LaTeX 渲染
plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}\n\\usepackage{wasysym}\\usepackage{CJKutf8}",  # 如果需要数学公式支持
    }
)

fig = plt.figure(
    figsize=(two_column_width, (3 / 4) * two_column_width), dpi=1200, facecolor="none"
)
fig.patch.set_alpha(0.0)  # 设置图形背景的透明度
ax_drop = fig.add_subplot(1, 1, 1, projection=ccrs.PlateCarree(central_longitude=150))
ax_drop.set_facecolor("none")  # 设置坐标区域的背景为透明
signals = get_data()
plot_map(ax_drop, signals)
plt.tight_layout()
plt.savefig("hxmt-catalog/output/djf.pdf", bbox_inches="tight", transparent=True)
