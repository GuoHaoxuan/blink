import sqlite3

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from scipy.optimize import curve_fit


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
    "SELECT fp_year, coincidence_probability FROM signal WHERE start < '2025-01-01'"
)
data = cursor.fetchall()
fp_years = [row[0] for row in data]
fp_years = np.array(fp_years)
min_fp = 1e-30
max_fp = np.max(fp_years)
bins = 100
fp_bins = np.logspace(np.log10(min_fp), np.log10(max_fp), bins + 1)
n_all, _, _ = plt.hist(
    fp_years,
    bins=fp_bins,
    histtype="step",
    label="All Signals",
)

# 误关联
misassociated_count = np.zeros_like(fp_bins[:-1])
for fp_year, prob in data:
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
    "SELECT fp_year FROM signal WHERE start < '2025-01-01' AND associated_lightning_count > 0"
)
data = cursor.fetchall()
fp_years = [row[0] for row in data]
fp_years = np.array(fp_years)
n_associated, _, _ = plt.hist(
    fp_years,
    bins=fp_bins,
    histtype="step",
    label="Signals with Lightning",
)

# 拟合
condition = (fp_bins[:-1] + fp_bins[1:]) / 2 > 1e-3
curve_fit_params_all_left, _ = curve_fit(
    power_law,
    ((fp_bins[:-1] + fp_bins[1:]) / 2)[condition],
    n_all[condition],
)
print(curve_fit_params_all_left)
x_fit = np.logspace(np.log10(min_fp), np.log10(max_fp), 100)
y_fit = power_law(x_fit, *curve_fit_params_all_left)
plt.plot(x_fit, y_fit, color="#CCCCCC", linestyle="--", zorder=-1)

condition = ((fp_bins[:-1] + fp_bins[1:]) / 2 < 1e-8) & (
    (fp_bins[:-1] + fp_bins[1:]) / 2 > 1e-30
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

condition = ((fp_bins[:-1] + fp_bins[1:]) / 2 < 1e-2) & (
    (fp_bins[:-1] + fp_bins[1:]) / 2 > 1e-30
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

plt.axvspan(1e-30, 1e-5, facecolor="C2", edgecolor="None", alpha=0.1, zorder=-2)
plt.axvspan(1e-5, 1, facecolor="C1", edgecolor="None", alpha=0.1, zorder=-2)
plt.axvspan(1, 20, facecolor="C3", edgecolor="None", alpha=0.1, zorder=-2)


plt.ylim(0.5, 1e6)

# 手动创建图例句柄
legend_handles = [
    mpatches.Patch(edgecolor="C0", facecolor="None", label="All Signals"),
    mpatches.Patch(edgecolor="C1", facecolor="None", label="Mis-associated Signals"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="Signals with Lightning"),
    plt.Line2D([0], [0], color="#CCCCCC", linestyle="--", label="Power Law Fit"),
    mpatches.Patch(facecolor="C2", edgecolor="None", alpha=0.1, label="Accept"),
    mpatches.Patch(
        facecolor="C1", edgecolor="None", alpha=0.1, label="Associated Only"
    ),
    mpatches.Patch(facecolor="C3", edgecolor="None", alpha=0.1, label="Reject"),
]
plt.legend(handles=legend_handles)
plt.xlabel("Expected Annual False Positive Under Poisson Assumption")
plt.ylabel("Number")

conn.close()

plt.xscale("log")
plt.yscale("log")
plt.xlim(max_fp, min_fp)
plt.savefig("hxmt-catalog/output/fp-distribution2.pdf", bbox_inches="tight")
