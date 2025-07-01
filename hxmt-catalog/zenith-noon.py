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
    SELECT solar_zenith_angle_at_noon, associated_lightning_count
    FROM signal
    WHERE start < '2025-01-01'
        AND (fp_year < 1e-5 OR (fp_year < 1 AND associated_lightning_count > 0));
    """
)
signals = cursor.fetchall()
angles = np.array([x[0] for x in signals])
angles_lightning = np.array([x[0] for x in signals if x[1] > 0])
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
plt.hist(angles, bins=np.arange(0, 91, 1), color="C0", histtype="step")
plt.hist(angles_lightning, bins=np.arange(0, 91, 1), color="C2", histtype="step")
plt.axvspan(
    0, 23.5, facecolor="C1", edgecolor="None", alpha=0.1, label="Summer", zorder=-2
)
plt.axvspan(23.5, 66.5, facecolor="C2", edgecolor="None", alpha=0.1, zorder=-2)
plt.axvspan(66.5, 90, facecolor="C0", edgecolor="None", alpha=0.1, zorder=-2)
plt.xlim(0, 90)
plt.xlabel("Solar Zenith Angle at Noon (degrees)")
plt.ylabel("Number")
plt.yscale("log")

plt.subplot(gs[0])
handles = [
    mpatches.Patch(edgecolor="C0", facecolor="None", label="All Signals"),
    mpatches.Patch(edgecolor="C2", facecolor="None", label="Signals with Lightning"),
    mpatches.Patch(facecolor="C0", edgecolor="None", alpha=0.1, label="Winter"),
    mpatches.Patch(facecolor="C1", edgecolor="None", alpha=0.1, label="Summer"),
    mpatches.Patch(
        facecolor="C2", edgecolor="None", alpha=0.1, label="Spring or Autumn"
    ),
]
plt.legend(handles=handles, ncols=len(handles), loc="center", frameon=False)
plt.axis("off")

plt.savefig(
    "hxmt-catalog/output/zenith-noon.pdf", bbox_inches="tight", transparent=True
)
