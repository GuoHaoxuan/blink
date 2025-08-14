import sqlite3

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from scipy.optimize import curve_fit
from scipy.stats import poisson


def false_positive_per_year(mean: float, count: int, duration: float) -> float:
    """
    计算每年的误报率
    :param sf: 生存函数值
    :param duration: 信号持续时间（秒）
    :return: 每年的误报率
    """
    return (
        poisson.sf(count, mean) * 365.25 * 24 * 3600 / duration
    )  # 每年秒数为 365.25 天


def power_law(x, a, b):
    return a * x**b


plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
cm = 1 / 2.54
plt.figure(figsize=(20 * cm, 7 * cm), dpi=1200)

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()

# 全部的
cursor.execute(
    "SELECT 200e-6, count_filtered_best, 7000, coincidence_probability, false_positive_per_year FROM signal WHERE start_full < '2025-01-01'"
)
data = cursor.fetchall()
fp_years = [false_positive_per_year(row[2] * row[0], row[1], row[0]) for row in data]
fp_years = np.array(fp_years)
fp_years2 = np.array([row[4] for row in data])
print(fp_years)
print(fp_years2)
coincidence_probability = np.array([row[3] for row in data])
min_fp = np.min(fp_years[fp_years > 0])
print(min_fp)
max_fp = np.max(fp_years)
bins = 200
fp_bins = np.logspace(np.log10(min_fp), np.log10(20), bins + 1)
n_all, _, _ = plt.hist(
    fp_years,
    bins=fp_bins,
    histtype="step",
    label="All Signals",
)

# 误关联
misassociated_count = np.zeros_like(fp_bins[:-1])
for fp_year, prob in zip(fp_years, coincidence_probability):
    index = np.digitize(fp_year, fp_bins) - 1
    if index < len(misassociated_count):
        misassociated_count[index] += prob
plt.stairs(
    misassociated_count,
    fp_bins,
    fill=False,
    edgecolor="C1",
)

# 闪电关联
cursor.execute(
    "SELECT 200e-6, count_filtered_best, 7000 FROM signal WHERE start_full < '2025-01-01' AND associated_lightning_count > 0"
)
data = cursor.fetchall()
fp_years = [false_positive_per_year(row[2] * row[0], row[1], row[0]) for row in data]
fp_years = np.array(fp_years)
n_associated, _, _ = plt.hist(
    fp_years,
    bins=fp_bins,
    histtype="step",
    label="Signals with Lightning",
    edgecolor="C2",
    alpha=0.5,
)

# 闪电关联 - 误关联
n_diff = n_associated - misassociated_count
plt.stairs(
    n_diff,
    fp_bins,
    fill=False,
    edgecolor="C2",
)

# 拟合
condition = ((fp_bins[:-1] + fp_bins[1:]) / 2 < 1e-4) & (
    (fp_bins[:-1] + fp_bins[1:]) / 2 > 1e-15
)
# plt.axvspan(1e-15, 1e-4, facecolor="C0", edgecolor="None", alpha=0.1, zorder=-2)
curve_fit_params_all_left, _ = curve_fit(
    power_law,
    ((fp_bins[:-1] + fp_bins[1:]) / 2)[condition],
    n_all[condition],
)
print(curve_fit_params_all_left)
x_fit = np.logspace(np.log10(min_fp), np.log10(max_fp), 100)
y_fit = power_law(x_fit, *curve_fit_params_all_left)
plt.plot(x_fit, y_fit, color="#CCCCCC", linestyle="--", zorder=-1)

condition = ((fp_bins[:-1] + fp_bins[1:]) / 2 < 1e-25) & (
    (fp_bins[:-1] + fp_bins[1:]) / 2 > 1e-60
)
curve_fit_params_all_right, _ = curve_fit(
    power_law,
    ((fp_bins[:-1] + fp_bins[1:]) / 2)[condition],
    n_all[condition],
    p0=curve_fit_params_all_left,
)
print(curve_fit_params_all_right)
x_fit = np.logspace(np.log10(min_fp), np.log10(max_fp), 100)
y_fit = power_law(x_fit, *curve_fit_params_all_right)
plt.plot(x_fit, y_fit, color="#CCCCCC", linestyle="--", zorder=-1)
# plt.fill_between(x_fit, y_fit, 1e-1, color="C0", alpha=0.1, zorder=-2)
estimated_tgfs = 0
for x in (fp_bins[:-1] + fp_bins[1:]) / 2:
    estimated_tgfs += power_law(x, *curve_fit_params_all_right)
print(f"Estimated TGFs: {estimated_tgfs}")

condition = ((fp_bins[:-1] + fp_bins[1:]) / 2 < 1e-2) & (
    (fp_bins[:-1] + fp_bins[1:]) / 2 > 1e-50
)
curve_fit_params_associated, _ = curve_fit(
    power_law,
    ((fp_bins[:-1] + fp_bins[1:]) / 2)[condition],
    n_associated[condition],
    p0=curve_fit_params_all_right,
)
x_fit = np.logspace(np.log10(min_fp), np.log10(max_fp), 100)
y_fit = power_law(x_fit, *curve_fit_params_associated)
plt.plot(x_fit, y_fit, color="#CCCCCC", linestyle="--", zorder=-1)


plt.ylim(0.5, 1e6)

# 手动创建图例句柄
legend_handles = [
    mpatches.Patch(edgecolor="C0", facecolor="None", label="All Signals"),
    mpatches.Patch(edgecolor="C1", facecolor="None", label="Mis-associated Signals"),
    plt.Line2D([0], [0], color="#CCCCCC", linestyle="--", label="Power Law Fits"),
    mpatches.Patch(
        edgecolor="C2", facecolor="None", alpha=0.5, label="Signals with Lightning"
    ),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="Signals with Lightning"),
    # mpatches.Patch(facecolor="C0", edgecolor="None", alpha=0.1, label="Estimated TGFs"),
]
plt.legend(handles=legend_handles, ncols=2, loc="upper right")
plt.xlabel(
    "Expected Annual False Positive Under Poisson Assumption (7000 cps Fake Background)"
)
plt.ylabel("Number")

conn.close()

plt.xscale("log")
plt.yscale("log")
plt.xlim(max_fp, min_fp)
plt.savefig("hxmt-catalog/output/fp-distribution3-fake-bg.pdf", bbox_inches="tight")
