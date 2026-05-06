# SCPMA Paper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Submit "Saturation Recovery for Insight-HXMT/HE Bright Burst Observations from Level-1B Raw Data" to Science China Physics, Mechanics & Astronomy (SCPMA) as Article, ~13 pages double-column, 8 figures, 4 tables.

**Architecture:** Brand-new English manuscript written from scratch (existing `paper/main.tex` retained as figure-reference only, not translated). Numbers single-sourced from `paper/numbers.csv`, populated by running current `blink_cli` against three target events (GRB 260226A, GRB 200415A, GRB 221009A). Figures regenerated to PDF via Python scripts living in `paper/figures/`. SCPMA LaTeX template replaces RevTeX. Code released MIT on GitHub with Zenodo DOI tag at submission.

**Tech Stack:** LaTeX (SCPMA template), Rust (`blink_cli` for numbers), Python (`matplotlib` + `astropy` for figures), Git (commits, Zenodo release tag), Bash (orchestration).

**Timeline:** ~21 days. Phase A (prep) + Phase B (algorithm sections) run in parallel with user's engineering-data work during W -2/-1. Phase C onward starts at D +0 = engineering-data done. Day numbers below are relative to D +0.

**Source-of-truth files:**
- Spec: `docs/superpowers/specs/2026-05-06-scpma-submission-design.md`
- Algorithm doc: `crates/instruments/blink_hxmt_he/src/algorithms/saturation/DESIGN.md`
- Old reference (untranslated): `paper/main.tex`
- New manuscript: `paper/main_en.tex`

---

## File Structure

| File | Purpose | Phase |
|---|---|---|
| `paper/main_en.tex` | Primary English manuscript | A → F |
| `paper/refs_en.bib` | BibTeX for ~25-30 references | A |
| `paper/numbers.csv` | Single source of truth for all metrics; commit-hash stamped | A |
| `paper/scpma.cls` | SCPMA LaTeX template (downloaded) | A |
| `paper/figures/f{1..8}.pdf` | 8 final figures | A → D |
| `paper/figures/make_f2.py` ... | Figure assembly scripts where needed | A → D |
| `paper/cover_letter.tex` | Cover letter for submission | F |
| `paper/availability_statement.tex` | Code & Data Availability | F |
| `scripts/freeze_numbers.sh` | Reproduce all numbers via blink_cli | A |

---

## Phase A — Preparation (W -2/-1, parallel with engineering-data work)

### Task 1: Set up English-manuscript directory scaffold

**Files:**
- Create: `paper/main_en.tex`
- Create: `paper/refs_en.bib` (empty)
- Create: `paper/numbers.csv` (header only)
- Create: `paper/figures/.gitkeep`

- [ ] **Step 1: Create skeleton main_en.tex with placeholders for every section**

Write `paper/main_en.tex`:

```latex
\documentclass[twocolumn]{article}  % swap to scpma.cls in Task 2

\usepackage{amsmath,amssymb}
\usepackage{graphicx}
\usepackage{booktabs}
\usepackage{multirow}
\usepackage{xcolor}
\usepackage[colorlinks=true,linkcolor=blue,citecolor=blue]{hyperref}

\title{Saturation Recovery for Insight-HXMT/HE Bright Burst Observations from Level-1B Raw Data}

\author{Hao-Xuan Guo\thanks{kuohaoxuanus@outlook.com} \and HXMT Collaboration TBD}

\begin{document}
\maketitle

\begin{abstract}
TBD --- to be drafted in Task 16 after Phase E numbers are frozen.
\end{abstract}

\section{Introduction}\label{sec:intro}
% Task 6
TBD

\section{HE Data System and Saturation Mechanisms}\label{sec:instrument}
% Task 7
TBD

\section{Event-Level Time Reconstruction}\label{sec:method}
% Task 8
TBD

\section{FIFO Reset Detection and Light Curve Recovery}\label{sec:recovery}
% Task 10
TBD

\section{Validation}\label{sec:validation}
% Task 12
TBD

\section{Discussion: Limits and Failure Modes}\label{sec:discussion}
% Task 14
TBD

\section{Conclusion}\label{sec:conclusion}
% Task 15
TBD

\bibliographystyle{plain}
\bibliography{refs_en}
\end{document}
```

- [ ] **Step 2: Create empty refs_en.bib**

Write `paper/refs_en.bib` containing only a comment line:

```bibtex
% Bibliography for SCPMA paper. Populated in Task 4.
```

- [ ] **Step 3: Create numbers.csv header**

Write `paper/numbers.csv`:

```csv
metric,value,source_grb,source_box,commit_hash,note
```

- [ ] **Step 4: Create figures/ directory placeholder**

```bash
mkdir -p paper/figures
touch paper/figures/.gitkeep
```

- [ ] **Step 5: Verify minimal compile (with default article class as placeholder)**

```bash
cd paper && pdflatex -interaction=nonstopmode main_en.tex && ls main_en.pdf
```

Expected: `main_en.pdf` produced; warnings about empty sections OK; no errors.

- [ ] **Step 6: Commit**

```bash
git add paper/main_en.tex paper/refs_en.bib paper/numbers.csv paper/figures/.gitkeep
git commit -m "paper: scaffold English manuscript and supporting files"
```

---

### Task 2: Adopt SCPMA LaTeX template

**Files:**
- Create: `paper/scpma.cls` (download from SCPMA template page)
- Modify: `paper/main_en.tex` (swap documentclass)

- [ ] **Step 1: Download SCPMA template**

Visit `https://www.springer.com/journal/11433/submission-guidelines` (Science China Physics, Mechanics & Astronomy author guidelines). Download the LaTeX template package. Extract `scpma.cls` (or whatever the canonical name is — current SCPMA template may use a different name, e.g., `cjphys.cls`) into `paper/`.

If the template package contains additional support files (`.bst`, logo files), include all required ones in `paper/`.

- [ ] **Step 2: Modify main_en.tex documentclass and frontmatter macros**

Replace the documentclass line and frontmatter to match SCPMA's conventions. The template's example file shows the exact macros (`\Author`, `\AuthorMark`, `\Address`, `\Email`, `\Abstract`, `\Keywords`, etc.). Copy that pattern.

If the template is incompatible with `hyperref` or `graphicx` defaults, drop the conflicting `\usepackage` lines.

- [ ] **Step 3: Compile and verify SCPMA formatting**

```bash
cd paper && pdflatex -interaction=nonstopmode main_en.tex
```

Expected: `main_en.pdf` produced with SCPMA two-column layout, journal banner, correct title/author block.

If compile fails: read the `.log` file, fix the macro mismatch, re-compile. Do not skip this step — the template MUST compile cleanly before Task 6 starts.

- [ ] **Step 4: Commit**

```bash
git add paper/scpma.cls paper/main_en.tex
git commit -m "paper: adopt SCPMA template for English manuscript"
```

---

### Task 3: Freeze numbers — generate single source of truth

**Files:**
- Create: `scripts/freeze_numbers.sh`
- Modify: `paper/numbers.csv` (populate)

