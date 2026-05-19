# Extract MET Offset and Segment-File Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two extract bugs in `scripts/extract_per_sec_day.py` — multi-segment 1B HE_Eng files (use latest segment) and anomalous-offset detection/override — then re-run the full 3260-day archive.

**Architecture:** Two independent surgical fixes. (1) Change `find_he_eng_path` to pick highest segment number. (2) Add a `compute_effective_offsets(date, port)` helper that pre-scans 24 hours' HE_Eng offsets per (box, day), detects outliers (> 10 s deviation from day median), and emits a `{hour: effective_offset}` map. Add `override_offset` parameter to `read_he_eng`. Wire into `extract_day` by calling the helper once per box per day before the per-hour loop.

**Tech Stack:** Python 3.9+, astropy.io.fits, numpy, pandas, pytest

---

### Task 1: `find_he_eng_path` selects highest segment

**Files:**
- Modify: `scripts/extract_per_sec_day.py:320-334`
- Test: `tests/test_extract_per_sec.py` (append)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_extract_per_sec.py`:

```python
def test_find_he_eng_path_picks_highest_segment(monkeypatch, tmp_path):
    """When multiple segments exist for one hour, pick the highest number."""
    # Build a synthetic 1B directory layout with two segments for hour 3.
    yr = tmp_path / "1B" / "2022" / "20220115" / "0766"
    yr.mkdir(parents=True)
    (yr / "HXMT_1B_0766_20220115T030000_G040183_000_004.fits").touch()
    (yr / "HXMT_1B_0766_20220115T030000_G040183_001_004.fits").touch()
    (yr / "HXMT_1B_0766_20220115T030000_G040183_002_004.fits").touch()

    monkeypatch.setenv("BLINK_1B_ROOT", str(tmp_path / "1B"))

    p = M.find_he_eng_path("20220115", 3, "0766")
    assert p is not None
    assert p.name == "HXMT_1B_0766_20220115T030000_G040183_002_004.fits"


def test_find_he_eng_path_single_segment_unchanged(monkeypatch, tmp_path):
    """When only one segment exists, behaviour unchanged."""
    yr = tmp_path / "1B" / "2022" / "20220115" / "0766"
    yr.mkdir(parents=True)
    (yr / "HXMT_1B_0766_20220115T040000_G040184_000_004.fits").touch()

    monkeypatch.setenv("BLINK_1B_ROOT", str(tmp_path / "1B"))

    p = M.find_he_eng_path("20220115", 4, "0766")
    assert p is not None
    assert p.name == "HXMT_1B_0766_20220115T040000_G040184_000_004.fits"


def test_find_he_eng_path_no_match(monkeypatch, tmp_path):
    """When no segment exists, returns None."""
    yr = tmp_path / "1B" / "2022" / "20220115" / "0766"
    yr.mkdir(parents=True)

    monkeypatch.setenv("BLINK_1B_ROOT", str(tmp_path / "1B"))

    p = M.find_he_eng_path("20220115", 5, "0766")
    assert p is None
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
pytest tests/test_extract_per_sec.py::test_find_he_eng_path_picks_highest_segment -v
```

Expected: FAIL — current code returns `matches[0]` (the `_000_` file), test expects `_002_`.

- [ ] **Step 3: Apply fix**

Change line 334 in `scripts/extract_per_sec_day.py`:

```python
# Before:
    return Path(matches[0]) if matches else None
# After:
    return Path(matches[-1]) if matches else None
```

Update the docstring to mention segment selection:

```python
def find_he_eng_path(date: str, hour: int, port: str) -> Path | None:
    """Locate the 1B HE_Eng file for one (date, hour, port).

    Expected layout::
        {BLINK_1B_ROOT}/{YYYY}/{YYYYMMDD}/{port}/HXMT_1B_{port}_{YYYYMMDD}T{HH}0000_*.fits

    When the archive contains multiple segments for one hour
    (``..._000_004.fits`` and ``..._001_004.fits``), returns the highest-numbered
    segment, which observation has shown to be the more-complete version.

    Returns None if no matching file found.
    """
```

- [ ] **Step 4: Run tests, verify they pass**

```bash
pytest tests/test_extract_per_sec.py -v -k find_he_eng_path
```

Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/extract_per_sec_day.py tests/test_extract_per_sec.py
git commit -m "extract: pick highest segment in find_he_eng_path"
```

---

### Task 2: Pure-function offset outlier detector

