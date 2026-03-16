#!/usr/bin/env python3
"""Generate HXMT HE saturation analysis presentation — 38 detailed slides."""

import sys, os
sys.path.insert(0, os.path.dirname(__file__))

from pptx_helpers import create_presentation
from slides_part1 import add_slides as add_part1
from slides_part2 import add_slides as add_part2
from slides_part3 import add_slides as add_part3
from slides_part4 import add_slides as add_part4

prs = create_presentation()

add_part1(prs)  # Slides 1-10:  Title, background, hardware, saturation modes 1-2
add_part2(prs)  # Slides 11-21: Deep sat, time recon, 4-pass, three paths, backward SEC
add_part3(prs)  # Slides 22-30: Pass 2-4, detection, R_true, reconstruction, cross-ref
add_part4(prs)  # Slides 31-38: Validation (3 GRBs), limitations, conclusions

out = os.path.join(os.path.dirname(__file__), "presentation.pptx")
prs.save(out)
print(f"Saved {out} ({len(prs.slides)} slides)")
