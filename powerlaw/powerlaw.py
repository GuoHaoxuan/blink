import json
import sqlite3

import matplotlib.pyplot as plt
import numpy as np

plt.rcParams.update(
    {
        "text.usetex": True,  # 使用 LaTeX 渲染文字
        "font.family": "serif",  # 使用衬线字体（如 Times New Roman）
        "font.serif": ["Computer Modern"],  # 如果你用的是 LaTeX 默认字体
        "text.latex.preamble": "\\usepackage{amsmath}\\usepackage{CJKutf8}",  # 如果需要数学公式支持
        "lines.linewidth": 1,
    }
)


def main():
    axs = plt.figure(dpi=1200).subplot_mosaic(
        [
            ["zoom"],
            ["main"],
        ]
    )

    conn = sqlite3.connect("blink.db")
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT fp_year
        FROM signal
        WHERE start < "2025-01-01"
        """
    )
    data = cursor.fetchall()
    data = np.array([row[0] for row in data], dtype=np.float64)

    min_main = np.min(data)
    max_main = np.max(data)
    min_zoom = 1e-8
    max_zoom = np.max(data)
    log_bins_main = np.logspace(np.log10(min_main), np.log10(max_main), 200)
    log_bins_zoom = np.logspace(np.log10(min_zoom), np.log10(max_zoom), 200)

    axs["main"].hist(
        data,
        bins=log_bins_main,
        histtype="step",
    )
    axs["zoom"].hist(
        data,
        bins=log_bins_zoom,
        histtype="step",
    )

    cursor.execute(
        """
        SELECT fp_year, lightnings
        FROM signal
        WHERE start < "2025-01-01"
        """
    )
    data = cursor.fetchall()
    data = np.array(
        [
            row[0]
            for row in data
            if np.any(
                np.array(
                    list(
                        map(
                            lambda lightning: lightning["is_associated"],
                            json.loads(row[1]),
                        )
                    )
                )
            )
        ],
        dtype=np.float64,
    )
    axs["main"].hist(
        data,
        bins=log_bins_main,
        histtype="step",
    )
    axs["zoom"].hist(
        data,
        bins=log_bins_zoom,
        histtype="step",
    )

    axs["main"].set_xscale("log")
    axs["main"].set_yscale("log")
    axs["main"].set_xlim(min_main, max_main)
    axs["main"].invert_xaxis()
    axs["main"].set_xlabel("\\begin{CJK*}{UTF8}{gbsn}年误触发个数\\end{CJK*}")
    axs["main"].set_ylabel("\\begin{CJK*}{UTF8}{gbsn}频数\\end{CJK*}")
    axs["zoom"].set_xscale("log")
    axs["zoom"].set_yscale("log")
    axs["zoom"].set_xlim(min_zoom, max_zoom)
    axs["zoom"].invert_xaxis()
    axs["zoom"].set_ylabel("\\begin{CJK*}{UTF8}{gbsn}频数\\end{CJK*}")

    plt.savefig("powerlaw/powerlaw.pdf", bbox_inches="tight")


main()