This task locks the metric values that propagate to abstract, validation tables, and discussion. After this task, do **not** modify the algorithm in any way until after submission.

- [ ] **Step 1: Identify the three target observations**

Confirmed targets:
- GRB 260226A — moderate saturation, headline 1B/1K residual case
- GRB 200415A — magnetar giant flare
- GRB 221009A — extreme saturation, coverage stress test

Trigger times (MET):
- 260226A: `2026-02-26T13:18:21` (UTC)
- 200415A: `2020-04-15T08:48:05.5` (UTC)
- 221009A: `2022-10-09T13:17:02` (UTC)

- [ ] **Step 2: Write freeze_numbers.sh orchestrating blink_cli runs**

Write `scripts/freeze_numbers.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
COMMIT=$(git rev-parse HEAD)
OUT=paper/numbers.csv

cargo build --release -p blink_cli

# Header
printf "metric,value,source_grb,source_box,commit_hash,note\n" > "$OUT"

# GRB 260226A — solve all three boxes, capture event counts and residual
for box in a b c; do
  COUNT=$(./target/release/blink_cli sat 2026-02-26T13 --box "$box" \
    solve 2026-02-26T13:18:21 --before 50 --after 100 \
    | tee /tmp/260226_box${box}.csv | grep -c '^EVT,')
  printf "260226a_box${box}_evt_count,%d,260226A,%s,%s,solve EVT count\n" \
    "$COUNT" "$box" "$COMMIT" >> "$OUT"
done

# GRB 260226A 1B vs 1K residual (Box A)
RESIDUAL=$(./target/release/blink_cli sat 2026-02-26T13 --box a \
  compare 2026-02-26T13:18:21 --before 50 --after 100 \
  | grep 'residual_events' | awk -F= '{print $2}')
printf "260226a_boxa_residual,%d,260226A,a,%s,1B vs 1K event count diff\n" \
  "$RESIDUAL" "$COMMIT" >> "$OUT"

# GRB 221009A coverage statistics for all three boxes
for box in a b c; do
  ./target/release/blink_cli sat 2022-10-09T13 --box "$box" \
    solve 2022-10-09T13:17:02 --before 50 --after 750 \
    > /tmp/221009a_box${box}.csv
  TOTAL=$(grep -c '^' /tmp/221009a_box${box}.csv)
  WITH_MET=$(awk -F, '$3 != "NaN"' /tmp/221009a_box${box}.csv | wc -l)
  printf "221009a_box${box}_total,%d,221009A,%s,%s,CRC-passed events\n" \
    "$TOTAL" "$box" "$COMMIT" >> "$OUT"
  printf "221009a_box${box}_covered,%d,221009A,%s,%s,events with assigned MET\n" \
    "$WITH_MET" "$box" "$COMMIT" >> "$OUT"
done

# GRB 221009A reconstruction reference availability
./target/release/blink_cli sat 2022-10-09T13 --box a \
  reconstruct 2022-10-09T13:17:02 --before 50 --after 750 \
  --report-coverage > /tmp/221009a_reconstruct_report.txt
# parse report for "two_refs", "at_least_one", "all_saturated" percentages
# (specific awk depends on the report format produced by current blink_cli)

echo "numbers.csv frozen at commit $COMMIT"
chmod +x "$0"
```

This script is approximate — when running it, adjust grep/awk patterns to match the exact `blink_cli` output format on the current `saturation` branch. Use whatever `--report-coverage` flag exists; if not, add it as a follow-up issue.

- [ ] **Step 3: Run freeze_numbers.sh**

```bash
chmod +x scripts/freeze_numbers.sh
./scripts/freeze_numbers.sh 2>&1 | tee scripts/freeze_numbers_run.log
```

Expected runtime: 5-30 minutes depending on data volume. The script prints `numbers.csv frozen at commit <hash>` on success.

- [ ] **Step 4: Verify numbers.csv populated**

```bash
wc -l paper/numbers.csv  # should be header + ~15-20 rows
head paper/numbers.csv
```

Sanity check by eye: 260226A residual should be O(1-10), 221009A coverage should be ~95-97%, 1B event counts in millions.

- [ ] **Step 5: Commit**

```bash
git add scripts/freeze_numbers.sh paper/numbers.csv
git commit -m "paper: freeze validation numbers for all target events"
```

After this commit, `numbers.csv` is the law. Discrepancies between paper text and `numbers.csv` are paper bugs, not number bugs.

---

### Task 4: Bibliography mining — populate refs_en.bib

**Files:**
- Modify: `paper/refs_en.bib`

- [ ] **Step 1: Compile the reference list from the spec**

Open `docs/superpowers/specs/2026-05-06-scpma-submission-design.md` §5.4 and copy the 8 categories with named references.

- [ ] **Step 2: Mine each reference from NASA ADS**

For each reference, search NASA ADS (`https://ui.adsabs.harvard.edu/`) and export BibTeX. Combine into `paper/refs_en.bib`.

Required entries (minimum):

```bibtex
@article{zhang2020,
  author={Zhang, S.-N. and others},
  title={Overview of the Insight-HXMT mission},
  journal={Sci. China Phys. Mech. Astron.},
  volume={63}, number={249502}, year={2020}
}
@article{liu2020,
  author={Liu, C. Z. and others},
  title={The High Energy X-ray telescope (HE) onboard the Insight-HXMT and its first results},
  journal={Sci. China Phys. Mech. Astron.},
  volume={63}, number={249503}, year={2020}
}
@article{burns2023,
  author={Burns, E. and others},
  title={GRB 221009A: The BOAT},
  journal={ApJL}, volume={946}, number={L31}, year={2023}
}
@article{frederiks2023,
  author={Frederiks, D. and others},
  title={Konus-Wind observations of GRB 221009A},
  journal={ApJL}, volume={949}, number={L7}, year={2023}
}
@article{an2023,
  author={An, Z.-H. and others},
  title={Insight-HXMT and GECAM-C observations of the brightest-of-all-time GRB 221009A},
  journal={Sci. China Phys. Mech. Astron.}, year={2023}
}
@article{roberts2021,
  author={Roberts, O. J. and others},
  title={Rapid spectral variability of a giant flare from a magnetar in NGC~253},
  journal={Nature}, volume={589}, year={2021}
}
@article{svinkin2021,
  author={Svinkin, D. and others},
  title={A bright {\\gamma}-ray flare interpreted as a giant magnetar flare in NGC~253},
  journal={Nature}, volume={589}, year={2021}
}
@article{meegan2009,
  author={Meegan, C. and others},
  title={The Fermi Gamma-ray Burst Monitor},
  journal={ApJ}, volume={702}, year={2009}
}
@article{li2022gecam,
  author={Li, X.-Q. and others},
  title={The GECAM and its capability of observing FRBs and GRBs},
  journal={Radiat. Detect. Technol. Methods}, year={2022}
}
@article{vedrenne2003,
  author={Vedrenne, G. and others},
  title={SPI: The spectrometer aboard INTEGRAL},
  journal={A\\&A}, volume={411}, year={2003}
}
@article{savchenko2017,
  author={Savchenko, V. and others},
  title={INTEGRAL IBIS, SPI, and JEM-X observations of LVT151012},
  journal={A\\&A}, volume={603}, year={2017}
}
@article{schensted1961,
  author={Schensted, C.},
  title={Longest increasing and decreasing subsequences},
  journal={Canadian J. Math.}, volume={13}, year={1961}
}
@article{aldous1999,
  author={Aldous, D. and Diaconis, P.},
  title={Longest increasing subsequences: from patience sorting to the Baik--Deift--Johansson theorem},
  journal={Bull. AMS}, volume={36}, year={1999}
}
@article{koopman2004,
  author={Koopman, P. and Chakravarty, T.},
  title={Cyclic redundancy code (CRC) polynomial selection for embedded networks},
  booktitle={DSN}, year={2004}
}
```

