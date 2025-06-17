import json
import sqlite3
from dataclasses import dataclass

import matplotlib.pyplot as plt
import numpy as np
from common import one_column_width


@dataclass
class Signal:
    fp_year: float
    lightnings: bool

    def __init__(self, row):
        self.fp_year = row[0]
        self.lightnings = False
        lightnings_json = json.loads(row[1])
        for lightning in lightnings_json:
            if lightning["is_associated"]:
                self.lightnings = True
                break


def get_data_all():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT fp_year, lightnings
        FROM signal
        WHERE start < "2025-01-01"
        """
    )
    data = cursor.fetchall()
    conn.close()
    signals = [Signal(row) for row in data]
    return signals


def get_data_ranged():
    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT fp_year, lightnings
        FROM signal
        WHERE start < "2025-01-01"
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
        "text.latex.preamble": "\\usepackage{amsmath}\n\\usepackage{wasysym}\\usepackage{CJKutf8}",  # 如果需要数学公式支持
    }
)

cm = 1 / 2.54  # 将厘米转换为英寸
data = get_data_all()
data_ranged = get_data_ranged()
plt.figure(
    figsize=(one_column_width, (3 / 4) * one_column_width), dpi=1200, facecolor="none"
)

# 计算数据的对数值
# 创建对数均匀的bin边界
min_fp = 1e-20
max_fp = 20

fp_values = np.array([signal.fp_year for signal in data], dtype=np.float64)
log_bins = np.logspace(np.log10(min_fp), np.log10(max_fp), 20)
n_all, bins, patches = plt.hist(
    fp_values,
    bins=log_bins,
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="/",
)
fp_values_ranged = np.array(
    [signal.fp_year for signal in data_ranged], dtype=np.float64
)
n_ranged, bins_ranged, patches_ranged = plt.hist(
    fp_values_ranged,
    bins=log_bins,
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="//",
)

fp_values = np.array(
    [signal.fp_year for signal in data if signal.lightnings], dtype=np.float64
)
log_bins = np.logspace(np.log10(min_fp), np.log10(max_fp), 20)
n_lightning, bins, patches_lightning = plt.hist(
    fp_values,
    bins=log_bins,
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="\\",
)
fp_values_ranged = np.array(
    [signal.fp_year for signal in data_ranged if signal.lightnings],
    dtype=np.float64,
)
n_lightning_ranged, bins_ranged, patches_lightning_ranged = plt.hist(
    fp_values_ranged,
    bins=log_bins,
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="\\\\",
)


plt.xscale("log")  # 设置x轴为对数刻度
plt.yscale("log")
plt.xlim(min_fp, max_fp)
plt.xlabel("FP (year$^{-1}$)")
plt.ylabel("Frequency")
plt.gca().invert_xaxis()  # 翻转x轴，使小值在右边

twinx = plt.twinx()  # 创建双y轴
ratio = twinx.plot(
    (bins[:-1] + bins[1:]) / 2,
    n_lightning / n_all,
    color="black",
    linestyle="--",
)
ratio_filtered = twinx.plot(
    (bins_ranged[:-1] + bins_ranged[1:]) / 2,
    n_lightning_ranged / n_ranged,
    color="black",
    linestyle=":",
)
# 设置双y轴的标签
twinx.set_ylabel("Lightning Ratio")
# 设置双y的y轴范围
twinx.set_ylim(0, 1)

# 修改图例样式以满足lineartwok要求 - 使用正确的参数
legend = plt.legend(
    handles=[
        patches[0],
        patches_ranged[0],
        patches_lightning[0],
        patches_lightning_ranged[0],
        ratio[0],
        ratio_filtered[0],
    ],
    labels=[
        "\\begin{CJK*}{UTF8}{gbsn}正\\end{CJK*}",
        "\\begin{CJK*}{UTF8}{gbsn}正\\end{CJK*}\\clock",
        "\\begin{CJK*}{UTF8}{gbsn}正\\end{CJK*}\\lightning",
        "\\begin{CJK*}{UTF8}{gbsn}正\\end{CJK*}\\lightning\\clock",
        "\\%",
        "\\%\\clock",
    ],
    frameon=True,
    edgecolor="black",
    fancybox=False,
    framealpha=1.0,
    loc="upper right",
    ncols=2,
    columnspacing=0.5,
)
# 单独设置边框线宽
legend.get_frame().set_linewidth(0.5)

plt.axvline(1e-3, color="black", linestyle="--", linewidth=0.5)

plt.tight_layout()
plt.savefig(
    "hxmt-catalog/output/fp-distribution.pdf", bbox_inches="tight", transparent=True
)
