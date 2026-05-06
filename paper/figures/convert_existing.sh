#!/usr/bin/env bash
#
# Convert existing PNG figures (paper/fig*.png) to PDF for LaTeX inclusion.
# Run from the paper/ directory.
#
# F1 = data path diagram (3-box hardware overview)
# F8 = cross-box cross-correlation uniqueness (GRB 221009A T+249-268)

set -euo pipefail
cd "$(dirname "$0")/.."

# F1: data path
sips -s format pdf fig_datapath.png --out figures/f1_datapath.pdf > /dev/null

# F8: cross-box uniqueness
sips -s format pdf fig13_crossbox.png --out figures/f8_uniqueness.pdf > /dev/null

ls -la figures/*.pdf
