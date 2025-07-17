import sqlite3

import cartopy.crs as ccrs
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.gridspec import GridSpec
from PIL import Image

# 增加PIL图像大小限制
Image.MAX_IMAGE_PIXELS = None

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT
        start_full, start_best, stop_full, stop_best,
        background, events,
        light_curve_1s_unfiltered, light_curve_1s_filtered,
        light_curve_100ms_unfiltered, light_curve_100ms_filtered,
        longitude, latitude,
        orbit, lightnings
    FROM signal
    WHERE start_full = '2022-10-07T15:01:48.278494Z' AND satellite = 'HXMT' AND detector = 'HE'
    """
)
signal = cursor.fetchone()
cursor.close()
conn.close()

(
    start_full,
    start_best,
    stop_full,
    stop_best,
    background,
    events,
    light_curve_1s_unfiltered,
    light_curve_1s_filtered,
    light_curve_100ms_unfiltered,
    light_curve_100ms_filtered,
    longitude,
    latitude,
    orbit,
    lightnings,
) = signal

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
fig = plt.figure(figsize=(20 * cm, 8 * cm), dpi=1200)

gs = GridSpec(4, 2, width_ratios=[1, 2], height_ratios=[1, 4, 4, 4])

# 创建各个子图
ax_legend = fig.add_subplot(gs[0, :])
ax_map = fig.add_subplot(
    gs[1:, 0], projection=ccrs.PlateCarree(central_longitude=longitude)
)
ax_detail = fig.add_subplot(gs[1, 1])
ax_1s = fig.add_subplot(gs[2, 1])
ax_100ms = fig.add_subplot(gs[3, 1])

ax_map.set_extent(
    [longitude - 15, longitude + 15, latitude - 15, latitude + 15],
    crs=ccrs.PlateCarree(),
)
ax_map.coastlines()
fname = "NE1_HR_LC_SR_W_DR/NE1_HR_LC_SR_W_DR.tif"
ax_map.imshow(
    plt.imread(fname),
    origin="upper",
    transform=ccrs.PlateCarree(),
    extent=[-180, 180, -90, 90],
    alpha=0.6,
)
ax_map.scatter(
    longitude,
    latitude,
    marker="o",
    s=20,
    facecolor="C1",
    edgecolors="None",
    transform=ccrs.PlateCarree(),
    label=f"Signal ({start_full})",
)

plt.tight_layout()
plt.savefig("hxmt-catalog/output/detail.pdf", bbox_inches="tight")
