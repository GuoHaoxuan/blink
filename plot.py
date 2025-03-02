import matplotlib.pyplot as plt
import json
from dateutil import parser


def main():
    with open("result_9.json", "r") as file:
        data = json.load(file)

    # example 2023-01-01T00:01:49.647608064 UTC
    start = parser.isoparse(data["start"][:-4])
    stop = parser.isoparse(data["stop"][:-4])
    print(f"Start: {start}")
    print(f"Stop: {stop}")

    # 解析事件时间戳并绘制图表
    event_times = [
        (parser.isoparse(event["time"][:-4]) - start).total_seconds() * 1e3
        for event in data["events"]
    ]
    pha_values = [event["pha"] for event in data["events"]]

    plt.scatter(event_times, pha_values)
    plt.axvline(x=0, color="red", linestyle="--")
    plt.axvline(x=(stop - start).total_seconds() * 1e3, color="red", linestyle="--")
    plt.twinx()
    plt.hist(event_times, bins=50, alpha=0.3, color="gray", label="Event Histogram")
    plt.xlim(-1, (stop - start).total_seconds() * 1e3 + 1)
    plt.ylabel("Event Count")
    plt.legend(loc="upper right")
    plt.xlabel("Time (ms)")
    plt.ylabel("PHA")
    plt.title("PHA vs Time from result_0.json")
    plt.xticks(rotation=45)
    plt.tight_layout()
    plt.savefig("pha_vs_time.png")

    plt.close()


if __name__ == "__main__":
    main()