**Files:**
- Modify: `scripts/extract_per_sec_day.py` (add helper near `compute_offset`)
- Test: `tests/test_extract_per_sec.py` (append)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_extract_per_sec.py`:

```python
def test_effective_offsets_no_outlier():
    """All hours within 10s of median: pass through unchanged."""
    raw = {0: 179821369, 1: 179821368, 2: 179821368, 3: 179821367}
    out = M.effective_offsets(raw, threshold_sec=10)
    assert out == raw


def test_effective_offsets_one_outlier_below():
    """One hour 800s below median: replace with neighbour median."""
    raw = {0: 179821369, 1: 179821369, 2: 179820567, 3: 179821369, 4: 179821368}
    out = M.effective_offsets(raw, threshold_sec=10)
    assert out[0] == 179821369
    assert out[1] == 179821369
    # Hour 2 is the outlier; immediate good neighbours are hours 1 and 3 (both 179821369)
    assert out[2] == 179821369
    assert out[3] == 179821369
    assert out[4] == 179821368


def test_effective_offsets_one_outlier_above():
    """Symmetric: also catches outliers above median."""
    raw = {0: 179821369, 1: 179822500, 2: 179821368}
    out = M.effective_offsets(raw, threshold_sec=10)
    # Hour 1 deviation = 1131 from median 179821369; gets overridden.
    # Neighbours that pass threshold: hours 0 (179821369) and 2 (179821368)
    assert out[1] in (179821368, 179821369)


def test_effective_offsets_no_good_neighbour_uses_median():
    """If immediate neighbours are also outliers, fall back to day median."""
    raw = {0: 179821369, 1: 179821369, 2: 179820567, 3: 179820567, 4: 179821369}
    out = M.effective_offsets(raw, threshold_sec=10)
    # Median is 179821369 (3 good hours vs 2 bad). Hours 2,3 both outliers; no
    # good immediate neighbour for hour 2's neighbour at hour 3, fallback to median.
    assert out[2] == 179821369


def test_effective_offsets_empty():
    """Empty input → empty output (no crash)."""
    assert M.effective_offsets({}, threshold_sec=10) == {}


def test_effective_offsets_single_hour():
    """One hour only → that hour is the median, no override."""
    raw = {5: 179821369}
    out = M.effective_offsets(raw, threshold_sec=10)
    assert out == raw
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
pytest tests/test_extract_per_sec.py -v -k effective_offsets
```

Expected: 6 FAIL — `M.effective_offsets` does not exist.

- [ ] **Step 3: Implement `effective_offsets`**

Add this function to `scripts/extract_per_sec_day.py` immediately after `compute_offset` (around line 70):

```python
def effective_offsets(
    raw_offsets: dict[int, int],
    threshold_sec: int = 10,
) -> dict[int, int]:
    """Detect and override anomalous per-hour HE_Eng offsets.

    Some 1B HE_Eng files have ``UTC_Last_Bdc - sTime_Last_Bdc`` offsets that
    deviate by hundreds of seconds from neighbouring hours, mis-placing the
    file's met_sec by that amount. Normal hour-to-hour drift is ~1 s/day.

    For each hour with deviation > ``threshold_sec`` from the day median,
    substitute the median of immediate-neighbour hours that pass threshold.
    Fall back to overall median if no good neighbours exist.

    Args:
        raw_offsets: ``{hour: offset}`` map for one (box, date).
        threshold_sec: max permitted deviation from day median.

    Returns:
        ``{hour: effective_offset}`` with outliers replaced. Same keys as input.
    """
    if not raw_offsets:
        return {}
    vals = list(raw_offsets.values())
    median = int(np.median(vals))
    good = {h: o for h, o in raw_offsets.items() if abs(o - median) <= threshold_sec}

    fixed: dict[int, int] = {}
    for h, off in raw_offsets.items():
        if abs(off - median) <= threshold_sec:
            fixed[h] = off
            continue
        # Outlier: search immediate-neighbour good hours, expanding outward.
        candidates = []
        for delta in (1, 2, 3):
            for h_n in (h - delta, h + delta):
                if h_n in good:
                    candidates.append(good[h_n])
            if candidates:
                break
        fixed[h] = int(np.median(candidates)) if candidates else median
    return fixed
```

- [ ] **Step 4: Run tests, verify they pass**

```bash
pytest tests/test_extract_per_sec.py -v -k effective_offsets
```

Expected: 6 PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/extract_per_sec_day.py tests/test_extract_per_sec.py
git commit -m "extract: add effective_offsets outlier detector"
```