Add 5-10 more references discovered while writing (at least: GRB 221009A LHAASO TeV companion paper, GECAM-C An+ 2022, HXMT mission Chen+ 2020, INTEGRAL ScW archive technote).

- [ ] **Step 3: Verify bibliography compiles**

Add one citation in main_en.tex (e.g., `\cite{zhang2020}` in Introduction placeholder). Run:

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
```

Expected: PDF contains a References section with `[1] Zhang, S.-N. et al. Overview...` correctly rendered.

- [ ] **Step 4: Commit**

```bash
git add paper/refs_en.bib paper/main_en.tex
git commit -m "paper: bibliography seed (~15 entries, more added during writing)"
```

---

### Task 5: Prepare reusable figures (F1, F8 — minimal edits)

**Files:**
- Create: `paper/figures/f1_datapath.pdf` (from existing `paper/fig_datapath.png`)
- Create: `paper/figures/f8_uniqueness.pdf` (from existing `paper/fig13_crossbox.png`)
- Create: `paper/figures/convert_existing.sh`

- [ ] **Step 1: Confirm existing figures are publication-ready**

```bash
file paper/fig_datapath.png paper/fig13_crossbox.png
```

If they are PNGs, convert to PDF (LaTeX prefers PDF for vector quality, falls back to PNG if necessary). If they are already vector, copy directly.

- [ ] **Step 2: Write conversion helper**

Write `paper/figures/convert_existing.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# F1 = data path
sips -s format pdf fig_datapath.png --out figures/f1_datapath.pdf

# F8 = cross-box uniqueness
sips -s format pdf fig13_crossbox.png --out figures/f8_uniqueness.pdf

ls -la figures/
```

(`sips` is macOS native — adjust to `convert` from ImageMagick on Linux.)

- [ ] **Step 3: Run conversion and verify**

```bash
chmod +x paper/figures/convert_existing.sh
./paper/figures/convert_existing.sh
file paper/figures/f1_datapath.pdf paper/figures/f8_uniqueness.pdf
```

Expected: both PDFs exist and report as `PDF document, version 1.x`.

- [ ] **Step 4: Commit**

```bash
git add paper/figures/f1_datapath.pdf paper/figures/f8_uniqueness.pdf paper/figures/convert_existing.sh
git commit -m "paper: stage F1 and F8 from existing figures"
```

---

## Phase B — Algorithm sections (W -1)

### Task 6: Section 1 (Introduction) — ~1.5 pages

**Files:**
- Modify: `paper/main_en.tex` §intro

- [ ] **Step 1: Write 4-paragraph Introduction**

Replace the placeholder under `\section{Introduction}` in `main_en.tex` with content following the spec §1.2 hook + 4-contribution-list structure. Each paragraph addresses one of:

1. HE's unique large effective area in 20-250 keV → bright source observations → trade-off with FIFO saturation
2. FIFO/MCU readout limit, 1K pipeline conservative filtering, gap left for 1B reconstruction
3. Two core challenges: ptime 1.05 s wrap + 4-bit CRC ghost; brief preview of LIS-based solution
4. The 4-bullet contribution list (verbatim from spec §1.3, edited to English)

Cite `zhang2020`, `liu2020`, `burns2023`, `an2023` where relevant.

The opening sentence must be the spec's Hook paragraph translated to English (NOT translated from existing Chinese paper/main.tex):

> The Hard X-ray Modulation Telescope (Insight-HXMT) High-Energy detector (HE) provides ~5100 cm² of effective area in the 20-250 keV band, making it uniquely sensitive to bright transients but also uniquely susceptible to electronics-level saturation: when source rates exceed the MCU readout capacity, FIFO buffers overflow and FPGA-side writes are blocked, silently dropping events.

- [ ] **Step 2: Compile and check page count**

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
pdfinfo main_en.pdf | grep Pages
```

Expected: at this stage, page count is small (~2 pages). Introduction occupies ~1.5 pages of double column.

- [ ] **Step 3: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Introduction section"
```

---

### Task 7: Section 2 (HE Data System) — ~1.5 pages

**Files:**
- Modify: `paper/main_en.tex` §instrument
- Insert: `\includegraphics{figures/f1_datapath.pdf}`

- [ ] **Step 1: Draft 4 subsections**

Following spec §3 outline, write subsections:

- 2.1 Hardware data path and FIFO architecture (cite `liu2020`; reference Figure 1)
- 2.2 Event packet format, CRC, ptime
- 2.3 Saturation modes: FIFO reset vs silent drop
- 2.4 1B vs 1K data products

Source content from `crates/instruments/blink_hxmt_he/src/algorithms/saturation/DESIGN.md` §"硬件" and §"事例与时间戳" — translate to English, keep technical specifics (FIFO A capacity ~4096 bytes, 109 events/CCSDS packet, MCU `HandlePhysicalLVDS()` ~7 ms, 19-bit ptime, 524288 mod, etc.).

- [ ] **Step 2: Insert Figure 1 with caption**

```latex
\begin{figure}[!tbp]
  \centering
  \includegraphics[width=\columnwidth]{figures/f1_datapath.pdf}
  \caption{HE data path. Each of three independent detector boxes (A, B, C) routes events through ASIC $\to$ FPGA $\to$ FIFO\,A $\to$ MCU $\to$ FIFO\,B $\to$ 1553B downlink. FIFO\,A overflow blocks FPGA writes (silent drops) or triggers a full-FIFO reset; both are saturation modes addressed in Section~\ref{sec:recovery}.}
  \label{fig:datapath}
\end{figure}
```

- [ ] **Step 3: Compile, verify Figure 1 placement**

```bash
cd paper && pdflatex main_en
```

Expected: Figure 1 renders, caption visible. If figure floats to wrong page, manually adjust placement or use `[H]` (with `\usepackage{float}`).

- [ ] **Step 4: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Section 2 (HE data system) and place Figure 1"
```

---

### Task 8: Section 3 (Time Reconstruction) — ~2.5 pages

**Files:**
- Modify: `paper/main_en.tex` §method
- Insert: F2, F3, F4 references (figures will be added in Task 9 and 13)

This section describes the algorithm but is framed as **prerequisite** for the saturation recovery in Section 4. Open with that framing.

