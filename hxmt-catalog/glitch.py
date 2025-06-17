from typing import Callable

import matplotlib.pyplot as plt
import numpy as np
from astropy.io import fits
from astropy.time import Time
from common import one_column_width


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
time_ref = isot_to_satellite_time("2017-06-27T14:08:39.626500Z")
cond = (time > time_ref - 20e-3) & (time < time_ref + 20e-3)
time = time[cond]
time = time - time_ref
channel = np.array(data["Events"].data["Channel"], dtype=np.int64)
channel = channel[cond]
channel[channel < 20] += 256  # 将通道号小于 20 的通道号加上 256


# 启用 LaTeX 渲染
plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}\n\\usepackage{wasysym}\\usepackage{CJKutf8}",  # 如果需要数学公式支持
    }
)
plt.figure(
    figsize=(one_column_width, (3 / 4) * one_column_width), dpi=1200, facecolor="none"
)

n, bins, patches = plt.hist(
    time,
    bins=np.linspace(-20e-3, 20e-3, 100),
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="/",
)
plt.xlim(-20e-3, 20e-3)
plt.xlabel("Time (s)")
plt.ylabel("Frequency")

twinx = plt.twinx()
sca = twinx.scatter(
    time, channel, s=0.5, facecolor="black", edgecolor="None", marker="o"
)
twinx.set_ylabel("Channel")

twinx.annotate(
    "Glitch",
    xy=(0.0, 25),
    xytext=(0.01, 180),
    arrowprops=dict(
        arrowstyle="->", lw=0.5, color="black", connectionstyle="arc3,rad=-0.1"
    ),
)
legend = plt.legend(
    handles=[patches[0], sca],
    labels=["Light Curve", "Events"],
    loc="center left",
    frameon=True,
    edgecolor="black",
    fancybox=False,
    framealpha=1.0,
)
legend.get_frame().set_linewidth(0.5)


plt.tight_layout()
plt.savefig(
    "hxmt-catalog/output/glitch.pdf",
    bbox_inches="tight",
    transparent=True,
)
