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
    [signal.latitude for signal in data],
    bins=43,
    range=(-43, 43),
    histtype="stepfilled",
    edgecolor="black",
    facecolor="None",
    hatch="/",
)
plt.yscale("log")
plt.xlabel("Latitude")
plt.ylabel("Frequency")
plt.xticks(
    ticks=[-43, -30, -15, 0, 15, 30, 43],
    labels=["43°S", "30°S", "15°S", "0°", "15°N", "30°N", "43°N"],
)
plt.xlim(-43, 43)
plt.tight_layout()
plt.savefig("hxmt-catalog/output/latitude-distribution.pdf", transparent=True)