- [ ] **Step 1: Write opening paragraph framing time reconstruction as prerequisite**

```latex
\section{Event-Level Time Reconstruction}\label{sec:method}

Time reconstruction in 1B raw data is the prerequisite for the saturation recovery presented in Section~\ref{sec:recovery}: without per-event MET assignment, lost events cannot be re-binned into a corrected light curve. The standard 1K pipeline produces accurate event times in non-saturated regions, but its conservative filtering removes precisely the events we wish to recover. We therefore reconstruct event times directly from 1B telemetry, exploiting hardware FIFO ordering as the central invariant.
```

- [ ] **Step 2: Write subsection 3.1 (Problem formulation)**

Source: spec §1.3 + DESIGN.md §"核心公式" + §"n 的唯一性条件". Center the equation:

```latex
t_{\rm MET} = t_{\rm SEC} + (p - p_{\rm SEC} + n \cdot P_{\rm MOD})\Delta t + t_{\rm corr}
```

Make explicit that $n$ is the only unknown. Brief mention of 4-bit CRC ghost mechanism.

- [ ] **Step 3: Write subsection 3.2 (SEC validation, two-stage)**

Source: DESIGN.md §"Step 2: SEC 验证". Describe phase clustering (mod-arithmetic phase $(p - s\cdot500000) \bmod P_{\rm MOD}$ clusters within ~113 ticks for real SECs) and stime-LIS purification.

Reference Figure 2 (placement in Task 13).

- [ ] **Step 4: Write subsection 3.3 (Δstime=1 direct LIS, with dead-zone safeguard sentence)**

Source: DESIGN.md §"Δstime=1 的 SEC 对：直接 LIS". Include:
- Definition of $e_i = (p_i - p_{\rm SEC_1}) \bmod P_{\rm MOD}$
- Valid range $[0, a]$ where $a = (p_{\rm SEC_2} - p_{\rm SEC_1}) \bmod P_{\rm MOD} \approx 500{,}000$
- Direct-LIS extracts the longest strictly-increasing subsequence; ghosts are excluded automatically
- One-sentence dead-zone safeguard:

> The complementary range $[a+1, P_{\rm MOD}-1]$ ($\sim$48.6 ms gap between the 1-second window and the next ptime wrap) corresponds to no real time within the SEC pair; events with $e_i$ in this region are CRC-collision artifacts and are correctly excluded by the validity check. This is a structural feature of the encoding, not a method limitation.

Subsubsection: Why LIS rather than greedy (cascade-failure illustration). Reference Figure 3.

Final paragraph: Bidirectional MET averaging ($t_{\rm fwd}$, $t_{\rm bwd}$, average).

- [ ] **Step 5: Write subsection 3.4 (Δstime>1 grouped LIS)**

Source: DESIGN.md §"Δstime>1 的 SEC 对". Cover:
- $k$ candidates $e_i^{(j)} = e_i + j \cdot P_{\rm MOD}$
- Modified patience sorting with **descending** intra-event candidate processing (proof sketch: descending guarantees parent chain points to prior events, no same-event cycle)
- UTC-tail constraint pruning: $e_i^{(j)} \le (t_{\rm utc} + 1 - t_{\rm SEC_1})/\Delta t$
- Wrap uniqueness: dual evidence (skip-wrap-0 LIS strictly shorter; cross-box cross-correlation residual minimum at wrap 0)

Reference Figure 4 and Table 1 (UTC pruning, placed in Task 12).

- [ ] **Step 6: Compile and check page count**

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
pdfinfo main_en.pdf | grep Pages
```

Expected: ~5-6 pages cumulative (sections 1-3).

- [ ] **Step 7: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Section 3 (time reconstruction algorithm)"
```

---

## Phase C — Headline content (D 1-3)

### Task 9: Make Figure 6 (engineering-data validation, headline)

**Files:**
- Create: `paper/figures/make_f6.py`
- Create: `paper/figures/f6_engineering_validation.pdf`

This figure depends on the user's engineering-data work being complete and converged.

- [ ] **Step 1: Confirm engineering-data results are stable**

Verify that `scripts/scan_eng_k.py` outputs $k$ values stable across hours (variation <15%) and that `plot_eng_vs_sci_compare_260226.py` produces clean comparisons. If not, decision-gate to R1 mitigation in spec §7 (downgrade F6 to single-GRB-only or supplementary).

- [ ] **Step 2: Author make_f6.py**

Write `paper/figures/make_f6.py` building on `scripts/plot_eng_vs_sci_compare_260226.py`. Produce a 3-row × 1-col composite (Box A, B, C) for GRB 260226A, each row showing:
- Engineering-data counter rate (with multiplicative dead-time correction applied) overlaid on 1B reconstructed event rate
- Ratio panel (engineering ÷ 1B) below

Use matplotlib, save as `figures/f6_engineering_validation.pdf`.

- [ ] **Step 3: Generate figure**

```bash
cd paper && uv run python figures/make_f6.py
file figures/f6_engineering_validation.pdf
```

- [ ] **Step 4: Commit**

```bash
git add paper/figures/make_f6.py paper/figures/f6_engineering_validation.pdf
git commit -m "paper: F6 engineering-data validation figure"
```

---

### Task 10: Section 4 (Saturation Recovery) — ~2 pages, headline

**Files:**
- Modify: `paper/main_en.tex` §recovery

- [ ] **Step 1: Write opening paragraph framing this as the headline section**

```latex
\section{FIFO Reset Detection and Light Curve Recovery}\label{sec:recovery}

With per-event times reconstructed in Section~\ref{sec:method}, we now address the headline problem: recovering events lost to FIFO resets. The recovery proceeds in three steps---detect packet-level gaps that mark FIFO resets, flag time intervals where reference data are themselves unreliable, and reconstruct each gap using calibrated shape functions drawn from independent detector boxes.
```

- [ ] **Step 2: Write subsection 4.1 (Adaptive gap threshold)**

Source: DESIGN.md §"FIFO 复位 gap 检测". Describe `g > T_base × F_gap` with `F_gap=100`, MCU read floor $R_{\rm MCU}=15{,}000$ evt/s, and 5-packet window for deep-saturation backoff.

- [ ] **Step 3: Write subsection 4.2 (Unreliable interval flagging)**

Three flags from DESIGN.md §"不可信区间检测":
1. FIFO reset gaps themselves
2. Wide congested packets (span >3× neighbor median)
3. In-packet anomalies (Poisson $\log_{10}p < -10$ for any inter-event gap, $\lambda$ from neighbor rate)

- [ ] **Step 4: Write subsection 4.3 (Cross-box shape-function reconstruction)**

Source: DESIGN.md §"FIFO 复位 gap 重建算法". Cover:
- 1 ms binning
- Calibration coefficient $k = R_{\rm target}/R_{\rm ref}$ over $\pm 0.5$ s windows around the gap
- Shape function = $R_{\rm ref}\cdot k$, multi-reference averaging, linear interpolation for empty bins
- $N_{\rm lost} = \lfloor \sum_i S_i \rfloor$ (form and norm from same source = self-consistent)
- Event allocation: equally spaced within each bin