---

### Task 3: Fast helper to probe a file's offset without reading data

**Files:**
- Modify: `scripts/extract_per_sec_day.py` (add helper)
- Test: `tests/test_extract_per_sec.py` (append)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_extract_per_sec.py`:

```python
def test_probe_he_eng_offset(require_file):
    """Probe just reads row 0 to extract offset — should be fast and correct."""
    from tests.conftest import HE_ENG_2017_BOXA
    require_file(HE_ENG_2017_BOXA)

    off = M.probe_he_eng_offset(HE_ENG_2017_BOXA)
    # Matches test_compute_offset_basic: 181439999 - 1618548 = 179821451
    assert off == 179821451


def test_probe_he_eng_offset_returns_none_for_bad_file(tmp_path):
    """Unreadable / non-existent file returns None (extract continues gracefully)."""
    bogus = tmp_path / "not_a_fits.fits"
    bogus.write_bytes(b"\x00" * 100)
    assert M.probe_he_eng_offset(bogus) is None
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
pytest tests/test_extract_per_sec.py -v -k probe_he_eng_offset
```

Expected: 2 FAIL — `M.probe_he_eng_offset` does not exist.

- [ ] **Step 3: Implement `probe_he_eng_offset`**

Add to `scripts/extract_per_sec_day.py` near `read_he_eng`:

```python
def probe_he_eng_offset(path) -> int | None:
    """Return UTC_Last_Bdc[0] - sTime_Last_Bdc[0] from an HE_Eng file.

    Used by the pre-scan in ``extract_day``: reads only the first row to get
    the offset constant, much faster than a full ``read_he_eng``. Returns
    ``None`` on any read error (caller treats as missing hour).
    """
    try:
        with fits.open(path, memmap=False) as f:
            d = f["HE_Eng"].data
            if len(d) == 0:
                return None
            return compute_offset(int(d["UTC_Last_Bdc"][0]),
                                  int(d["sTime_Last_Bdc"][0]))
    except Exception:
        return None
```

- [ ] **Step 4: Run tests, verify they pass**

```bash
pytest tests/test_extract_per_sec.py -v -k probe_he_eng_offset
```

Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/extract_per_sec_day.py tests/test_extract_per_sec.py
git commit -m "extract: add probe_he_eng_offset helper"
```

---

### Task 4: `read_he_eng` accepts `override_offset` parameter

**Files:**
- Modify: `scripts/extract_per_sec_day.py:101-165` (existing `read_he_eng`)
- Test: `tests/test_extract_per_sec.py` (append)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_extract_per_sec.py`:

```python
def test_read_he_eng_default_offset(require_file):
    """Without override: reader returns offset from file header."""
    from tests.conftest import HE_ENG_2017_BOXA
    require_file(HE_ENG_2017_BOXA)

    d = M.read_he_eng(HE_ENG_2017_BOXA)
    # The reader returns the file's own offset baseline.
    assert d["offset"] == 179821451


def test_read_he_eng_override_offset(require_file):
    """With override: reader returns the override value."""
    from tests.conftest import HE_ENG_2017_BOXA
    require_file(HE_ENG_2017_BOXA)

    d = M.read_he_eng(HE_ENG_2017_BOXA, override_offset=179800000)
    assert d["offset"] == 179800000
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
pytest tests/test_extract_per_sec.py -v -k "read_he_eng_default_offset or read_he_eng_override_offset"
```

Expected: 2 FAIL — `read_he_eng` does not yet return `offset`.

- [ ] **Step 3: Add `offset` field + override support**

In `scripts/extract_per_sec_day.py`, modify `read_he_eng` (line 101):

```python
def read_he_eng(path, *, override_offset: int | None = None) -> dict:
    """Read one 1B HE_Eng FITS file. Returns dict of numpy arrays + offset.

    The returned dict includes an ``offset`` key (int) — either the file's
    own ``UTC_Last_Bdc[0] - sTime_Last_Bdc[0]``, or ``override_offset`` when
    that argument is provided. Used downstream to compute met_sec.

    Schema (per-second, ~3600 rows per file):
        Time, Length_Time_Cycle:                shape (n,)         int
        UTC_Last_Bdc, sTime_Last_Bdc:           shape (n,)         int
        Cnt_PHODet, Cnt_OOCDet,
        Cnt_CsI_PHODet, Cnt_LargeEvt,
        DeadTime_PHODet:                        shape (n, 6)       int  (per-det)
        BUS_Time_Bdc:                           shape (n, 6)       uint8 (raw)
        Error_code:                             shape (n, 4)       uint8 (raw)
        offset:                                 int (scalar)
    """
    with fits.open(path, memmap=False) as f:
        d = f["HE_Eng"].data
        n = len(d)
        col_names = set(f["HE_Eng"].columns.names)

        # ... [keep the existing per_det helper and column-format detection] ...
        # (lines 116-150 unchanged)

        out = {
            # ... [existing entries] ...
        }

        # Resolve offset: caller-supplied override takes priority, else file header.
        file_offset = compute_offset(int(d["UTC_Last_Bdc"][0]),
                                     int(d["sTime_Last_Bdc"][0]))
        out["offset"] = int(override_offset) if override_offset is not None else file_offset

        return out
