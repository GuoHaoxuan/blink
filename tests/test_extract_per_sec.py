"""Tests for scripts/extract_per_sec_day.py."""
from __future__ import annotations

import extract_per_sec_day as M


def test_module_imports():
    """Sanity check: the worker module is importable."""
    assert M.MET_CORRECTION == 4.0
    assert M.BOX_PORTS == {"A": "0766", "B": "1009", "C": "1781"}
    assert M.BOX_INDEX == {"A": 0, "B": 1, "C": 2}
