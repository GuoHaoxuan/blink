"""HXMT/HE per-second raw data product extractor (one date per invocation).

Reads 1B HE_Eng + 1K HE-HV / Orbit / Att / HE-Evt FITS files for one UTC date
and emits ``per_sec_parquet/{YYYYMMDD}.parquet`` (~48 columns, ~1.56M rows).
ETL only — no filtering. Idempotent: if output exists, exits 0.

Server paths default to:
    1B   /hxmtfs/data/Archive_tmp/1B
    1K   /hxmt/work/HXMT-DATA/1K
Override via environment variables BLINK_1B_ROOT and BLINK_1K_ROOT for local
testing.

CLI:
    python3 extract_per_sec_day.py YYYYMMDD [--output-dir DIR]
"""
from __future__ import annotations

import os
from pathlib import Path

MET_CORRECTION = 4.0  # 1B Time → 1K MET, verified sub-microsecond

BOX_PORTS = {"A": "0766", "B": "1009", "C": "1781"}
BOX_INDEX = {"A": 0, "B": 1, "C": 2}

DEFAULT_1B_ROOT = "/hxmtfs/data/Archive_tmp/1B"
DEFAULT_1K_ROOT = "/hxmt/work/HXMT-DATA/1K"


def root_1b() -> Path:
    return Path(os.environ.get("BLINK_1B_ROOT", DEFAULT_1B_ROOT))


def root_1k() -> Path:
    return Path(os.environ.get("BLINK_1K_ROOT", DEFAULT_1K_ROOT))


if __name__ == "__main__":
    raise NotImplementedError("CLI is implemented in Task 12")
