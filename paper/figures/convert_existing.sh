#!/usr/bin/env bash
#
# Convert existing PNG figures (paper/fig*.png) to PDF for LaTeX inclusion.
# Run from the paper/ directory.
#
# F3 = LIS vs greedy alternative (cascade failure)
# F5 = cross-box recovery worked example (GRB 221009A FIFO reset)
# F7 = cross-satellite (GRB 200415A: SPI-ACS + GBM 5ms composite)
#
# F1 (data path diagram) is now drawn inline with TikZ in main_en.tex.
# F8 (cross-box uniqueness) was removed when §5.4 was deleted.

set -euo pipefail
cd "$(dirname "$0")/.."

# F3: LIS vs greedy
sips -s format pdf fig_greedy_vs_lis.png --out figures/f3_lis_vs_greedy.pdf > /dev/null

# F5: cross-box recovery worked example
sips -s format pdf fig_crossbox.png --out figures/f5_crossbox_recovery.pdf > /dev/null

# F7: cross-satellite (GRB 200415A: SPI-ACS + GBM 5ms composite)
sips -s format pdf GRB200415A_hxmt_vs_spiacs_gbm.png --out figures/f7_cross_satellite.pdf > /dev/null

ls -la figures/*.pdf
