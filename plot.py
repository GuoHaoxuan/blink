import json
import sqlite3
from dataclasses import dataclass
from datetime import datetime
from typing import List

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
from dateutil import parser
from tqdm import tqdm


@dataclass
class Event:
    time: datetime
    energy: List[float]
    detector: str


@dataclass
class Signal:
    start: datetime
    stop: datetime
    fp_year: float
    longitude: float
    latitude: float
    altitude: float
    events: List[Event]
    lightnings: str
    background: float


def main():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        "SELECT start, stop, fp_year, longitude, latitude, altitude, events, lightnings, background FROM signals WHERE start < '2025-01-01' AND fp_year > 10 AND lightnings != '[]'"
    )
    data = cursor.fetchall()
    conn.close()
    signals = [
        {
            "start": parser.parse(row[0]),
            "stop": parser.parse(row[1]),
            "fp_year": float(row[2]),
            "longitude": float(row[3]),
            "latitude": float(row[4]),
            "altitude": float(row[5]),
            "events": json.loads(row[6]),
            "lightnings": row[7],
            "background": float(row[8])
            * (
                ((parser.parse(row[1]) - parser.parse(row[0])).total_seconds() + 1e-3)
                / 50
            ),
        }
        for row in data
    ]
    for data in tqdm(signals):
        start = data["start"]
        stop = data["stop"]

        # 解析事件时间戳并绘制图表
        event_times = [
            (parser.isoparse(event["time"]) - start).total_seconds() * 1e3
            for event in data["events"]
        ]

        plt.figure(dpi=600)
        for time, event in zip(event_times, data["events"]):
            # print(event)
            plt.scatter(
                [time],
                [event["energy"][0]],
                color="C1" if "true" in event["detector"] else "C0",
                marker="^" if "NaI" in event["detector"] else "o",
            )

        plt.axvline(x=0, color="red", linestyle="--")
        plt.axvline(x=(stop - start).total_seconds() * 1e3, color="red", linestyle="--")
        plt.axhline(y=38, color="red", linestyle="--")
        # plt.yscale("log")
        plt.ylabel("Energy (keV)")
        plt.xlabel("Time (ms)")

        plt.twinx()
        plt.hist(
            event_times,
            bins=50,
            color="C2",
            histtype="step",
            alpha=0.3,
        )
        plt.axhline(y=data["background"], color="blue", linestyle="--")
        plt.xlim(-1, (stop - start).total_seconds() * 1e3 + 1)
        plt.ylabel("Event Count")
        handles = [
            plt.Line2D([0], [0], color="C0", label="Photon"),
            plt.Line2D([0], [0], color="C1", label="Electron"),
            plt.Line2D(
                [0], [0], marker="^", color="black", label="NaI", linestyle="None"
            ),
            plt.Line2D(
                [0], [0], marker="o", color="black", label="CsI", linestyle="None"
            ),
            mpatches.Patch(edgecolor="C2", label="Light Curve", fill=False),
            plt.Line2D(
                [0],
                [0],
                color="blue",
                linestyle="--",
                label="Background ({:.2f})".format(data["background"]),
            ),
            plt.Line2D([0], [0], color="red", linestyle="--", label="drop below"),
        ]

        labels = [handle.get_label() for handle in handles]

        plt.legend(handles, labels)

        plt.tight_layout()
        plt.savefig(
            "lightcurve/" + data["start"].strftime("%Y-%m-%d_%H-%M-%S") + ".png"
        )

        plt.close()


if __name__ == "__main__":
    main()