- [ ] **Step 5: Write subsection 4.4 (Three-box co-saturation degenerate case)**

Brief — when all three boxes are saturated, fall back to linear interpolation of pre/post-reset packet rates, uniform allocation. Cite the ~8% prevalence (number from `numbers.csv`).

- [ ] **Step 6: Compile and check Section 4 occupies ~2 pages**

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
```

- [ ] **Step 7: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Section 4 (saturation recovery, headline section)"
```

---

## Phase D — Validation (D 4-5)

### Task 11: Make Figure 7 (cross-satellite three-panel composite)

**Files:**
- Create: `paper/figures/make_f7.py`
- Create: `paper/figures/f7_cross_satellite.pdf`

- [ ] **Step 1: Author make_f7.py orchestrating three sub-plots**

Write `paper/figures/make_f7.py` that calls (or inlines logic from) `scripts/plot_hxmt_vs_gbm.py`, `scripts/plot_hxmt_vs_spiacs.py`, `scripts/plot_hxmt_vs_gecam.py` and stitches their outputs into a single 3-panel figure (rows: GBM/260226A, SPI-ACS/200415A, GECAM-C/221009A), each panel:
- HXMT/HE 1B (observed) — blue
- HXMT/HE 1B (observed + recovered) — orange
- Reference instrument scaled — green
- Ratio sub-panel below

Use the time/binning settings from DESIGN.md cross-validation tables (260226A: 0.5 s bin; 200415A: 50 ms bin; 221009A: 1 s bin).

- [ ] **Step 2: Generate F7**

```bash
cd paper && uv run python figures/make_f7.py
file figures/f7_cross_satellite.pdf
```

- [ ] **Step 3: Commit**

```bash
git add paper/figures/make_f7.py paper/figures/f7_cross_satellite.pdf
git commit -m "paper: F7 cross-satellite three-panel composite"
```

---

### Task 12: Section 5 (Validation) — ~3 pages

**Files:**
- Modify: `paper/main_en.tex` §validation
- Insert: F6, F7, F8 placements; Tables T1, T2, T3, T4

- [ ] **Step 1: Write subsection 5.1 (Internal consistency, 1B vs 1K, 260226A)**

Source: DESIGN.md §"GRB 260226A Box A：3 events 残差". State residual = N events out of N total (numbers from `numbers.csv`); enumerate the 3 (or current count) residual cases with their causes (CRC false-reject ×2, dead-zone edge ×1).

- [ ] **Step 2: Write subsection 5.2 (HE engineering-data cross-check) — HEADLINE**

Place Figure 6 prominently. Describe:
- Engineering-data path independence from FIFO-A (counter-based, not queue-based)
- Multiplicative dead-time correction: $\hat C_{\rm eng} = C_{\rm PHO}\cdot(L-D)/L - C_{\rm CsI} - C_{\rm Large}$
- Calibration coefficient $k$ derived from quiet intervals
- Result: agreement at the few-percent level across all three boxes for GRB 260226A; same for 200415A and 221009A within available coverage

- [ ] **Step 3: Write subsection 5.3 (Cross-satellite validation)**

Place Figure 7 and Table 4 (cross-satellite summary).

5.3.1 Time system handling — single paragraph. Refer to DESIGN.md §"标准 Workflow / Step 1-3":
- HXMT/HE: MET in SI seconds since 2012-01-01 UTC, leap-second corrected via astropy.time
- GBM: MET since 2001-01-01 UTC
- SPI-ACS: TIMESYS=TT, MJDREF=51544.0, TT-UTC=69.184 s (2020)
- GECAM-C: TIMESYS=TT, MJDREF=59215.00080074074 (TT-UTC offset baked in)
- Light-travel correction: $t_{\rm geo} = t_{\rm sat} + (\vec r_{\rm sat}\cdot\hat n_{\rm src})/c$, **sign convention from projection formula** (do not eyeball)

5.3.2 GRB 260226A × Fermi/GBM (NaI n0,n3 ch72-124 + BGO b0 ch0-19, ~200-900 keV; T0 offset 5.958 s)

5.3.3 GRB 200415A × INTEGRAL/SPI-ACS (magnetar giant flare; INTEGRAL high-elliptical orbit, projection -406.6 ms; HXMT LEO ~0)

5.3.4 GRB 221009A × GECAM-C (low-gain GAIN_TYPE=1, T+397-676 s tail-only coverage due to Earth occultation)

- [ ] **Step 4: Write subsection 5.4 (Cross-box internal cross-correlation, 221009A)**

Source: DESIGN.md §"跨 Box 互相关验证". Wrap-0 residual 10,862 vs wrap-+1 residual 30,572 over T+249-268 s. Place Figure 8.

- [ ] **Step 5: Write Tables T2, T3, T4**

T2 (GRB 221009A reconstruction stats — 3 boxes):

```latex
\begin{table}[!tbp]
\centering
\caption{GRB~221009A 1B time-reconstruction statistics for the three HE detector boxes. Numbers from \texttt{numbers.csv} at commit \texttt{<HASH>}.}
\label{tab:221009a}
\begin{tabular}{lccc}
\toprule
 & Box A & Box B & Box C \\
\midrule
CRC-passed events & XXX & XXX & XXX \\
Valid SEC anchors & XXX & XXX & XXX \\
Resolved events  & XXX & XXX & XXX \\
Coverage (\%)    & XX.X & XX.X & XX.X \\
\bottomrule
\end{tabular}
\end{table}
```

Replace `XXX` with values from `paper/numbers.csv`.

T3 (cross-box reference availability):

```latex
\begin{table}[!tbp]
\caption{Fraction of FIFO-reset gaps with cross-box reference availability for GRB~221009A.}
\label{tab:coverage}
% similar structure
\end{table}
```

T4 (cross-satellite summary):

```latex
\begin{table}[!tbp]
\caption{Cross-satellite light-curve comparison summary.}
\label{tab:crosssat}
\begin{tabular}{lcccc}
\toprule
Event & Reference & Bin & Ratio & Light-travel correction \\
\midrule
GRB 260226A & Fermi/GBM   & 0.5\,s & X.XX$\pm$X.XX & \\
GRB 200415A & INTEGRAL/SPI-ACS & 50\,ms & 1.09$\pm$0.14 & SPI-ACS $-406.6$\,ms; HXMT $\sim 0$ \\
GRB 221009A & GECAM-C & 1\,s & X.XX$\pm$X.XX & both LEO, $<47$\,ms \\
\bottomrule
\end{tabular}
\end{table}
```

- [ ] **Step 6: Compile and verify Section 5 occupies ~3 pages, all figures and tables render**

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
pdfinfo main_en.pdf | grep Pages
```

- [ ] **Step 7: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Section 5 (validation) with all four sub-validations"
```

---

### Task 13: Make Figures F2, F3, F4, F5 (composites)

