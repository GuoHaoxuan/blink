import matplotlib.pyplot as plt
from astropy.io import fits
from typing import Callable
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


def main():
    filename = "HXMT_20221009T13_HE-Evt_FFFFFF_V1_1K.FITS"
    start = isot_to_satellite_time("2022-10-09T13:20:00.000")
    stop = isot_to_satellite_time("2022-10-09T13:22:00.000")
    data = fits.open(filename)
    events = data[1].data
    print(start, stop, events["Time"].min(), events["Time"].max())
    time = events["Time"][(events["Time"] >= start) & (events["Time"] <= stop)] - start
    plt.hist(time, bins=10000, histtype="step", color="black")
    plt.xlabel("Time (s)")
    plt.ylabel("Counts")
    plt.title("Light Curve")
    plt.show()


if __name__ == "__main__":
    main()
