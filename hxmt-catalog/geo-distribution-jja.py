import sqlite3

import cartopy.crs as ccrs
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse
from PIL import Image

# 增加PIL图像大小限制
Image.MAX_IMAGE_PIXELS = None

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT longitude, latitude, associated_lightning_count, start
    FROM signal
    WHERE start < '2025-01-01'
        AND (fp_year < 1e-5 OR (fp_year < 1 AND associated_lightning_count > 0));
    """
)
signals = [row for row in cursor.fetchall() if parse(row[3]).month in [6, 7, 8]]
cursor.close()
conn.close()

longitudes_no_lightning = [signal[0] for signal in signals if signal[2] == 0]
latitudes_no_lightning = [signal[1] for signal in signals if signal[2] == 0]
longitudes_with_lightning = [signal[0] for signal in signals if signal[2] > 0]
latitudes_with_lightning = [signal[1] for signal in signals if signal[2] > 0]
longitudes_all = [signal[0] for signal in signals]
latitudes_all = [signal[1] for signal in signals]

plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
cm = 1 / 2.54
fig = plt.figure(figsize=(20 * cm, 6.6 * cm), dpi=1200)

# 创建网格布局，共享坐标轴
# 调整width_ratios，让右侧直方图更窄，以匹配地图的长宽比
gs = fig.add_gridspec(
    3, 2, height_ratios=[0.7, 1, 3.5], width_ratios=[12, 1], hspace=0, wspace=0
)

# 创建子图并共享坐标轴
ax_map = fig.add_subplot(gs[2, 0], projection=ccrs.PlateCarree(central_longitude=150))
longitude_ax = fig.add_subplot(gs[1, 0])
latitude_ax = fig.add_subplot(gs[2, 1])
legend_ax = fig.add_subplot(gs[0, :])

ax_map.set_extent([-180, 180, -43, 43], crs=ccrs.PlateCarree())
fname = "NE1_HR_LC_SR_W_DR/NE1_HR_LC_SR_W_DR.tif"
ax_map.imshow(
    plt.imread(fname),
    origin="upper",
    transform=ccrs.PlateCarree(),
    extent=[-180, 180, -90, 90],
    alpha=0.6,
)
ax_map.coastlines()
ax_map.scatter(
    longitudes_no_lightning,
    latitudes_no_lightning,
    marker="o",
    s=2,
    facecolor="C0",
    edgecolors="None",
    transform=ccrs.PlateCarree(),
    label=f"Signal ({len(longitudes_no_lightning)})",
)
ax_map.scatter(
    longitudes_with_lightning,
    latitudes_with_lightning,
    marker="o",
    s=2,
    facecolor="C2",
    edgecolors="None",
    transform=ccrs.PlateCarree(),
    label=f"Signal with Lightnings ({len(longitudes_with_lightning)})",
)
SAA_Lon_ARR_Raw = np.array(
    [-74.3, -88.2, -96, -92, -70, -45, -33, -15, 0.8, 18.2, 31, 27.3, 22, -74.3]
)
SAA_Lat_ARR_Raw = np.array(
    [-45, -28, -13, -9, -2.5, 3, 2.1, -15, -18.8, -23, -31, -39, -45, -45]
)
ax_map.fill(
    SAA_Lon_ARR_Raw,
    SAA_Lat_ARR_Raw,
    facecolor="white",
    edgecolor="None",
    linewidth=0,
    transform=ccrs.PlateCarree(),
)
ax_map.fill(
    SAA_Lon_ARR_Raw,
    SAA_Lat_ARR_Raw,
    facecolor="C3",
    edgecolor="None",
    linewidth=0,
    alpha=0.1,
    transform=ccrs.PlateCarree(),
)

# 设置地图坐标轴
ax_map.set_xlim(-180, 180)
ax_map.set_ylim(-43, 43)
ax_map.set_xticks(np.arange(-180, 179, 45))
ax_map.set_yticks(np.arange(-30, 36, 15))
ax_map.set_xticklabels(
    [
        f"{abs(int(x))}°{'W' if x < 0 else 'E' if x > 0 else ''}"
        for x in np.arange(-180, 179, 45)
    ]
)
ax_map.set_yticklabels(
    [
        f"{abs(int(y))}°{'S' if y < 0 else 'N' if y > 0 else ''}"
        for y in np.arange(-30, 36, 15)
    ]
)
# ax_map.set_xlabel("Longitude")
# ax_map.set_ylabel("Latitude")

longitude_ax.hist(
    [x if x < 330 else x - 360 for x in longitudes_all],
    range=(-30, 330),  # 设置直方图范围与地图一致
    bins=180,
    color="C0",
    label="Signal",
    histtype="step",
)
longitude_ax.hist(
    [x if x < 330 else x - 360 for x in longitudes_with_lightning],
    range=(-30, 330),  # 设置直方图范围与地图一致
    bins=180,
    color="C2",
    label="Signal with Lightnings",
    histtype="step",
)
longitude_ax.set_xlim(-30, 330)
# longitude_ax.set_yscale("log")  # 设置y轴为对数刻度
longitude_ax.tick_params(labelbottom=False, bottom=False)  # 隐藏x轴标签和底部tick短线
longitude_ax.set_ylabel("Number")

latitude_ax.hist(
    latitudes_all,
    bins=43,
    range=(-43, 43),  # 设置直方图范围与地图一致
    color="C0",
    label="Signal",
    orientation="horizontal",
    histtype="step",
)
latitude_ax.hist(
    latitudes_with_lightning,
    bins=43,
    range=(-43, 43),  # 设置直方图范围与地图一致
    color="C2",
    label="Signal with Lightnings",
    orientation="horizontal",
    histtype="step",
)
latitude_ax.set_ylim(-43, 43)
latitude_ax.tick_params(labelleft=False, left=False)  # 隐藏y轴标签和左侧tick短线
# latitude_ax.set_xscale("log")  # 设置x轴为对数刻度
latitude_ax.set_xlabel("Number")

legend_handles = [
    mpatches.Patch(facecolor="C0", edgecolor="None", label="All Signals"),
    mpatches.Patch(facecolor="C2", edgecolor="None", label="Lightning Associated"),
    # plt.Line2D([], [], color="black", linestyle="-", label="Coastline"),
    mpatches.Patch(facecolor="C3", edgecolor="None", alpha=0.1, label="SAA"),
]
legend_ax.legend(
    handles=legend_handles, loc="center", ncol=len(legend_handles), frameon=False
)
# hide legend axes
legend_ax.axis("off")  # 隐藏图例轴

# 隐藏地图顶部和右侧的tick短线
ax_map.tick_params(top=False, right=False)

plt.savefig("hxmt-catalog/output/jja.pdf", bbox_inches="tight")
