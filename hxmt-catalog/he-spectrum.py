import json
import sqlite3

import matplotlib.pyplot as plt
import numpy as np

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT value FROM statistics WHERE status = 'Finished' AND time < '2025-01-01'
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
        "text.latex.preamble": "\\usepackage{amsmath}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)
cm = 1 / 2.54
plt.figure(figsize=(8 * cm, 7 * cm), dpi=1200)
plt.stairs(
    data_new,
    np.arange(len(data_new) + 1),
    edgecolor="C0",
    facecolor="None",
    label="HE CsI Spectrum",
)
plt.axvline(
    38,
    color="C3",
    lw=0.5,
    label="Threshold",
)
plt.xlabel("Channel")
plt.ylabel("Frequency")
plt.legend()

plt.tight_layout()

plt.savefig(
    "hxmt-catalog/output/he-spectrum.pdf", bbox_inches="tight", transparent=True
)
