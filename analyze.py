import json
import math
import sqlite3
from dataclasses import dataclass
from datetime import datetime

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse


@dataclass
class Signal:
    start: datetime
    stop: datetime
    fp_year: float
    longitude: float
    latitude: float
    altitude: float
    events: str
    lightnings: str
    satellite: str
    detector: str

    def __init__(self, row):
        self.start = parse(row[0])
        self.stop = parse(row[1])
        self.fp_year = min(-math.log10(row[2]) if row[2] > 0 else 100, 14.5)
        self.longitude = float(row[3])
        self.latitude = float(row[4])
        self.altitude = float(row[5])
        self.events = row[6]
        self.lightnings = row[7]
        self.satellite = row[8]
        self.detector = row[9]


def plot_map(ax_drop, signals):
    ax_drop.set_extent([-180, 180, -43, 43], crs=ccrs.PlateCarree())
    ax_drop.coastlines(linewidth=1.5)
    len_signals = len(signals)
    ax_drop.scatter(
        [signal.longitude for signal in signals if signal.lightnings == "[]"],
        [signal.latitude for signal in signals if signal.lightnings == "[]"],
        s=1,
        c="C0",
        transform=ccrs.PlateCarree(),
        label=f"Signal ({len_signals})",
    )
    len_signals = len([signal for signal in signals if signal.lightnings != "[]"])
    ax_drop.scatter(
        [signal.longitude for signal in signals if signal.lightnings != "[]"],
        [signal.latitude for signal in signals if signal.lightnings != "[]"],
        s=1,
        c="C1",
        transform=ccrs.PlateCarree(),
        label=f"Signal with Lightnings ({len_signals})",
    )
    SAA_Lon_ARR_Raw = np.array(
        [-74.3, -88.2, -96, -92, -70, -45, -33, -15, 0.8, 18.2, 31, 27.3, 22, -74.3]
    )
    SAA_Lat_ARR_Raw = np.array(
        [-45, -28, -13, -9, -2.5, 3, 2.1, -15, -18.8, -23, -31, -39, -45, -45]
    )
    ax_drop.plot(
        SAA_Lon_ARR_Raw,
        SAA_Lat_ARR_Raw,
        color="black",
        linewidth=1,
        linestyle="--",
        label="SAA",
        transform=ccrs.PlateCarree(),
    )
    ax_drop.legend(
        loc="lower right",
        markerscale=5,
    )


def get_signals():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT start, stop, fp_year, longitude, latitude, altitude, events, lightnings, satellite, detector
        FROM signals
        WHERE start < "2025-01-01 00:00:00.000000000 UTC"
        """
    )
    data = cursor.fetchall()
    conn.close()
    signals = [Signal(row) for row in data]
    return signals


fig = plt.figure(figsize=(16, 9), dpi=300)
ax_drop = fig.add_subplot(1, 1, 1, projection=ccrs.PlateCarree(central_longitude=150))
signals = get_signals()
plot_map(ax_drop, signals)
plt.title("TGFs Found by HXMT between 2017-06-15 and 2024-12-31")
plt.savefig("signals_map.png", bbox_inches="tight")