```

**Note:** Only change the function signature, add the `offset` key in the
returned dict, and update the docstring. Do not touch any of the
2017/2026 column-format detection logic.

- [ ] **Step 4: Run tests, verify they pass**

```bash
pytest tests/test_extract_per_sec.py -v
```

Expected: all existing tests still pass; the 2 new tests pass.

- [ ] **Step 5: Commit**

```bash
git add scripts/extract_per_sec_day.py tests/test_extract_per_sec.py
git commit -m "extract: read_he_eng accepts override_offset"
```

---

### Task 5: Pre-scan offsets in `extract_day` and use them per-hour

**Files:**
- Modify: `scripts/extract_per_sec_day.py` (`extract_day` around lines 481-538, `_box_hour_arrays` around lines 361-471)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_extract_per_sec.py`:

```python
def test_extract_day_no_dups_for_known_offset_bug(require_file, monkeypatch, tmp_path):
    """Integration test: 20220115 box A has a known bad offset in hour 2.
    After the fix, the produced dataframe has zero (box, det, met_sec) duplicates.
    """
    # Skip if the real 1B/1K archive paths aren't available
    src_b = Path("/hxmtfs/data/Archive_tmp/1B/2022/20220115")
    if not src_b.exists():
        import pytest
        pytest.skip("Real 1B archive not mounted")

    monkeypatch.setenv("BLINK_1B_ROOT", "/hxmtfs/data/Archive_tmp/1B")
    monkeypatch.setenv("BLINK_1K_ROOT", "/hxmt/work/HXMT-DATA/1K")

    df = M.extract_day("20220115")
    assert len(df) > 0
    dup_count = df.duplicated(subset=["box", "det", "met_sec"]).sum()
    assert dup_count == 0, f"Expected 0 dups, got {dup_count}"


def test_extract_day_monotonic_after_fix(require_file, monkeypatch):
    """Each (box, det) group has strictly increasing met_sec after the fix."""
    import numpy as np

    src_b = Path("/hxmtfs/data/Archive_tmp/1B/2022/20220115")
    if not src_b.exists():
        import pytest
        pytest.skip("Real 1B archive not mounted")

    monkeypatch.setenv("BLINK_1B_ROOT", "/hxmtfs/data/Archive_tmp/1B")
    monkeypatch.setenv("BLINK_1K_ROOT", "/hxmt/work/HXMT-DATA/1K")

    df = M.extract_day("20220115")
    for (box, det), sub in df.groupby(["box", "det"]):
        s = sub["met_sec"].to_numpy()
        assert (np.diff(s) > 0).all(), f"non-monotonic in ({box}, {det})"
```

These tests will be skipped unless the real archive is mounted. They drive the production fix verification.

- [ ] **Step 2: Run tests, verify the integration tests skip (locally)**

```bash
pytest tests/test_extract_per_sec.py -v -k "no_dups or monotonic_after_fix"
```

Expected: 2 SKIPPED (assuming no real archive locally). On the IHEP server the test will run and FAIL until step 3 ships.

- [ ] **Step 3: Refactor `extract_day` to pre-scan offsets per (box, date)**

The current flow walks `hour` outer, `box` inner. The fix swaps to `box` outer for the offset pre-scan, then keeps the existing hour×box processing. Replace `extract_day` (lines 481-538) with:

