import sqlite3

import matplotlib.pyplot as plt
import numpy as np

plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
plt.figure(dpi=1200)

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute("SELECT fp_year FROM signal WHERE start < '2025-01-01'")
data = cursor.fetchall()
fp_years = [row[0] for row in data]
fp_years = np.array(fp_years)
min_fp = 1e-50
max_fp = np.max(fp_years)
bins = 100
fp_bins = np.logspace(np.log10(min_fp), np.log10(max_fp), bins + 1)
n, _, _ = plt.hist(
    fp_years,
    bins=fp_bins,
    histtype="step",
    label="All Signals",
)

cursor.execute(
    "SELECT fp_year FROM signal WHERE start < '2025-01-01' AND associated_lightning_count > 0"
)
data = cursor.fetchall()
fp_years_lightning = [row[0] for row in data]
fp_years_lightning = np.array(fp_years_lightning)
plt.hist(
    fp_years_lightning,
    bins=fp_bins,
    histtype="step",
    label="Signals with Lightning",
)

plt.stairs(
    n * 4e-4,
    fp_bins,
    fill=False,
    label="Mis-associated Signals",
)

plt.axvline(1e-3, color="C3", linewidth=0.5, label="Old Threshold")

plt.legend()
plt.xlabel("FP per year")
plt.ylabel("Number")

conn.close()

plt.xscale("log")
plt.yscale("log")
plt.xlim(max_fp, min_fp)
plt.savefig("hxmt-catalog/output/fp-distribution2.pdf", bbox_inches="tight")
