import json
import sqlite3

import cartopy.crs as ccrs
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse as dateutil_parse
from matplotlib.gridspec import GridSpec
from PIL import Image

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
    WHERE start_full = '2020-04-28T19:01:59.904861956Z' AND satellite = 'HXMT' AND detector = 'HE'
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

gs = GridSpec(2, 1, height_ratios=[1, 4])

ax_legend = fig.add_subplot(gs[0, 0])
ax_legend.axis("off")

ax_detail = fig.add_subplot(gs[1, 0])
ax_detail_twin = ax_detail.twinx()
start_full = dateutil_parse(start_full)
events = json.loads(events)
for event in events:
    ax_detail_twin.scatter(
        (dateutil_parse(event["time"]).timestamp() - start_full.timestamp()) * 1000,
        event["channel"],
        marker="^" if event["info"]["acd"] != 0 else "o",
        s=20,
        facecolor="None",
        edgecolor=["C2" if event["info"]["scintillator"] == "NaI" else "C0"],
        lw=0.5,
    )
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
        if event["info"]["scintillator"] == "NaI"
    ],
    bins=bins,
    histtype="step",
    color="C0",
    weights=[1000 / duration]
    * len([event for event in events if event["info"]["scintillator"] == "NaI"]),
)
ax_detail.hist(
    [
        (dateutil_parse(event["time"]).timestamp() - start_full.timestamp()) * 1000
        for event in events
        if event["info"]["scintillator"] == "CsI"
    ],
    bins=bins,
    histtype="step",
    color="C2",
    weights=[1000 / duration]
    * len([event for event in events if event["info"]["scintillator"] == "CsI"]),
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

plt.savefig("detail.pdf", bbox_inches="tight", dpi=1200)
