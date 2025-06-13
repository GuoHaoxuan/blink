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
        self.lightnings = False
        lightnings_json = json.loads(row[7])
        for lightning in lightnings_json:
            if lightning["is_associated"]:
                self.lightnings = True
                break
        self.satellite = row[8]
        self.detector = row[9]


def get_data():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT start, stop, fp_year, longitude, latitude, altitude, events, lightnings, satellite, detector
        FROM signal
        WHERE start < "2025-01-01"
        AND fp_year < 1e-3
        AND duration > 200e-6
        AND duration < 3e-3
        """
    )
    data = cursor.fetchall()
    conn.close()
    signals = [Signal(row) for row in data]
    return signals
