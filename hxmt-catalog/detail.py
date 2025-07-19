import json
import sqlite3

import cartopy.crs as ccrs
import dateutil
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse as dateutil_parse
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
        "font.size": 8,
        "text.latex.preamble": "\\usepackage{amsmath}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
cm = 1 / 2.54
fig = plt.figure(figsize=(20 * cm, 8 * cm), dpi=1200)

gs = GridSpec(4, 2, width_ratios=[1, 2], height_ratios=[1, 4, 4, 4])

# 创建各个子图
ax_legend_map = fig.add_subplot(gs[0, 0])
ax_legend_light_curve = fig.add_subplot(gs[0, 1])
ax_map = fig.add_subplot(
    gs[1:, 0], projection=ccrs.PlateCarree(central_longitude=longitude)
)
ax_detail = fig.add_subplot(gs[1, 1])
ax_100ms = fig.add_subplot(gs[2, 1])
ax_1s = fig.add_subplot(gs[3, 1])

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
# ax_map.set_title("Signal Location", fontsize=10)
ax_map.set_xticks(np.arange(-10, 11, 10), crs=ccrs.PlateCarree())
ax_map.set_yticks(np.arange(15, 36, 10), crs=ccrs.PlateCarree())
ax_map.set_xticklabels(
    [
        f"{abs(int(x))}°{'W' if x < 0 else 'E' if x > 0 else ''}"
        for x in np.arange(-10, 11, 10)
    ]
)
ax_map.set_yticklabels(
    [
        f"{abs(int(y))}°{'S' if y < 0 else 'N' if y > 0 else ''}"
        for y in np.arange(15, 36, 10)
    ]
)
# orbit = json.loads(orbit)
# longitudes = [point["longitude"] for point in orbit]
# latitudes = [point["latitude"] for point in orbit]
# ax_map.plot(
#     longitudes,
#     latitudes,
#     color="C3",
#     linewidth=0.5,
#     transform=ccrs.PlateCarree(),
#     label="Orbit",
# )
lightnings = json.loads(lightnings)
longitudes_with_lightning = [lightning["lightning"]["lon"] for lightning in lightnings]
latitudes_with_lightning = [lightning["lightning"]["lat"] for lightning in lightnings]
ax_map.scatter(
    longitudes_with_lightning,
    latitudes_with_lightning,
    marker="o",
    s=5,
    facecolor="C4",
    edgecolors="None",
    transform=ccrs.PlateCarree(),
    label=f"Signal with Lightnings ({len(longitudes_with_lightning)})",
)
# draw a 800km radius circle around the signal point
circle = mpatches.Circle(
    (longitude, latitude),
    radius=800 / 6371 * 180 / np.pi,  # Convert km to degrees
    transform=ccrs.PlateCarree(),
    color="C5",
    alpha=0.3,
)
ax_map.add_patch(circle)

start_full = dateutil_parse(start_full)
events = json.loads(events)
ax_detail_twin = ax_detail.twinx()
for event in events:
    ax_detail_twin.scatter(
        (dateutil_parse(event["time"]).timestamp() - start_full.timestamp()) * 1000,
        event["channel"],
        marker="^" if event["info"]["scintillator"] == "NaI" else "o",
        s=20,
        facecolor="None",
        edgecolor=["C0" if event["info"]["acd"] == 0 else "C1"],
    )
ax_detail_twin.set_ylabel("Channel")
ax_detail.hist(
    [
        (dateutil_parse(event["time"]).timestamp() - start_full.timestamp()) * 1000
        for event in events
    ],
    histtype="step",
)
# ax_detail.set_xlabel("Time (ms)")
ax_detail.set_ylabel("Counts")

ax_100ms.stairs(
    json.loads(light_curve_100ms_unfiltered),
    np.linspace(
        -50,
        50,
        101,
    ),
    label="100ms Unfiltered",
    color="C0",
)
ax_100ms.stairs(
    json.loads(light_curve_100ms_filtered),
    np.linspace(
        -50,
        50,
        101,
    ),
    label="100ms Filtered",
    color="C1",
)
# ax_100ms.set_xlabel("Time (ms)")
ax_100ms.set_ylabel("Counts")
ax_100ms.set_xlim(-50, 50)

ax_1s.stairs(
    json.loads(light_curve_1s_unfiltered),
    np.linspace(
        -500,
        500,
        101,
    ),
    label="1s Unfiltered",
    color="C0",
)
ax_1s.stairs(
    json.loads(light_curve_1s_filtered),
    np.linspace(
        -500,
        500,
        101,
    ),
    label="1s Filtered",
    color="C1",
)
# ax_1s.set_xlabel("Time (ms)")
ax_1s.set_ylabel("Counts")
ax_1s.set_xlim(-500, 500)
ax_1s.set_xticks(
    [-500, -250, 0, 250, 500], labels=["-500", "-250", "0", "250", "500 (ms)"]
)

legend_map_handles = [
    mpatches.Patch(color="C0", label="Subsatellite"),
    mpatches.Patch(color="C1", label="800km Radius"),
    mpatches.Patch(color="C0", label="lightning"),
]
legend_light_curve_handles = [
    mpatches.Patch(color="C1", label="NaI"),
    mpatches.Patch(color="C2", label="CsI"),
    mpatches.Patch(color="C3", label="No Veto"),
    mpatches.Patch(color="C4", label="Veto"),
    mpatches.Patch(color="C5", label="Filtered"),
    mpatches.Patch(color="C6", label="Unfiltered"),
]
ax_legend_map.legend(
    handles=legend_map_handles,
    ncols=1,
    loc="center",
    frameon=False,
)
ax_legend_map.axis("off")
ax_legend_light_curve.legend(
    handles=legend_light_curve_handles,
    ncols=3,
    loc="center",
    frameon=False,
)
ax_legend_light_curve.axis("off")

plt.tight_layout()
plt.savefig("hxmt-catalog/output/detail.pdf", bbox_inches="tight")