**Files:**
- Create: `paper/figures/make_f2.py`, `make_f3.py`, `make_f4.py`, `make_f5.py`
- Create: `paper/figures/f{2..5}.pdf`

- [ ] **Step 1: F2 = pipeline overview (top) + phase clustering (bottom)**

Author `make_f2.py` that:
1. Top sub-panel: a hand-drawn-style flow `CRC parse → SEC validate → SEC-pair LIS → events`. Use matplotlib boxes/arrows or ingest `paper/fig_pipeline.png` as bitmap top half.
2. Bottom sub-panel: phase clustering scatter (using `paper/phase_data.js` data — converted from existing `fig_phase_cluster.png` source).

Save `figures/f2_pipeline_phase.pdf`.

- [ ] **Step 2: F3 = greedy cascade (left) + LIS clean (right)**

Two-panel side-by-side:
- Left: `paper/fig07_cascade.png` content — greedy fails when ghost ef is too high
- Right: `paper/fig01_crc_ghost.png` content — LIS auto-excludes ghost

Save `figures/f3_lis_vs_greedy.pdf`.

- [ ] **Step 3: F4 = grouped LIS three-panel (candidates / descending / UTC pruning)**

Three sub-panels horizontal:
- (a) Candidate grid: each event has $k$ candidates
- (b) Descending intra-event processing illustration
- (c) UTC-tail bound vs ds=19 wrap pruning effect

Source from `paper/fig08_candidates.png`, `paper/fig09_descending.png`, `paper/fig_utc_constraint.png`.

Save `figures/f4_grouped_lis.pdf`.

- [ ] **Step 4: F5 = cross-box light curve reconstruction example**

Multi-box visualization of one FIFO reset gap from GRB 221009A, showing observed/filled events and shape function. Source from `commit ac14623` script output (which produced `paper/fig...` — locate or regenerate).

Save `figures/f5_crossbox_recovery.pdf`.

- [ ] **Step 5: Insert F2-F5 into main_en.tex at the right places**

```latex
% Section 3.2 area
\begin{figure}[!tbp]
  \centering
  \includegraphics[width=\columnwidth]{figures/f2_pipeline_phase.pdf}
  \caption{(Top) Reconstruction pipeline overview. (Bottom) SEC phase clustering: real SECs cluster within ~113 ticks of the modular phase, ghost SECs scatter uniformly.}
  \label{fig:pipeline}
\end{figure}

% Section 3.3 area
\begin{figure}[!tbp]
  \centering
  \includegraphics[width=\columnwidth]{figures/f3_lis_vs_greedy.pdf}
  \caption{Failure mode of greedy event ordering vs LIS. Left: a single ghost event causes cascading rejection of subsequent real events. Right: LIS automatically excludes the ghost.}
  \label{fig:lis_vs_greedy}
\end{figure}

% Section 3.4 area
\begin{figure*}[!tbp]
  \centering
  \includegraphics[width=0.85\textwidth]{figures/f4_grouped_lis.pdf}
  \caption{Grouped LIS for $\Delta s_{\rm time}>1$. (a) Each event has $k$ candidate elapsed values. (b) Within-event candidates processed in descending order to prevent self-cycling. (c) UTC-tail upper bound prunes implausible wrap assignments.}
  \label{fig:grouped_lis}
\end{figure*}

% Section 4 area
\begin{figure}[!tbp]
  \centering
  \includegraphics[width=\columnwidth]{figures/f5_crossbox_recovery.pdf}
  \caption{Light curve reconstruction across a FIFO reset for GRB~221009A. Reference boxes carry the burst shape; calibration matches their counts to the saturated box.}
  \label{fig:crossbox_recovery}
\end{figure}
```

- [ ] **Step 6: Compile, view PDF, verify all figures render**

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
open main_en.pdf  # macOS preview
```

- [ ] **Step 7: Commit**

```bash
git add paper/figures/make_f2.py paper/figures/make_f3.py paper/figures/make_f4.py paper/figures/make_f5.py paper/figures/f2_pipeline_phase.pdf paper/figures/f3_lis_vs_greedy.pdf paper/figures/f4_grouped_lis.pdf paper/figures/f5_crossbox_recovery.pdf paper/main_en.tex
git commit -m "paper: F2 F3 F4 F5 composites and placement"
```

---

## Phase E — Discussion + finalization (D 6-7)

### Task 14: Section 6 (Discussion: Limits and Failure Modes) — ~1.5 pages

**Files:**
- Modify: `paper/main_en.tex` §discussion

- [ ] **Step 1: Write subsection 6.1 (Too bright)**

Quantify: ~8% of FIFO-reset gaps in GRB 221009A have all three boxes saturated (number from `numbers.csv`); SEC gaps >10 s reduce coverage; SEE-induced data corruption in CCSDS payloads (0x5A misalignment 0.39%→1.13% during 221009A). Frame as "we measured the limits", not "we failed".

- [ ] **Step 2: Write subsection 6.2 (Too short)**

Source: DESIGN.md §"FRB 200428" + ASIM/MXGS LED case. Cite ratio ~0.10 at 1 ms peak as concrete failure example. The intrinsic limit comes from 1 ms gap reconstruction binning + cross-box reference resolution. Sub-millisecond burst structure (FRBs, ms magnetar peaks) is below the method's reach.

- [ ] **Step 3: Write subsection 6.3 (Silent drops, mechanism real but undetectable)**

Source: DESIGN.md §"静默丢数（Silent drop）检测". Mechanism real (FPGA blocking under saturation), no hardware flag, attempted detection had high false-positive rate, contribution to total event loss is dominated by FIFO-reset losses anyway (FIFO resets follow silent drops within ms in deep saturation). **Do NOT frame as method limitation** — frame as "honestly characterized hardware behavior".

- [ ] **Step 4: Write subsection 6.4 (Why CRC-failed event recovery is infeasible)**

Source: DESIGN.md §"CRC 失败事件恢复". 50% CRC error rate during saturation, LIS spacing ~50 ticks ≈ 0.1 ms means random ptime almost always finds a slot, contamination >> recovery. Net gain: 3 events out of 549,661.

- [ ] **Step 5: Compile, verify Section 6 occupies ~1.5 pages, no dead-zone subsection present**

```bash
grep -i "dead zone" paper/main_en.tex
```

Expected: matches only in Section 3.3 boundary-safeguard sentence (Task 8).

- [ ] **Step 6: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Section 6 (limits and failure modes)"
```

---

### Task 15: Section 7 (Conclusion) — ~0.5 pages

**Files:**
- Modify: `paper/main_en.tex` §conclusion

- [ ] **Step 1: Write 4-bullet conclusion + open-source pointer**

Mirror spec §1.3 contribution list, reframed as accomplished work:

1. LIS-based time reconstruction recovers events from 1B raw data with intrinsic CRC-ghost robustness (260226A residual N events).
2. Grouped-LIS + UTC-tail constraint resolves wrap ambiguity for $\Delta s_{\rm time}\le 10$ s with proven uniqueness.
3. Three-box cross-reconstruction recovers FIFO-reset gaps; ~92% coverage for 221009A.
4. Limits quantified: $\le$1 ms intrinsic resolution; ~8% three-box co-saturation residual; ~4% NaN coverage in extreme bursts.

