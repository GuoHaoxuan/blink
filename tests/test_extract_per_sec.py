"""Tests for scripts/extract_per_sec_day.py."""
from __future__ import annotations

import math

import extract_per_sec_day as M


def test_module_imports():
    """Sanity check: the worker module is importable."""
    assert M.MET_CORRECTION == 4.0
    assert M.BOX_PORTS == {"A": "0766", "B": "1009", "C": "1781"}
    assert M.BOX_INDEX == {"A": 0, "B": 1, "C": 2}


def test_compute_offset_basic():
    # From HE_Eng row 0 of data/1B/2017/20171001/0766/...
    assert M.compute_offset(utc_last_bdc=181439999, stime_last_bdc=1618548) == 179821451


def test_compute_met_float_basic():
    # Verified: 1618548 + 179821451 + 4.0 = 181440003.0
    out = M.compute_met_float(time_1b=1618548, offset=179821451)
    assert math.isclose(out, 181440003.0, abs_tol=1e-9)


def test_compute_met_float_array():
    import numpy as np
    times = np.array([1618548, 1618549, 1618550], dtype=np.int64)
    out = M.compute_met_float(time_1b=times, offset=179821451)
    np.testing.assert_allclose(out, [181440003.0, 181440004.0, 181440005.0])
