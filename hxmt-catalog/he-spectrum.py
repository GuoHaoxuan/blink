import json
import sqlite3

import matplotlib.pyplot as plt
import numpy as np
from common import one_column_width

conn = sqlite3.connect("statistics.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT value FROM statistics WHERE status = 'Finished'
    """
)
data = cursor.fetchall()
cursor.close()
conn.close()

data = np.array([json.loads(row[0]) for row in data], dtype=np.int64)
data = data.sum(axis=0)
data_new = np.zeros(256 + 20, dtype=np.int64)
for i in range(256):
    if i < 20:
        data_new[i + 256] = data[i]
    else:
        data_new[i] = data[i]

plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}\n\\usepackage{wasysym}\\usepackage{CJKutf8}",  # 如果需要数学公式支持
    }
)
cm = 1 / 2.54  # 将厘米转换为英寸
plt.figure(
    figsize=(one_column_width, (3 / 4) * one_column_width), dpi=1200, facecolor="none"
)
plt.stairs(
    data_new,
    np.arange(len(data_new) + 1),
    edgecolor="black",
    facecolor="None",
    hatch="/",
)
plt.axvline(
    38,
    color="black",
    linestyle="--",
    lw=0.5,
)
plt.xlabel("Channel")
plt.ylabel("Frequency")

plt.tight_layout()

plt.savefig(
    "hxmt-catalog/output/he-spectrum.pdf", bbox_inches="tight", transparent=True
)