End with a sentence referencing the open-source release (Zenodo DOI, MIT license) — placeholder for now, fill in Task 19.

- [ ] **Step 2: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: draft Section 7 (conclusion)"
```

---

### Task 16: Finalize abstract with frozen numbers

**Files:**
- Modify: `paper/main_en.tex` `\begin{abstract}...\end{abstract}` block

- [ ] **Step 1: Read current numbers from numbers.csv**

```bash
cat paper/numbers.csv
```

Note key values: 260226A residual count, 221009A per-box coverage %, three-box availability %.

- [ ] **Step 2: Write abstract using spec §2.2 template, substituting actual numbers**

The template is in spec §2.2. Substitute `549,661` and `3` with actual current values, `96%` with actual coverage, `~4%` with actual residual.

- [ ] **Step 3: Compile, verify abstract length ~200 words, fits SCPMA's abstract box**

```bash
cd paper && pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
```

Open PDF, check abstract appearance. Word-count target: 180-220.

- [ ] **Step 4: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: finalize abstract with frozen numbers"
```

---

## Phase F — Polish + submission package (D 8-10)

### Task 17: Full pass — terminology, citations, page count, no LaTeX warnings

**Files:**
- Modify: `paper/main_en.tex` (light edits)

- [ ] **Step 1: Terminology audit**

```bash
grep -nE "(LIS|FIFO|SEC|MET|MCU|ptime|stime|1B|1K|HE|HXMT|GRB|CRC|SEE)" paper/main_en.tex \
  | head -100
```

