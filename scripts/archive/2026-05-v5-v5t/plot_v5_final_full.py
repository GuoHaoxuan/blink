#!/usr/bin/env python3
"""Read v5 aggregator npz and produce the final v5 triptych on full data.

Inputs:
  v5_agg_full.npz   (from v5_aggregator_worker.py, possibly merged)

Output:
  plots/v5_final_full.png
"""
from __future__ import annotations
import argparse
from pathlib import Path
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.cm as cm
from matplotlib.colors import LogNorm, LinearSegmentedColormap

# viridis whose low-density end fades to white, so count=1 bins sit close to the
# white (count=0) background instead of jumping to deep blue — mimics the soft
# sparse-region look of a scatter plot.
_v = cm.get_cmap("viridis")(np.linspace(0, 1, 256))
_n_fade = 40
_v[:_n_fade] = np.linspace([1, 1, 1, 1], _v[_n_fade], _n_fade)
WHITE_VIRIDIS = LinearSegmentedColormap.from_list("white_viridis", _v)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--input", default="n_below_study/v5_agg_full.npz")
    p.add_argument("--output", default="plots/v5_final_full.png")
    args = p.parse_args()

    z = np.load(args.input)
    H_loglog = z["H_loglog"].astype(float)
    H_before = z["H_before"].astype(float)
    H_after = z["H_after"].astype(float)
    XE = z["XE"]
    YE_LIN = z["YE_LIN"]
    YE_LOG = z["YE_LOG"]
    k = float(z["k_global"])
    n_total = int(z["n_total"])
    n_valid = int(z["n_valid"])
    n_below = int(z["n_below_yx"])
    s_det_daily = z["s_det_daily"]  # (n_days, 3, 6)
    dates = z["dates"]
    n_capped = int(z["n_capped"])
    n_blob = int(z["n_blob"])
    n_main = int(z["n_main"])

    # Median s_det over all days for the title
    s_det_med = np.nanmedian(s_det_daily, axis=0)
    s_min, s_max = float(np.nanmin(s_det_med)), float(np.nanmax(s_det_med))
    s_mean = float(np.nanmean(s_det_med))

    fig, axes = plt.subplots(1, 3, figsize=(24, 7.5))
    ax1, ax2, ax3 = axes

    LO, HI = 30.0, 10_000.0
    Y_LO, Y_HI = -500, 1500

    # Panel 1: log-log
    Hp = H_loglog.copy()
    Hp[Hp < 1] = np.nan
    ax1.pcolormesh(XE, YE_LOG, Hp.T, norm=LogNorm(vmin=1, vmax=np.nanmax(Hp)),
                    cmap=WHITE_VIRIDIS, rasterized=True)
    xx = np.logspace(np.log10(LO), np.log10(HI), 200)
    ax1.plot(xx, xx, "k--", lw=1.5, label="y = x")
    ax1.plot(xx, xx + s_min, "b-", lw=1.0, alpha=0.6, label=f"y = x + {s_min:.0f} (s_det min)")
    ax1.plot(xx, xx + s_mean, "b-", lw=2.0, label=f"y = x + {s_mean:.0f} (s_det mean)")
    ax1.plot(xx, xx + s_min * (1 + k * (43 - 20) ** 2), "m-", lw=1.0, alpha=0.7,
              label=f"y = x + s_min·(1+k·23²), |mlat|=43")
    ax1.plot(xx, xx + s_max * (1 + k * (43 - 20) ** 2), "m-", lw=1.0, alpha=0.7,
              label=f"y = x + s_max·(1+k·23²), |mlat|=43")
    ax1.set_xscale("log"); ax1.set_yscale("log")
    ax1.set_xlim(LO, HI); ax1.set_ylim(LO, HI)
    ax1.set_xlabel("Sci_1s observed (cnt/s)")
    ax1.set_ylabel("Sci_pred_base with v5 unwrap (cnt/s)")
    ax1.set_title(f"log-log Sci_pred vs Sci_obs — v5 unified, full {len(dates)}-day data",
                   fontsize=11)
    ax1.legend(loc="lower right", fontsize=9)
    ax1.grid(True, alpha=0.3, which="both")

    # Panel 2: residual BEFORE model
    Hb = H_before.copy()
    Hb[Hb < 1] = np.nan
    ax2.pcolormesh(XE, YE_LIN, Hb.T, norm=LogNorm(vmin=1, vmax=np.nanmax(Hb)),
                    cmap=WHITE_VIRIDIS, rasterized=True)
    ax2.axhline(s_mean, color="blue", lw=2.0, label=f"<s_det> = {s_mean:.0f}")
    ax2.axhline(0, color="k", ls=":", lw=0.7)
    ax2.set_xscale("log"); ax2.set_xlim(LO, HI); ax2.set_ylim(Y_LO, Y_HI)
    ax2.set_xlabel("Sci_1s observed (cnt/s)")
    ax2.set_ylabel("residual = base − Sci_obs (cnt/s, BEFORE model)")
    ax2.set_title("residual BEFORE model (v5)", fontsize=11)
    ax2.legend(loc="upper left", fontsize=10)
    ax2.grid(True, alpha=0.3, which="both")

    # Panel 3: residual AFTER model
    Ha = H_after.copy()
    Ha[Ha < 1] = np.nan
    ax3.pcolormesh(XE, YE_LIN, Ha.T, norm=LogNorm(vmin=1, vmax=np.nanmax(Ha)),
                    cmap=WHITE_VIRIDIS, rasterized=True)
    ax3.axhline(0, color="r", lw=2.0, label="zero (perfect model)")
    ax3.set_xscale("log"); ax3.set_xlim(LO, HI); ax3.set_ylim(Y_LO, Y_HI)
    ax3.set_xlabel("Sci_1s observed (cnt/s)")
    ax3.set_ylabel("residual_clean (cnt/s, AFTER unified model)")
    blob_ratio = n_blob / max(n_main, 1)
    ax3.set_title(f"residual AFTER unified model (v5)\nblob/main = {blob_ratio:.3f}",
                   fontsize=11)
    ax3.legend(loc="upper left", fontsize=10)
    ax3.grid(True, alpha=0.3, which="both")

    # dates are ISO strings "YYYY-MM-DD" (clean cache date column)
    n_yrs = (np.datetime64(dates[-1]) - np.datetime64(dates[0])).astype('timedelta64[D]').astype(int) / 365.25

    fig.suptitle(
        f"v5 unified model on FULL DATA — {len(dates):,} days ({n_yrs:.1f} yr), "
        f"{n_total:,} rows, {n_valid:,} valid\n"
        f"Per-day s_det refit;  k={k:.5f};  event-balance cap on {n_capped:,} rows;  "
        f"below y=x: {n_below:,} ({n_below/max(n_valid,1)*100:.3f}%)",
        fontsize=12, fontweight="bold", y=1.02,
    )
    plt.tight_layout()
    Path(args.output).parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(args.output, dpi=120, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved {args.output}")


if __name__ == "__main__":
    main()