```python
def extract_day(date: str) -> pd.DataFrame:
    """Build the full per-sec dataframe for one UTC date."""
    # Step 1: per-box offset pre-scan — detect anomalous-offset hours.
    box_effective_offsets: dict[str, dict[int, int]] = {}
    for box in ("A", "B", "C"):
        port = BOX_PORTS[box]
        raw: dict[int, int] = {}
        for hour in range(24):
            path = find_he_eng_path(date, hour, port)
            if path is None:
                continue
            off = probe_he_eng_offset(path)
            if off is not None:
                raw[hour] = off
        eff = effective_offsets(raw, threshold_sec=10)
        # Emit a WARN log line for each overridden hour
        for h in raw:
            if eff[h] != raw[h]:
                print(
                    f"[per_sec_extract] WARN: box {box} hour {h:02d} offset override "
                    f"{raw[h]} → {eff[h]} (deviation {raw[h]-eff[h]:+d}s)",
                    file=sys.stderr, flush=True,
                )
        box_effective_offsets[box] = eff

    # Step 2: per-hour processing — pass the effective_offset for each (box, hour).
    parts: list[dict[str, np.ndarray]] = []
    for hour in range(24):
        hv_path    = find_1k_aux_path(date, hour, "HE-HV")
        orbit_path = find_1k_aux_path(date, hour, "Orbit")
        att_path   = find_1k_aux_path(date, hour, "Att")
        evt_path   = find_1k_aux_path(date, hour, "HE-Evt")

        hv_raw = _try_read(read_he_hv, hv_path, "HE-HV", date, hour)
        hv_table = _index_by_time(hv_raw) if hv_raw is not None else None

        orbit_raw = _try_read(read_orbit, orbit_path, "Orbit", date, hour)
        orbit_table = _index_by_time(orbit_raw) if orbit_raw is not None else None

        evt_table = _try_read(read_he_evt, evt_path, "HE-Evt", date, hour)

        for box in ["A", "B", "C"]:
            eng_path = find_he_eng_path(date, hour, BOX_PORTS[box])
            if eng_path is None:
                continue
            override = box_effective_offsets[box].get(hour)
            d_probe = _try_read(
                lambda p: read_he_eng(p, override_offset=override),
                eng_path, f"HE_Eng ({box})", date, hour,
            )
            if d_probe is None:
                continue
            met_sec_probe = np.floor(
                compute_met_float(d_probe["Time"], d_probe["offset"])
            ).astype(np.int64)

            att_vals = None
            if att_path is not None:
                try:
                    att_vals = read_att(att_path, met_sec_probe)
                except Exception as e:
                    print(f"[per_sec_extract] WARN: read Att for {date} hour {hour} failed: {e}",
                          file=sys.stderr, flush=True)
                    att_vals = None

            chunk = _box_hour_arrays(
                date, box, hour,
                hv_lookup=hv_table,
                orbit_lookup=orbit_table,
                att_lookup=att_vals,
                evt=evt_table,
                override_offset=override,
            )
            if chunk is not None:
                parts.append(chunk)

    if not parts:
        return pd.DataFrame()

    cols = list(parts[0].keys())
    data = {c: np.concatenate([p[c] for p in parts]) for c in cols}
    return pd.DataFrame(data)
```

Also modify `_box_hour_arrays` signature and offset computation (lines 361-388). Add `override_offset` parameter; in the body, replace lines 386-388:

```python
def _box_hour_arrays(
    date: str, box: str, hour: int,
    hv_lookup: dict | None,
    orbit_lookup: dict | None,
    att_lookup: dict | None,
    evt: dict | None,
    override_offset: int | None = None,
) -> dict[str, np.ndarray] | None:
    """Build column-arrays for all (sec × 6 det) rows of one (box, hour).

    Returns None if HE_Eng missing or unreadable. Otherwise returns a dict of
    numpy arrays, one per output column, all of length n_sec * 6.

    ``override_offset`` is the corrected offset from ``effective_offsets``;
    when provided, replaces the file's own UTC/sTime-derived offset.
    """
    eng_path = find_he_eng_path(date, hour, BOX_PORTS[box])
    if eng_path is None:
        return None
    try:
        d = read_he_eng(eng_path, override_offset=override_offset)
    except Exception as e:
        print(f"[per_sec_extract] WARN: read HE_Eng {eng_path} failed: {e}",
              file=sys.stderr, flush=True)
        return None

    n_sec = len(d["Time"])
    n_rows = n_sec * 6
    box_idx = BOX_INDEX[box]
    offset = d["offset"]
    met_float = compute_met_float(d["Time"], offset)
    met_sec = np.floor(met_float).astype(np.int64)
    # ... rest of function unchanged from line 389 onward ...
```

