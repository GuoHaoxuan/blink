import sqlite3

import matplotlib.pyplot as plt
import numpy as np
from data import Signal


def get_data_all():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT start, stop, fp_year, longitude, latitude, altitude, events, lightnings, satellite, detector, count_best
        FROM signal
        WHERE start < "2025-01-01"
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

cm = 1 / 2.54  # 将厘米转换为英寸
data = get_data_all()
plt.figure(figsize=(8 * cm, 6 * cm), dpi=1200, facecolor="none")

# 计算数据的对数值
# 创建对数均匀的bin边界


fp_values = np.array([signal.fp_year for signal in data], dtype=np.float64)
fp_values = np.log10(fp_values)  # 对fp_year取对数
count_best_values = np.array([signal.count_best for signal in data], dtype=np.float64)

plt.scatter(
    fp_values,
    count_best_values,
    s=0.5,  # 点的大小
    edgecolor="black",
    facecolor="None",
    label="All Signals",
)

plt.xscale("log")  # 设置x轴为对数刻度
plt.yscale("log")
plt.xlabel("log(FP Year)")
plt.ylabel("Count Best")
plt.tight_layout()
plt.savefig("hxmt-catalog/output/fp-distribution.pdf", transparent=True)
