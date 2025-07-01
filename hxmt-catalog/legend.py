import matplotlib.patches as mpatches
import matplotlib.pyplot as plt

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
plt.figure(figsize=(20 * cm, 0.1 * cm), dpi=1200)
legend_handles = [
    mpatches.Patch(facecolor="C0", edgecolor="None", label="All Signals"),
    mpatches.Patch(facecolor="C2", edgecolor="None", label="Lightning Associated"),
    # plt.Line2D([], [], color="black", linestyle="-", label="Coastline"),
    mpatches.Patch(facecolor="C3", edgecolor="None", alpha=0.1, label="SAA"),
]
plt.legend(
    handles=legend_handles, loc="center", ncol=len(legend_handles), frameon=False
)
plt.axis("off")
plt.savefig("hxmt-catalog/output/legend.pdf", bbox_inches="tight")
