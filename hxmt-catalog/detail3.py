import json
import sqlite3

import cartopy.crs as ccrs
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
    WHERE start_full = '2020-08-06T08:51:41.908577919Z' AND satellite = 'HXMT' AND detector = 'HE'
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
        "text.latex.preamble": "\\usepackage{amsmath}\\usepackage{siunitx}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
cm = 1 / 2.54
fig = plt.figure(figsize=(20 * cm, 7.25 * cm), dpi=1200)

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
    marker="x",
    s=20,
    color="C0",
    transform=ccrs.PlateCarree(),
)
# ax_map.set_title("Signal Location", fontsize=10)
ax_map.set_xticks(np.arange(70, 91, 10), crs=ccrs.PlateCarree())
ax_map.set_yticks(np.arange(25, 46, 10), crs=ccrs.PlateCarree())
ax_map.set_xticklabels(
    [
        f"{abs(int(x))}°{'W' if x < 0 else 'E' if x > 0 else ''}"
        for x in np.arange(70, 91, 10)
    ]
)
ax_map.set_yticklabels(
    [
        f"{abs(int(y))}°{'S' if y < 0 else 'N' if y > 0 else ''}"
        for y in np.arange(25, 46, 10)
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
    facecolor="C2",
    edgecolors="None",
    transform=ccrs.PlateCarree(),
)
# draw a 800km radius circle around the signal point
circle = mpatches.Circle(
    (longitude, latitude),
    radius=800 / 6371 * 180 / np.pi,  # Convert km to degrees
    transform=ccrs.PlateCarree(),
    edgecolor="#CCCCCC",
    facecolor="None",
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
        edgecolor=["C2" if event["info"]["acd"] == 0 else "C0"],
        lw=0.5,
    )
ax_detail_twin.axhline(38, color="#CCCCCC", linestyle="--", zorder=-1)
ax_detail_twin.set_ylabel("Channel")
duration = (
    dateutil_parse(stop_best).timestamp() - dateutil_parse(start_best).timestamp()
) * 1000
offset = (dateutil_parse(start_best).timestamp() - start_full.timestamp()) * 1000
bins = np.arange(
    offset - 30 * duration,
    offset + 30 * duration,
    duration,
)
ax_detail.hist(
    [
        (dateutil_parse(event["time"]).timestamp() - start_full.timestamp()) * 1000
        for event in events
    ],
    bins=bins,
    histtype="step",
    color="C0",
    weights=[1000 / duration] * len(events),
)
ax_detail.hist(
    [
        (dateutil_parse(event["time"]).timestamp() - start_full.timestamp()) * 1000
        for event in events
        if event["keep"]
    ],
    bins=bins,
    histtype="step",
    color="C2",
    weights=[1000 / duration] * len([event for event in events if event["keep"]]),
)
ax_detail.axvspan(0, offset, facecolor="C0", edgecolor="None", alpha=0.1, zorder=-2)
ax_detail.axvspan(
    offset + duration,
    (dateutil_parse(stop_full).timestamp() - start_full.timestamp()) * 1000,
    facecolor="C0",
    edgecolor="None",
    alpha=0.1,
    zorder=-2,
)
ax_detail.axvspan(
    offset,
    offset + duration,
    facecolor="C2",
    edgecolor="None",
    alpha=0.2,
    zorder=-2,
)
ax_detail.axhline(background, color="#CCCCCC", linestyle="-", zorder=-1)
ax_detail.set_xlim(
    (dateutil_parse(events[0]["time"]).timestamp() - start_full.timestamp()) * 1000,
    (dateutil_parse(events[-1]["time"]).timestamp() - start_full.timestamp()) * 1000,
)
ax_detail.yaxis.set_major_formatter(lambda x, pos: f"$\\num{{{x:.0f}}}$")
# ax_detail.set_xlabel("Time (ms)")
ax_detail.set_ylabel("CPS (\\unit{\\per\\second})")
ax_detail.xaxis.set_major_formatter(
    plt.FuncFormatter(lambda x, _: f"$\\SI{{{x}}}{{\\milli\\second}}$")
)

ax_100ms.stairs(
    np.array(json.loads(light_curve_100ms_unfiltered)) / 1e-3,
    np.linspace(
        -50,
        50,
        101,
    ),
    label="100ms Unfiltered",
    color="C0",
)
ax_100ms.stairs(
    np.array(json.loads(light_curve_100ms_filtered)) / 1e-3,
    np.linspace(
        -50,
        50,
        101,
    ),
    label="100ms Filtered",
    color="C2",
)
ax_100ms.axhline(background, color="#CCCCCC", linestyle="-", zorder=-1)
# ax_100ms.set_xlabel("Time (ms)")
ax_100ms.yaxis.set_major_formatter(lambda x, pos: f"$\\num{{{x:.0f}}}$")
ax_100ms.set_ylabel("CPS (\\unit{\\per\\second})")
ax_100ms.set_xlim(-50, 50)
ax_100ms.xaxis.set_major_locator(plt.MultipleLocator(25))
ax_100ms.xaxis.set_major_formatter(
    plt.FuncFormatter(lambda x, _: f"$\\SI{{{x:.0f}}}{{\\milli\\second}}$")
)

ax_1s.stairs(
    np.array(json.loads(light_curve_1s_unfiltered)) / 1e-2,
    np.linspace(
        -500,
        500,
        101,
    ),
    label="1s Unfiltered",
    color="C0",
)
ax_1s.stairs(
    np.array(json.loads(light_curve_1s_filtered)) / 1e-2,
    np.linspace(
        -500,
        500,
        101,
    ),
    label="1s Filtered",
    color="C2",
)
ax_1s.axhline(background, color="#CCCCCC", linestyle="-", zorder=-1)
# ax_1s.set_xlabel("Time (ms)")
ax_1s.yaxis.set_major_formatter(lambda x, pos: f"$\\num{{{x:.0f}}}$")
ax_1s.set_ylabel("CPS (\\unit{\\per\\second})")
ax_1s.set_xlim(-500, 500)
ax_1s.xaxis.set_major_locator(plt.MultipleLocator(250))
ax_1s.xaxis.set_major_formatter(
    plt.FuncFormatter(lambda x, _: f"$\\SI{{{x:.0f}}}{{\\milli\\second}}$")
)

legend_map_handles = [
    plt.Line2D(
        [],
        [],
        markerfacecolor="C0",
        marker="x",
        linestyle="None",
        label="Sub-satellite",
    ),
    plt.Line2D(
        [],
        [],
        color="#CCCCCC",
        linestyle="-",
        label="\\SI{800}{\\kilo\\meter} Radius",
    ),
    plt.Line2D(
        [],
        [],
        markerfacecolor="C2",
        markeredgecolor="None",
        marker="o",
        linestyle="None",
        label="Lightning",
    ),
]
legend_light_curve_handles = [
    plt.Line2D(
        [],
        [],
        markeredgecolor="black",
        markerfacecolor="None",
        marker="^",
        linestyle="None",
        label="NaI",
    ),
    plt.Line2D(
        [],
        [],
        markeredgecolor="black",
        markerfacecolor="None",
        marker="o",
        linestyle="None",
        label="CsI",
    ),
    mpatches.Patch(color="C0", label="Veto"),
    mpatches.Patch(color="C2", label="No Veto"),
    mpatches.Patch(edgecolor="C0", facecolor="None", label="Unfiltered"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="Filtered"),
    plt.Line2D(
        [],
        [],
        color="#CCCCCC",
        linestyle="-",
        label="Background",
    ),
    plt.Line2D(
        [],
        [],
        color="#CCCCCC",
        linestyle="--",
        label="Threshold",
    ),
    mpatches.Patch(facecolor="C0", edgecolor="None", alpha=0.1, label="Full Duration"),
    mpatches.Patch(facecolor="C2", edgecolor="None", alpha=0.2, label="Best Duration"),
]
ax_legend_map.legend(
    handles=legend_map_handles,
    ncols=2,
    loc="center",
    frameon=False,
)
ax_legend_map.axis("off")
ax_legend_light_curve.legend(
    handles=legend_light_curve_handles,
    ncols=5,
    loc="center",
    frameon=False,
)
ax_legend_light_curve.axis("off")

plt.tight_layout()
plt.savefig("hxmt-catalog/output/detail3.pdf", bbox_inches="tight")
