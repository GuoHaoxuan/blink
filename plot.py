import matplotlib.pyplot as plt
import json
from dateutil import parser
import matplotlib.patches as mpatches


def main():
    with open("result_0.json", "r") as file:
        data = json.load(file)

    start = parser.isoparse(data["start"][:-4])
    stop = parser.isoparse(data["stop"][:-4])

    # 解析事件时间戳并绘制图表
    event_times = [
        (parser.isoparse(event["time"][:-4]) - start).total_seconds() * 1e3
        for event in data["events"]
    ]

    plt.figure(dpi=600)
    for time, event in zip(event_times, data["events"]):
        plt.plot(
            [time, time],
            [event["energy"]["start"], event["energy"]["stop"]],
            color="C0" if event["detector"][:3] == "NaI" else "C1",
        )

    plt.axvline(x=0, color="red", linestyle="--")
    plt.axvline(x=(stop - start).total_seconds() * 1e3, color="red", linestyle="--")
    plt.yscale("log")
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
    plt.xlim(-1, (stop - start).total_seconds() * 1e3 + 1)
    plt.ylabel("Event Count")
    handles = [
        plt.Line2D([0], [0], color="C0", label="NaI"),
        plt.Line2D([0], [0], color="C1", label="BGO"),
        mpatches.Patch(edgecolor="C2", label="Light Curve", fill=False),
    ]

    labels = [handle.get_label() for handle in handles]

    plt.legend(handles, labels)

    plt.tight_layout()
    plt.savefig("pha_vs_time.png")

    plt.close()


if __name__ == "__main__":
    main()