Verify: capitalization consistent, abbreviations defined on first use, no spelling drift. For example: "SEC" (not "Sec" — that's our `\section{}` reference), "ptime" lowercase italic in math, "MCU" all caps.

- [ ] **Step 2: Citation audit**

```bash
cd paper && pdflatex main_en && bibtex main_en 2>&1 | tee bib_pass.log
grep -i 'warning\|undefined\|missing' bib_pass.log
```

Expected: zero warnings. Fix every `Undefined citation` and `Missing definition`.

- [ ] **Step 3: Page count target check**

```bash
pdfinfo paper/main_en.pdf | grep Pages
```

Expected: 12-14 pages (text + refs). If >15 pages: tighten. If <11 pages: expand somewhere.

- [ ] **Step 4: Overfull/underfull box scan**

```bash
cd paper && pdflatex main_en 2>&1 | grep -iE 'overfull|underfull' | wc -l
```

Expected: <10 such warnings, all minor. Manually fix any \hbox warning >10 pt.

- [ ] **Step 5: Read through full PDF once for flow**

Open `paper/main_en.pdf` and read end to end — flag any place where Chinese phrasing leaked into English, where "we" is over-used, where transitions are awkward.

- [ ] **Step 6: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: full-pass polish (terminology, citations, page count)"
```

---

### Task 18: Cover letter

**Files:**
- Create: `paper/cover_letter.tex`

- [ ] **Step 1: Author cover letter following spec §6.4**

Write `paper/cover_letter.tex`:

```latex
\documentclass[11pt]{letter}
\usepackage{geometry}
\geometry{margin=1in}
\address{Hao-Xuan Guo \\ Institute of High Energy Physics, CAS \\ Beijing 100049, China \\ kuohaoxuanus@outlook.com}
\signature{Hao-Xuan Guo (corresponding-author-name TBD)}
\begin{document}
\begin{letter}{Editor-in-Chief \\ Science China Physics, Mechanics \& Astronomy}
\opening{Dear Editor,}

We submit for your consideration the manuscript entitled ``Saturation Recovery for Insight-HXMT/HE Bright Burst Observations from Level-1B Raw Data,'' reporting a saturation-recovery pipeline for the High-Energy detector aboard Insight-HXMT.

The HE detector's $\sim 5100\,\mathrm{cm}^2$ effective area in 20--250\,keV makes it uniquely sensitive to bright transients but also susceptible to FIFO-buffer saturation that is incompletely handled by the standard 1K processing pipeline. We address this gap by reconstructing event times directly from 1B raw telemetry using a longest-increasing-subsequence formulation, then recovering FIFO-reset losses via cross-detector-box shape functions calibrated against simultaneous unsaturated boxes. The work is validated through internal 1B/1K consistency checks, HE's own engineering-data counters, and cross-satellite light-curve comparisons against Fermi/GBM, GECAM-C, and INTEGRAL/SPI-ACS.

Highlights:
\begin{itemize}
\item Near-lossless recovery on moderately saturated GRB 260226A (residual $N$ events out of $X$).
\item Quantified method limits on extreme GRB 221009A: $\sim$96\,\% coverage with the remaining $\sim 4\,\%$ in three-box co-saturation regions.
\item Three independent validation layers: same-instrument engineering counters, cross-detector-box, and cross-satellite.
\end{itemize}

The pipeline is implemented in Rust and released MIT-licensed (Zenodo DOI \texttt{10.XXXX/zenodo.YYYY}).

We suggest the following potential reviewers: [TBD --- 3-5 names not in the author team's collaboration tree, e.g., GBM/GECAM/INTEGRAL instrument-paper authors].

We confirm that this manuscript has not been published or submitted elsewhere.

\closing{Sincerely,}
\end{letter}
\end{document}
```

Replace `[TBD]` with 3-5 actual reviewer names by D 10.

- [ ] **Step 2: Compile and review**

```bash
cd paper && pdflatex cover_letter.tex
open cover_letter.pdf
```

- [ ] **Step 3: Commit**

```bash
git add paper/cover_letter.tex
git commit -m "paper: draft cover letter"
```

---

### Task 19: Zenodo release prep + Code & Data Availability statement

**Files:**
- Create: `LICENSE` (MIT)
- Create: `README.md` (top-level)
- Modify: `paper/main_en.tex` (insert availability statement)

- [ ] **Step 1: Add MIT LICENSE to repo root**

Write `/Users/skyair/Developer/ihep/blink/LICENSE`:

```
MIT License

Copyright (c) 2026 Hao-Xuan Guo and contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 2: Write top-level README**

```markdown
# blink — Insight-HXMT/HE 1B Saturation Recovery

Reference implementation of the saturation-recovery pipeline for HXMT/HE
described in Guo et al. (2026, submitted to Science China PMA).

## Build

```bash
cargo build --release
```

## Quick start

```bash
./target/release/blink_cli sat <EPOCH> [--box a|b|c] solve <TRIGGER>
./target/release/blink_cli sat <EPOCH> [--box a|b|c] reconstruct <TRIGGER>
```

See `crates/instruments/blink_hxmt_he/src/algorithms/saturation/DESIGN.md` for algorithm details.

## License

MIT.
```

- [ ] **Step 3: Tag a release for Zenodo**

```bash
git tag -a paper-submission -m "Manuscript-frozen state for SCPMA submission"
git push origin paper-submission
```

Configure Zenodo (https://zenodo.org/account/settings/github/) to mint a DOI for this tag. Note the DOI when assigned (typically within minutes).

- [ ] **Step 4: Insert Code & Data Availability statement in main_en.tex**

Just before `\bibliographystyle{plain}`, add:

```latex
\section*{Code and Data Availability}
The reconstruction pipeline (\texttt{blink\_cli}) is released under the MIT License at \url{https://github.com/<USER>/blink} (commit hash \texttt{<HASH>}, Zenodo DOI \texttt{10.XXXX/zenodo.YYYY}). Plotting and validation scripts are included in the same repository under \texttt{scripts/}. Insight-HXMT raw 1B telemetry is the property of the Institute of High Energy Physics, Chinese Academy of Sciences, and is available upon reasonable request to the HXMT Science Operation Center (\texttt{hxmtsoc@ihep.ac.cn}). Cross-validation data are from public archives: Fermi/GBM at HEASARC, GECAM-C at the National Space Science Data Center, and INTEGRAL/SPI-ACS at the ISDC.
```

Replace `<USER>`, `<HASH>`, and the DOI placeholder with actual values once Zenodo issues the DOI.

- [ ] **Step 5: Commit**

```bash
git add LICENSE README.md paper/main_en.tex
git commit -m "paper: MIT license, README, and Code & Data Availability statement"
```

---

### Task 20: Final compile + ORCID/affiliation/author list cleanup

**Files:**
- Modify: `paper/main_en.tex` author block

- [ ] **Step 1: Replace TBD author list with finalized list**

Get from advisor: full author list with ORCIDs, affiliations, corresponding-author designation.

Update `\Author{}` / `\Address{}` / `\Email{}` macros (or whatever SCPMA template uses) accordingly. Mark corresponding author per template convention (typically `\Email{...}` or `\thanks{}`).

- [ ] **Step 2: Final clean compile**

```bash
cd paper
rm -f main_en.aux main_en.bbl main_en.blg main_en.out main_en.log main_en.pdf
pdflatex main_en
bibtex main_en
pdflatex main_en
pdflatex main_en
ls main_en.pdf
```

Expected: `main_en.pdf` produced cleanly. Check first page (title + author block) for typos.

- [ ] **Step 3: Commit**

```bash
git add paper/main_en.tex
git commit -m "paper: finalize author list and addresses"
```

---

## Phase G — Submit (D 11)

### Task 21: Submit to SCPMA

**Files:** None (external action)

- [ ] **Step 1: Verify all artifacts present**

```bash
ls paper/main_en.pdf paper/main_en.tex paper/refs_en.bib paper/cover_letter.pdf paper/figures/f{1..8}*.pdf
```

All 8 figures + main PDF + cover letter PDF present.

- [ ] **Step 2: Bundle for submission**

Most journals want LaTeX source + PDF. Make sure all figures referenced from main_en.tex are in `paper/figures/`. Test "fresh-clone" compilability:

```bash
cp -r paper /tmp/paper-fresh
cd /tmp/paper-fresh
pdflatex main_en && bibtex main_en && pdflatex main_en && pdflatex main_en
ls main_en.pdf
```

Must succeed on a clean copy.

- [ ] **Step 3: Submit via SCPMA online portal**

Go to `https://www.editorialmanager.com/scpma/` (or current SCPMA submission URL) — corresponding author logs in, creates new submission, uploads:
- main_en.tex + refs_en.bib + scpma.cls + figures/*.pdf
- main_en.pdf (compiled)
- cover_letter.pdf
- Suggested reviewer list

Confirm submission, save the manuscript ID.

- [ ] **Step 4: Final commit recording submission state**

```bash
git tag -a paper-submitted -m "SCPMA submission ID: <ID>"
git push origin saturation paper-submitted
```

---

## Self-Review Notes

### Spec coverage

- Spec §1 (narrative framing) → Task 6 (intro) + Task 8 §3 opening + Task 10 §4 opening
- Spec §2 (title/abstract) → Task 1 (title in scaffold) + Task 16 (abstract finalize)
- Spec §3 (chapter outline) → Tasks 6-15 (one task per section)
- Spec §4.1 (figures) → Tasks 5, 9, 11, 13
- Spec §4.2 (tables) → Task 12 (T2-T4) + Task 8 (T1 in §3.4 prose)
- Spec §5.1 (numbers freeze) → Task 3
- Spec §5.2 (new content sections) → Task 9 (F6) + Task 12 §5.2/5.3.3
- Spec §5.3 (rewrite Sec 4 etc.) → Task 10 (Sec 4 expanded)
- Spec §5.4 (bibliography) → Task 4
- Spec §5.5 (LaTeX template) → Task 2
- Spec §5.6 (cover letter etc.) → Task 18-19
- Spec §6.1 (authors) → Task 20
- Spec §6.2 (timeline) → entire phase structure
- Spec §6.3 (open source) → Task 19
- Spec §6.4 (cover letter) → Task 18
- Spec §7 (risks) → mitigations baked into individual task notes (Task 9 R1, Task 3 R2, Task 17 R5, etc.)

All spec items have a task. ✓

### Type / pathname consistency

- Path conventions: `paper/main_en.tex`, `paper/refs_en.bib`, `paper/figures/f{N}_*.pdf`, `paper/numbers.csv` — used consistently.
- Figure file names: `f1_datapath.pdf`, `f2_pipeline_phase.pdf`, `f3_lis_vs_greedy.pdf`, `f4_grouped_lis.pdf`, `f5_crossbox_recovery.pdf`, `f6_engineering_validation.pdf`, `f7_cross_satellite.pdf`, `f8_uniqueness.pdf`. Used consistently across Tasks 5, 9, 11, 13.
- LaTeX section labels: `\label{sec:intro|instrument|method|recovery|validation|discussion|conclusion}` — used consistently across tasks 1, 6-15.
- Table labels: `tab:221009a`, `tab:coverage`, `tab:crosssat` (Task 12) — referenced from Section 5 prose.
- Figure labels: `fig:datapath`, `fig:pipeline`, `fig:lis_vs_greedy`, `fig:grouped_lis`, `fig:crossbox_recovery`, `fig:eng_data` (TBD — add in Task 9), `fig:cross_sat` (TBD — add in Task 11), `fig:uniqueness` (TBD — add in Task 5/12). Need to standardize fig:eng_data/cross_sat/uniqueness labels at first use in main_en.tex; insert when each figure is placed.

### Placeholder scan

- Task 18 cover letter has `[TBD --- 3-5 names]` reviewer placeholder — flagged with explicit deadline (D 10).
- Task 19 has `<USER>`, `<HASH>`, DOI `10.XXXX/zenodo.YYYY` placeholders — flagged with explicit fill instructions.
- Task 20 has TBD author list — flagged as advisor-dependent.

These are NOT "TODOs to fill in later" but actual decision points where the engineer needs an external input (Zenodo DOI assignment, advisor input). Acceptable.

No "implement appropriate error handling" or "write tests for the above" — every step has concrete content or a precise external input requirement.

---
