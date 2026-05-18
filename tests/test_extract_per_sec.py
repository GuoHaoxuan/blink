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


def test_count_acd_bits_zero():
    import numpy as np
    acd = np.zeros((1, 18), dtype=bool)
    assert M.count_acd_bits(acd).tolist() == [0]


def test_count_acd_bits_single():
    import numpy as np
    acd = np.zeros((3, 18), dtype=bool)
    acd[0, 0] = True
    acd[1, 5] = True
    acd[2, 17] = True
    assert M.count_acd_bits(acd).tolist() == [1, 1, 1]


def test_count_acd_bits_multi():
    import numpy as np
    acd = np.zeros((2, 18), dtype=bool)
    acd[0, [0, 1, 3]] = True   # 3 bits
    acd[1, :] = True            # 18 bits
    assert M.count_acd_bits(acd).tolist() == [3, 18]


def test_window_indices_basic():
    import numpy as np
    times = np.array([10.0, 10.5, 11.0, 11.5, 12.0])
    i_start, i_end = M.window_indices(times, 10.0, 11.5)
    # half-open [10.0, 11.5): indices 0,1,2  → i_start=0, i_end=3
    assert i_start == 0
    assert i_end == 3


def test_window_indices_empty():
    import numpy as np
    times = np.array([1.0, 2.0, 3.0])
    i_start, i_end = M.window_indices(times, 10.0, 11.0)
    assert i_start == 3
    assert i_end == 3   # zero-length window past end
