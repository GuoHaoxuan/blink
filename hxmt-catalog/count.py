import sqlite3

import cartopy.crs as ccrs
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from dateutil.parser import parse
from PIL import Image

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT count_filtered_best, associated_lightning_count
    FROM signal
    WHERE start < '2025-01-01'
        AND (fp_year < 1e-5 OR (fp_year < 1 AND associated_lightning_count > 0));
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
bins = np.logspace(np.log10(minval), np.log10(maxval), 20)
plt.hist(counts, bins=bins, color="C0", histtype="step")
plt.hist(counts_lightning, bins=bins, color="C2", histtype="step")
plt.xscale("log")
plt.yscale("log")
plt.xlim(minval, maxval)
plt.xlabel("Count")
plt.ylabel("Number")

plt.subplot(gs[0])
handles = [
    mpatches.Patch(edgecolor="C0", facecolor="None", label="All Signals"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="Signals with Lightning"),
]
plt.legend(handles=handles, ncols=len(handles), loc="center", frameon=False)
plt.axis("off")

plt.savefig("hxmt-catalog/output/count.pdf", bbox_inches="tight", transparent=True)
