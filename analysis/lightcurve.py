import matplotlib.pyplot as plt
from astropy.io import fits
from typing import Callable
from astropy.time import Time
import numpy as np


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


def main():
    filename = "HXMT_20200415T08_HE-Evt_FFFFFF_V2_1K.FITS"
    start = isot_to_satellite_time("2020-04-15T08:48:03.564")
    stop = isot_to_satellite_time("2020-04-15T08:48:07.564")
    data = fits.open(filename)
    events = data[1].data
    print(start, stop, events["Time"].min(), events["Time"].max())
    time = events["Time"][(events["Time"] >= start) & (events["Time"] <= stop)] - start
    plt.hist(time, bins=10000, histtype="step", color="black")

    # saturation = []
    # with open("res.txt", "r") as f:
    #     for line in f:
    #         if line.startswith("true"):
    #             saturation.append(1)
    #         else:
    #             saturation.append(0)
    # saturation_index = np.linspace(0, stop - start, len(saturation))

    # saturation_values = np.array(saturation) * plt.ylim()[1]
    # plt.step(
    #     saturation_index,
    #     saturation_values,
    #     where="post",
    #     color="red",
    #     label="Saturation",
    # )
    # plt.fill_between(
    #     saturation_index,
    #     0,
    #     saturation_values,
    #     where=None,
    #     step="post",
    #     color="red",
    #     alpha=0.3,
    # )
    plt.xlabel("Time (s)")
    plt.ylabel("Counts")
    plt.title("Light Curve")
    plt.savefig("lightcurve.png", dpi=300)


if __name__ == "__main__":
    main()
