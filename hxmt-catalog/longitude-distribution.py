import matplotlib.pyplot as plt
from data import get_data

# 启用 LaTeX 渲染
plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": r"\usepackage{amsmath}",  # 如果需要数学公式支持
    }
)

cm = 1 / 2.54  # 将厘米转换为英寸
data = get_data()
plt.figure(figsize=(8 * cm, 6 * cm), dpi=1200, facecolor="none")
plt.hist(
    [
        signal.longitude if signal.longitude < 330 else signal.longitude - 360
        for signal in data
    ],
    bins=60,
    range=(-30, 330),
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="/",
)
plt.yscale("log")
plt.xlabel("Longitude")
plt.ylabel("Frequency")
plt.xticks(
    ticks=[0, 90, 180, 270],
    labels=["0°", "90°E", "180°", "90°W"],
)
plt.xlim(-30, 330)
plt.ylim(1e0, 1e3)
plt.text(20, 3e2, "Africa", ha="center", va="center")
plt.text(130, 4e2, "India, SE Asia", ha="center", va="center")
plt.text(210, 8e1, "Oceania", ha="center", va="center")
plt.text(280, 4e2, "Americas", ha="center", va="center")
plt.tight_layout()
plt.savefig("hxmt-catalog/output/longitude-distribution.pdf", transparent=True)
