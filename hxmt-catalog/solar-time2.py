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
    SELECT apparent_solar_time, associated_lightning_count
    FROM signal
    WHERE start_full < '2025-01-01'
        AND (
            false_positive_per_year <= 1e-5
                OR false_positive_per_year <= 1 AND associated_lightning_count > 0)
    """
)
signals = cursor.fetchall()
angles = np.array([parse(x[0]).hour + parse(x[0]).minute / 60 for x in signals])
angles_lightning = np.array(
    [parse(x[0]).hour + parse(x[0]).minute / 60 for x in signals if x[1] > 0]
)
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
plt.hist(
    angles,
    bins=np.arange(0, 24.5, 0.5),
    color="C0",
    histtype="step",
    label="All Signals",
)
plt.hist(
    angles_lightning,
    bins=np.arange(0, 24.5, 0.5),
    color="C2",
    histtype="step",
    label="Lightning-Associated Signals",
)
plt.axvspan(
    0, 6, facecolor="C0", edgecolor="None", alpha=0.1, label="Nighttime", zorder=-2
)
plt.axvspan(
    6, 18, facecolor="C1", edgecolor="None", alpha=0.1, label="Daytime", zorder=-2
)
plt.axvspan(18, 24, facecolor="C0", edgecolor="None", alpha=0.1, zorder=-2)
plt.xlim(0, 24)
plt.xticks(np.arange(0, 25, 1))
plt.xlabel("Apparent Solar Time (hours)")
plt.ylabel("Number")

plt.subplot(gs[0])
handles = [
    mpatches.Patch(edgecolor="C0", facecolor="None", label="TGFs"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="TGFs with Lightning"),
    mpatches.Patch(facecolor="C0", edgecolor="None", alpha=0.1, label="Nighttime"),
    mpatches.Patch(facecolor="C1", edgecolor="None", alpha=0.1, label="Daytime"),
]
plt.legend(handles=handles, ncols=len(handles), loc="center", frameon=False)
plt.axis("off")
# plt.yscale("log")

plt.savefig(
    "hxmt-catalog/output/solar-time2.pdf", bbox_inches="tight", transparent=True
)
