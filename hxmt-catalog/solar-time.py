import sqlite3
from dataclasses import dataclass
from datetime import datetime

import matplotlib.pyplot as plt
import numpy as np
from common import one_column_width
from dateutil.parser import parse


@dataclass
class Signal:
    apparent_solar_time: datetime

    def __init__(self, row):
        self.apparent_solar_time = parse(row[0])


def get_data():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT apparent_solar_time
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


# 启用 LaTeX 渲染
plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": r"\usepackage{amsmath}",  # 如果需要数学公式支持
    }
)

data = get_data()
plt.figure(figsize=(one_column_width, one_column_width), dpi=1200, facecolor="none")

# 创建极坐标子图
ax = plt.subplot(111, projection="polar")

# 计算直方图数据
hours = [
    signal.apparent_solar_time.hour + signal.apparent_solar_time.minute / 60
    for signal in data
]
counts, bin_edges = plt.hist(hours, bins=24, range=(0, 24))[:2]
plt.clf()  # 清除直角坐标的直方图

# 重新创建极坐标子图
ax = plt.subplot(111, projection="polar")

# 将小时转换为角度（0小时在顶部，顺时针）
bin_centers = (bin_edges[:-1] + bin_edges[1:]) / 2
theta = (bin_centers / 24) * 2 * np.pi - np.pi / 2  # -π/2 使0点在顶部

# 绘制极坐标柱状图
bars = ax.bar(
    theta,
    counts,
    width=2 * np.pi / 24,
    bottom=0,
    edgecolor="black",
    facecolor="None",
    # hatch="/",
)

# 隐藏径向刻度和网格
ax.set_rticks([])  # 隐藏径向刻度
ax.grid(False)  # 隐藏所有网格线

# 在每个扇形外侧显示数值
max_count = max(counts)
# 设置径向范围，确保标签不会超出圆圈
ax.set_ylim(0, max_count * 1.2)
for i, (angle, count) in enumerate(zip(theta, counts)):
    if count > 0:  # 只为非零值添加标签
        # 计算标签位置（在柱子外侧，但更紧凑）
        label_radius = count + max_count * 0.1
        ax.text(
            angle, label_radius, f"{int(count)}", ha="center", va="center", fontsize=8
        )

# 设置角度刻度
ax.set_theta_zero_location("N")  # 0度在顶部
ax.set_theta_direction(-1)  # 顺时针方向
ax.set_thetagrids([0, 90, 180, 270], ["0", "6", "12", "18"])

# 移除原有的标签设置，因为不再需要径向标签
# ax.set_xlabel("Apparent Solar Time (hours)")

plt.tight_layout()
plt.savefig(
    "hxmt-catalog/output/solar-time.pdf",
    bbox_inches="tight",
    transparent=True,
)
