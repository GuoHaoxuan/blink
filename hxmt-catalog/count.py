import sqlite3

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT count_filtered_best, associated_lightning_count
    FROM signal
    WHERE start_full < '2025-01-01'
        AND (
            false_positive_per_year <= 1e-5
                OR false_positive_per_year <= 1 AND associated_lightning_count > 0)
    """
)
signals = cursor.fetchall()
counts = np.array([x[0] for x in signals])
counts_lightning = np.array([x[0] for x in signals if x[1] > 0])
cursor.close()
conn.close()

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
fig = plt.figure(figsize=(20 * cm, 5 * cm), dpi=1200)
gs = fig.add_gridspec(2, 1, height_ratios=[1, 3.5], hspace=0, wspace=0)

plt.subplot(gs[1])
maxval = np.max(counts)
minval = np.min(counts)
bins = np.logspace(np.log10(minval), np.log10(maxval), 38)
n_all, _, _ = plt.hist(counts, bins=bins, color="C0", histtype="step")
n_lightning, _, _ = plt.hist(counts_lightning, bins=bins, color="C2", histtype="step")
plt.xscale("log")
plt.yscale("log")
plt.xlim(minval, maxval)
plt.xlabel("Count")
plt.ylabel("Number")

twinx = plt.twinx()
twinx.set_zorder(-1)
twinx.plot(
    (bins[1:] + bins[:-1]) / 2,
    n_lightning / n_all,
    color="#CCCCCC",
    linestyle="--",
)
twinx.set_ylabel("Proportion")
twinx.set_ylim(0, 1)

plt.subplot(gs[0])
handles = [
    mpatches.Patch(edgecolor="C0", facecolor="None", label="TGFs"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="TGFs with Lightning"),
    plt.Line2D(
        [0], [0], color="#CCCCCC", linestyle="--", label="Proportion with Lightning"
    ),
]
plt.legend(handles=handles, ncols=len(handles), loc="center", frameon=False)
plt.axis("off")

plt.savefig("hxmt-catalog/output/count.pdf", bbox_inches="tight", transparent=True)