**Critical:** lines 389 onward (date/box/det column construction, HV lookup, Sci aggregation, Orbit/Att blocks) stay exactly as they were — only the function signature line and the offset-computation lines (386-388) change.

- [ ] **Step 4: Run all tests locally**

```bash
pytest tests/test_extract_per_sec.py -v
```

Expected:
- All previously-passing tests still pass.
- The 2 integration tests skip (no local archive) or pass on the server.

- [ ] **Step 5: Commit**

```bash
git add scripts/extract_per_sec_day.py tests/test_extract_per_sec.py
git commit -m "extract: pre-scan and override anomalous HE_Eng offsets"
```

---

### Task 6: Deploy to server and re-run full archive

**Files:**
- No code changes; this task is the production rollout.

- [ ] **Step 1: Push branch to remote**

```bash
git push origin per-sec-extract
```

- [ ] **Step 2: Pull on the server**

```bash
ssh guohx@lxlogin.ihep.ac.cn 'cd /scratchfs/gecam/guohx/blink && git pull --ff-only origin per-sec-extract'
```

If the IHEP server has no GitHub SSH key, fall back to rsync:

```bash
rsync -avz scripts/extract_per_sec_day.py guohx@lxlogin.ihep.ac.cn:/scratchfs/gecam/guohx/blink/scripts/
rsync -avz tests/test_extract_per_sec.py guohx@lxlogin.ihep.ac.cn:/scratchfs/gecam/guohx/blink/tests/
```

- [ ] **Step 3: Spot-test on 20220115 before mass re-run**

```bash
ssh guohx@lxlogin.ihep.ac.cn 'cd /scratchfs/gecam/guohx/blink && rm -f per_sec_parquet/20220115.parquet && BLINK_1B_ROOT=/hxmtfs/data/Archive_tmp/1B BLINK_1K_ROOT=/hxmt/work/HXMT-DATA/1K python3 scripts/extract_per_sec_day.py 20220115 --output-dir per_sec_parquet 2>&1 | tail -30'
```

Verify the output:

```bash
ssh guohx@lxlogin.ihep.ac.cn 'cd /scratchfs/gecam/guohx/blink && python3 -c "
import pyarrow.parquet as pq
t = pq.read_table(\"per_sec_parquet/20220115.parquet\").to_pandas()
print(f\"rows: {len(t)}\")
print(f\"dups: {t.duplicated([\"box\",\"det\",\"met_sec\"]).sum()}\")
print(f\"unique met_sec per box: {t.groupby(\"box\")[\"met_sec\"].nunique().to_dict()}\")
"'
```

Expected:
- `dups: 0`
- WARN line(s) in stderr for box A hour 2 (offset override).
- Total rows for box A is ~3589 more than before for the affected hours (segment fix).

- [ ] **Step 4: Delete all old parquets**

```bash
ssh guohx@lxlogin.ihep.ac.cn 'cd /scratchfs/gecam/guohx/blink && rm -i per_sec_parquet/*.parquet'
```

(Use `rm -f` if no manual confirmation needed.)

- [ ] **Step 5: Re-submit full range via hep_sub**

```bash
ssh guohx@lxlogin.ihep.ac.cn 'cd /scratchfs/gecam/guohx/blink && nohup ./scripts/submit_extract_jobs.sh > logs/submit.log 2>&1 &'
```

Default range: 2017-06-15 → today. Submission rate ~40 jobs/min; cluster gives ~50-62 concurrent slots; ~16 hours wall time expected.

- [ ] **Step 6: Tomorrow — verify completion and re-run QA**

```bash
ssh guohx@lxlogin.ihep.ac.cn '
echo "queue: $(/afs/ihep.ac.cn/soft/common/sysgroup/hep_job/bin/hep_q -u guohx | tail -1)"
echo "parquets: $(ls /scratchfs/gecam/guohx/blink/per_sec_parquet/*.parquet | wc -l) / 3260"
echo "errors: $(find /scratchfs/gecam/guohx/blink/logs/extract -name "*.err" -size +200c | wc -l)"
'
```

Re-run the QA scan from `qa_scan.py` against the new output. Acceptance:
- 0 days with dup keys (was 82/96).
- 0 days with non-monotonic groups (was 34/96).
- High-NaN days (corrupt 1K aux) unchanged — that's source-data, not extract.

- [ ] **Step 7: Final commit / merge**

Once QA passes, merge `per-sec-extract` into `main`:

```bash
git checkout main
git merge per-sec-extract
git push origin main
```
