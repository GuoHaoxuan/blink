from typing import Callable

import matplotlib.pyplot as plt
import numpy as np
from astropy.io import fits
from astropy.time import Time, TimeDelta


def satellite_time_to_isot_helper(
    date_ref: str,
) -> Callable[[float], str]:
    def satellite_time_to_isot(satellite_time: float) -> str:
        return (
            (
                Time(date_ref, format="isot", scale="utc")
                + TimeDelta(satellite_time, format="sec", scale="tai")
            )
            .to_datetime()
            .strftime("%Y-%m-%dT%H:%M:%S.%f")
        )

    return satellite_time_to_isot


def isot_to_satellite_time_helper(
    date_ref: str,
) -> Callable[[str], float]:
    def isot_to_satellite_time(isot: str) -> float:
        return (
            Time(isot, format="isot", scale="utc")
            - Time(date_ref, format="isot", scale="utc")
        ).sec

    return isot_to_satellite_time


satellite_time_to_isot = satellite_time_to_isot_helper("2012-01-01T00:00:00.000")
isot_to_satellite_time = isot_to_satellite_time_helper("2012-01-01T00:00:00.000")


def satellite_time_to_iso8601(satellite_time: float) -> str:
    return satellite_time_to_isot(satellite_time) + "Z"


data = fits.open("HXMT_20180826T12_HE-Evt_FFFFFF_V1_1K.FITS")
time = data["Events"].data["Time"]
print("[DEBUG]", len(time), "events")
pulse_width = data["Events"].data["Pulse_Width"]
time_ref = isot_to_satellite_time("2018-08-26T12:31:59.217Z")
cond = (time > time_ref - 50e-3) & (time < time_ref + 50e-3) & (pulse_width >= 75)
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
    bins=np.linspace(-50e-3, 50e-3, 100),
    histtype="step",
    edgecolor="C0",
)
plt.xlim(-50e-3, 50e-3)
plt.xlabel("Time Since 2018-08-26T12:31:59.217Z (s)")
plt.ylabel("Frequency")

twinx = plt.twinx()
sca = twinx.scatter(time, channel, s=5, facecolor="C1", edgecolor="None", marker="o")
twinx.set_ylabel("Channel")

legend = plt.legend(
    handles=[patches[0], sca],
    labels=["Light Curve", "HE CsI Event"],
    loc="upper left",
)

plt.tight_layout()
plt.savefig(
    "hxmt-catalog/output/saturation.pdf",
    bbox_inches="tight",
    transparent=True,
)
