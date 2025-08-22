from typing import Callable

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from astropy.io import fits
from astropy.time import Time


def isot_to_satellite_time_helper(
    date_ref: str,
) -> Callable[[str], float]:
    def isot_to_satellite_time(isot: str) -> float:
        return (
            Time(isot, format="isot", scale="utc")
            - Time(date_ref, format="isot", scale="utc")
        ).sec

    return isot_to_satellite_time


isot_to_satellite_time = isot_to_satellite_time_helper("2012-01-01T00:00:00.000")

data = fits.open("HXMT_20170627T14_HE-Evt_FFFFFF_V1_1K.FITS")
time = data["Events"].data["Time"]
pulse_width = data["Events"].data["Pulse_Width"]
acd = data["Events"].data["ACD"]
event_type = data["Events"].data["Event_Type"]
# print(data["Events"].data.columns)

time_ref = isot_to_satellite_time("2017-06-27T14:08:39.627Z")
cond = (time > time_ref - 20e-3) & (time < time_ref + 20e-3)
time = time[cond]
time = time - time_ref
channel = np.array(data["Events"].data["Channel"], dtype=np.int64)
channel = channel[cond]
channel[channel < 20] += 256  # 将通道号小于 20 的通道号加上 256
acd = acd[cond]
pulse_width = pulse_width[cond]
event_type = event_type[cond]


# 启用 LaTeX 渲染
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
fig = plt.figure(figsize=(20 * cm, 5 * cm), dpi=1200)
gs = fig.add_gridspec(2, 1, height_ratios=[1, 3.5], hspace=0, wspace=0)

plt.subplot(gs[1])

n, bins, patches = plt.hist(
    time,
    bins=np.linspace(-20e-3, 20e-3, 100),
    histtype="step",
    edgecolor="C0",
)
cond2 = (pulse_width >= 75) & (event_type == 0) & (channel >= 38)
plt.hist(
    time[cond2],
    bins=np.linspace(-20e-3, 20e-3, 100),
    histtype="step",
    edgecolor="C2",
)

plt.xlim(-20e-3, 20e-3)
plt.gca().xaxis.set_major_formatter(
    lambda x, _: f"\\SI{{{x * 1e3:.0f}}}{{\\milli\\second}}"
)
plt.xlabel("Time Since 2017-06-27 14:08:39.627 UTC")
plt.ylabel("Frequency")

twinx = plt.twinx()
for event in zip(time, channel, acd, pulse_width, event_type):
    twinx.scatter(
        event[0],
        event[1],
        marker="P" if event[4] == 1 else "o" if event[3] >= 75 else "^",
        s=20,
        facecolor="None",
        edgecolor="black" if event[4] == 1 else "C0" if any(event[2]) else "C2",
        linewidth=0.5,
    )
threshold = plt.axhline(38, color="#CCCCCC", linestyle="--", label="Channel 38")
twinx.set_ylabel("Channel")
plt.axvspan(
    bins[43],
    bins[53],
    facecolor="C1",
    edgecolor="None",
    alpha=0.1,
    zorder=-2,
)

plt.subplot(gs[0])
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
    plt.Line2D(
        [],
        [],
        markeredgecolor="black",
        markerfacecolor="None",
        marker="P",
        linestyle="None",
        label="Am-241",
    ),
    mpatches.Patch(color="black", label="Am-241"),
    mpatches.Patch(color="C0", label="Veto"),
    mpatches.Patch(color="C2", label="No Veto"),
    mpatches.Patch(edgecolor="C0", facecolor="None", label="Unfiltered"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="Filtered"),
    plt.Line2D(
        [],
        [],
        color="#CCCCCC",
        linestyle="--",
        label="Threshold",
    ),
    mpatches.Patch(facecolor="C1", edgecolor="None", alpha=0.1, label="Spike"),
]
plt.legend(
    handles=legend_light_curve_handles,
    ncols=5,
    loc="center",
    frameon=False,
)
plt.axis("off")

plt.tight_layout()
plt.savefig(
    "hxmt-catalog/output/spike2.pdf",
    bbox_inches="tight",
    transparent=True,
)
