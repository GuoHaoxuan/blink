from typing import Callable

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
time_ref = isot_to_satellite_time("2017-06-27T14:08:39.627Z")
cond = (time > time_ref - 20e-3) & (time < time_ref + 20e-3) & (pulse_width >= 75)
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
        "text.latex.preamble": "\\usepackage{amsmath}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
cm = 1 / 2.54
plt.figure(figsize=(20 * cm, 5 * cm), dpi=1200)

n, bins, patches = plt.hist(
    time,
    bins=np.linspace(-20e-3, 20e-3, 100),
    histtype="step",
    edgecolor="C0",
)
plt.xlim(-20e-3, 20e-3)
plt.xlabel("Time Since 2017-06-27T14:08:39.627Z (s)")
plt.ylabel("Frequency")

twinx = plt.twinx()
sca = twinx.scatter(time, channel, s=20, facecolor="None", edgecolor="C1", marker="o")
threshold = plt.axhline(
    38, color="C3", linestyle="--", linewidth=0.5, label="Channel 38"
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
    handles=[patches[0], sca, threshold],
    labels=["Light Curve", "HE CsI Event", "Channel 38"],
    loc="upper left",
)

plt.tight_layout()
plt.savefig(
    "hxmt-catalog/output/spike.pdf",
    bbox_inches="tight",
    transparent=True,
)
