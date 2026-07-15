#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Insight-HXMT / HE NaI calibration helpers.

This module implements the NaI calibration chain requested by the user:

    Channel -> Energy(piecewise E-C) -> PI(redistribution via e2p) -> EBOUNDS

Design choices
--------------
1. `hxmt_he_gain_20171030_v1.fits` drives Channel->Energy.
   - In Normal mode, we follow Li et al. (2020): three second-order polynomials.
   - In LowGain mode, we use one linear function.
2. `hxmt_he_e2p_20190311.fits` is used only for CHAN->PI redistribution.
3. EBOUNDS is rebuilt from the detector-wise E-C relation plus the e2p mapping.

Important implementation note
-----------------------------
The published E-C relation is

    E(ch) = p1 * ch^2 + p2 * ch + p3

and the uploaded gain FITS has one row per detector with columns:

    DetID, NP, TEMP, GC0, GC1, GC2, GC3, GC4, GC5

The actual numeric content of the uploaded file strongly supports the following
binding for the *normal-mode* NaI E-C relation:

    NP   -> number of piecewise segments (3 in this file)
    GC1  -> p1 of each segment
    GC2  -> p2 of each segment
    GC3  -> p3 of each segment

while `TEMP` is preserved as reference metadata and `GC0/GC4/GC5` are stored as
CALDB metadata for traceability.

For LowGain mode, this specific uploaded gain file does not contain an explicit,
non-zero detector-wise linear coefficient pair. Therefore the code uses the
following policy:

- if GC4/GC5 are non-zero for a detector, interpret them as the linear slope and
  intercept of the LowGain E-C relation;
- otherwise derive a detector-wise linear approximation from the highest-energy
  normal-mode segment over its valid range.

That keeps the code operational and explicitly ties the linear branch back to the
same detector-wise CALDB row, while being honest about what is and is not
explicitly present in the uploaded FITS.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

import numpy as np
from astropy.io import fits

# -----------------------------------------------------------------------------
# Published / analysis-level constants
# -----------------------------------------------------------------------------
N_DET = 18
N_CHAN = 256
N_PI = 256

# Li et al. 2020 normal-mode NaI piecewise energy ranges.
NORMAL_SEGMENTS_KEV: Tuple[Tuple[float, float], ...] = (
    (16.0, 33.17),
    (33.17, 50.2),
    (50.2, 350.0),
)

# HE analysis-level PI-energy relation from the HXMT guide.
HE_PI_E0_KEV = 15.0
HE_PI_SPAN_KEV = 370.0
HE_PI_E1_KEV = HE_PI_E0_KEV + HE_PI_SPAN_KEV

# Heuristic operational range for the fallback LowGain linearization.
LOWGAIN_FIT_RANGE_KEV = (50.2, 350.0)


# -----------------------------------------------------------------------------
# Small utilities
# -----------------------------------------------------------------------------
def _as_float_array(x) -> np.ndarray:
    return np.asarray(x, dtype=np.float64)



def _normalize_prob(p: np.ndarray) -> np.ndarray:
    p = _as_float_array(p)
    total = p.sum()
    if total <= 0:
        return np.zeros_like(p, dtype=np.float64)
    return p / total



def energy_to_pi_simple(energy_kev: np.ndarray | float, zero_based: bool = True) -> np.ndarray:
    """Analysis-level HE PI relation from the HXMT handbook."""
    e = _as_float_array(energy_kev)
    pi = np.rint(N_PI * (e - HE_PI_E0_KEV) / HE_PI_SPAN_KEV).astype(np.int64)
    pi = np.clip(pi, 0, N_PI - 1)
    if zero_based:
        return pi
    return pi + 1



def _copy_nonstructural_header(src: fits.Header, dst: fits.Header) -> fits.Header:
    skip_exact = {
        'XTENSION', 'BITPIX', 'NAXIS', 'NAXIS1', 'NAXIS2',
        'PCOUNT', 'GCOUNT', 'TFIELDS'
    }
    skip_prefix = ('TTYPE', 'TFORM', 'TUNIT', 'TDIM', 'TNULL', 'TSCAL', 'TZERO', 'TDISP')

    for key, value in src.items():
        if key in skip_exact:
            continue
        if any(key.startswith(prefix) for prefix in skip_prefix):
            continue
        if key in ('SIMPLE', 'EXTEND'):
            continue
        dst[key] = value
    return dst


# -----------------------------------------------------------------------------
# Gain / E-C calibration
# -----------------------------------------------------------------------------
@dataclass
class DetectorGainRow:
    det_id: int
    npiece: int
    temp: np.ndarray
    gc0: np.ndarray
    gc1: np.ndarray
    gc2: np.ndarray
    gc3: np.ndarray
    gc4: np.ndarray
    gc5: np.ndarray


class HEGainCalibration:
    """Detector-wise NaI E-C calibration driven by `hxmt_he_gain_*.fits`."""

    def __init__(self, gainfile: str | Path):
        self.gainfile = str(gainfile)
        self.rows: Dict[int, DetectorGainRow] = {}
        self._lowgain_cache: Dict[int, Tuple[float, float, str]] = {}
        self._read_gain_file()

    def _read_gain_file(self):
        with fits.open(self.gainfile) as hdul:
            tab = hdul['HE_Gain'].data
            self.header = hdul['HE_Gain'].header.copy()
            for row in tab:
                det = int(row['DetID'])
                self.rows[det] = DetectorGainRow(
                    det_id=det,
                    npiece=int(row['NP']),
                    temp=_as_float_array(row['TEMP']),
                    gc0=_as_float_array(row['GC0']),
                    gc1=_as_float_array(row['GC1']),
                    gc2=_as_float_array(row['GC2']),
                    gc3=_as_float_array(row['GC3']),
                    gc4=_as_float_array(row['GC4']),
                    gc5=_as_float_array(row['GC5']),
                )

    def _normal_polynomial_values(self, det_id: int, channel: np.ndarray) -> np.ndarray:
        row = self.rows[int(det_id)]
        ch = _as_float_array(channel)
        vals = []
        for i in range(row.npiece):
            vals.append(row.gc1[i] * ch * ch + row.gc2[i] * ch + row.gc3[i])
        return np.vstack(vals)

    def _select_normal_piece(self, det_id: int, channel: np.ndarray) -> Tuple[np.ndarray, np.ndarray]:
        """
        Choose the valid Normal-mode piece by checking which quadratic result lands
        inside the published segment energy range.
        """
        energies = self._normal_polynomial_values(det_id, channel)
        ch = _as_float_array(channel)
        n = ch.size
        chosen_energy = np.full(n, np.nan, dtype=np.float64)
        chosen_piece = np.full(n, -1, dtype=np.int16)

        for i, (emin, emax) in enumerate(NORMAL_SEGMENTS_KEV[:energies.shape[0]]):
            seg = energies[i]
            if i == energies.shape[0] - 1:
                valid = (seg >= emin) & (seg <= emax)
            else:
                valid = (seg >= emin) & (seg < emax)
            take = valid & (chosen_piece < 0)
            chosen_energy[take] = seg[take]
            chosen_piece[take] = i

        # Rare unresolved channels: choose the piece whose predicted energy is
        # closest to its nominal segment range.
        unresolved = chosen_piece < 0
        if np.any(unresolved):
            dist = np.full_like(energies, np.inf, dtype=np.float64)
            for i, (emin, emax) in enumerate(NORMAL_SEGMENTS_KEV[:energies.shape[0]]):
                seg = energies[i]
                dist[i] = np.where(seg < emin, emin - seg,
                                   np.where(seg > emax, seg - emax, 0.0))
            best = np.argmin(dist[:, unresolved], axis=0)
            idx = np.where(unresolved)[0]
            chosen_piece[idx] = best.astype(np.int16)
            chosen_energy[idx] = energies[best, idx]

        return chosen_energy, chosen_piece

    def _derive_lowgain_linear(self, det_id: int) -> Tuple[float, float, str]:
        """
        Return a detector-wise linear LowGain E-C relation.

        Priority:
        1) explicit non-zero GC4 / GC5 in the gain row;
        2) detector-wise linear fit derived from the highest-energy normal-mode piece.
        """
        det_id = int(det_id)
        if det_id in self._lowgain_cache:
            return self._lowgain_cache[det_id]

        row = self.rows[det_id]
        if row.gc4.size > 0 and row.gc5.size > 0 and (
            np.any(np.abs(row.gc4) > 0) or np.any(np.abs(row.gc5) > 0)
        ):
            slope = float(row.gc4[0])
            intercept = float(row.gc5[0])
            src = 'GC4_GC5'
        else:
            # Derive from the highest-energy normal-mode segment (piece index 2).
            ch = np.linspace(0.0, 255.0, 4096)
            seg_idx = min(max(row.npiece - 1, 0), 2)
            energy = row.gc1[seg_idx] * ch * ch + row.gc2[seg_idx] * ch + row.gc3[seg_idx]
            emin, emax = LOWGAIN_FIT_RANGE_KEV
            mask = (energy >= emin) & (energy <= emax)
            if mask.sum() < 8:
                mask = np.isfinite(energy)
            slope, intercept = np.polyfit(ch[mask], energy[mask], 1)
            slope = float(slope)
            intercept = float(intercept)
            src = 'derived_from_normal_piece_2'

        self._lowgain_cache[det_id] = (slope, intercept, src)
        return slope, intercept, src

    def channel_to_energy(self,
                          det_id: int,
                          channel: np.ndarray | float,
                          obs_mode: str = 'Normal',
                          return_piece: bool = False):
        """
        Convert channel to energy for one detector.

        Parameters
        ----------
        det_id : int
            HE detector id, 0..17.
        channel : array-like or float
            Raw channel values.
        obs_mode : {'Normal', 'LowGain', 'LHV', ...}
            Normal -> three quadratic pieces.
            LowGain/LHV -> one linear relation.
        return_piece : bool
            If True, also return the selected Normal-mode piece index.
        """
        ch = _as_float_array(channel)
        mode = str(obs_mode).strip().lower()

        if mode.startswith('low') or mode.startswith('lhv'):
            slope, intercept, _ = self._derive_lowgain_linear(det_id)
            energy = slope * ch + intercept
            if return_piece:
                return energy, np.full(ch.shape, -1, dtype=np.int16)
            return energy

        energy, piece = self._select_normal_piece(det_id, ch)
        if return_piece:
            return energy, piece
        return energy

    def channel_bounds_to_energy_bounds(self,
                                        det_id: int,
                                        channel: np.ndarray | float,
                                        obs_mode: str = 'Normal') -> Tuple[np.ndarray, np.ndarray]:
        """Channel-center -> channel-bound energy edges using ch±0.5."""
        ch = _as_float_array(channel)
        e_low = self.channel_to_energy(det_id, ch - 0.5, obs_mode=obs_mode)
        e_high = self.channel_to_energy(det_id, ch + 0.5, obs_mode=obs_mode)
        swap = e_high < e_low
        if np.any(swap):
            tmp = e_low[swap].copy()
            e_low[swap] = e_high[swap]
            e_high[swap] = tmp
        return e_low, e_high

    def describe_detector(self, det_id: int) -> dict:
        row = self.rows[int(det_id)]
        info = {
            'DetID': row.det_id,
            'NP': row.npiece,
            'TEMP': row.temp.copy(),
            'GC0': row.gc0.copy(),
            'GC1': row.gc1.copy(),
            'GC2': row.gc2.copy(),
            'GC3': row.gc3.copy(),
            'GC4': row.gc4.copy(),
            'GC5': row.gc5.copy(),
        }
        lg_m, lg_b, lg_src = self._derive_lowgain_linear(det_id)
        info['LOWGAIN_SLOPE'] = lg_m
        info['LOWGAIN_INTERCEPT'] = lg_b
        info['LOWGAIN_SOURCE'] = lg_src
        return info


# -----------------------------------------------------------------------------
# E2P redistribution calibration
# -----------------------------------------------------------------------------
class HEE2PCalibration:
    """Detector-wise CHAN->PI redistribution from `hxmt_he_e2p_*.fits`."""

    def __init__(self, e2pfile: str | Path):
        self.e2pfile = str(e2pfile)
        self.pi_map: Dict[int, List[np.ndarray]] = {}
        self.eff_map: Dict[int, List[np.ndarray]] = {}
        self._read_e2p_file()

    def _read_e2p_file(self):
        with fits.open(self.e2pfile) as hdul:
            self.headers = {}
            for det_id in range(N_DET):
                extname = f'DET{det_id}'
                hdu = hdul[extname]
                self.headers[det_id] = hdu.header.copy()
                pis = []
                effs = []
                for row in hdu.data:
                    pi = np.asarray(row['PI'], dtype=np.int16)
                    eff = _normalize_prob(np.asarray(row['EFF'], dtype=np.float64))
                    pis.append(pi)
                    effs.append(eff)
                self.pi_map[det_id] = pis
                self.eff_map[det_id] = effs

    def distribution(self, det_id: int, channel: int) -> Tuple[np.ndarray, np.ndarray]:
        det_id = int(det_id)
        channel = int(channel)
        if channel < 0 or channel >= N_CHAN:
            return np.array([], dtype=np.int16), np.array([], dtype=np.float64)
        return self.pi_map[det_id][channel], self.eff_map[det_id][channel]

    def expected_pi(self, det_id: int, channel: int) -> float:
        pi, eff = self.distribution(det_id, channel)
        if len(pi) == 0:
            return np.nan
        return float(np.sum(pi.astype(np.float64) * eff))

    def sample_pi(self,
                  det_id: int,
                  channel: np.ndarray | Iterable[int],
                  rng: np.random.Generator) -> np.ndarray:
        ch = np.asarray(channel, dtype=np.int16)
        out = np.full(ch.shape, -1, dtype=np.int16)
        for i, c in enumerate(ch):
            pi, eff = self.distribution(int(det_id), int(c))
            if len(pi) == 0:
                continue
            if len(pi) == 1:
                out[i] = int(pi[0])
            else:
                out[i] = int(rng.choice(pi, p=eff))
        return out


# -----------------------------------------------------------------------------
# EBOUNDS builder
# -----------------------------------------------------------------------------
def _fill_monotonic_edges(values: np.ndarray) -> np.ndarray:
    x = np.arange(values.size, dtype=np.float64)
    y = values.astype(np.float64).copy()
    finite = np.isfinite(y)
    if not finite.any():
        # last-resort fallback to the handbook linear PI-energy relation
        return np.linspace(HE_PI_E0_KEV, HE_PI_E1_KEV, values.size, dtype=np.float64)
    if finite.sum() == 1:
        y[:] = y[finite][0]
    else:
        y[~finite] = np.interp(x[~finite], x[finite], y[finite])
    y = np.maximum.accumulate(y)
    return y



def build_ebounds(gainfile: str | Path,
                  e2pfile: str | Path,
                  obs_mode: str = 'Normal',
                  zero_based: bool = True) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
    """
    Build a common OGIP-style EBOUNDS extension.

    Method
    ------
    1) For each detector and raw channel, compute channel-edge energies from the
       detector-specific E-C relation.
    2) Use the e2p redistribution to collect all PI bins that the channel can map to.
    3) For each detector/PI, store the min/max contributing energies.
    4) Combine the 18 detectors with a median to get one common EBOUNDS table.
    """
    gain = HEGainCalibration(gainfile)
    e2p = HEE2PCalibration(e2pfile)

    det_pi_low = []
    det_pi_high = []

    channels = np.arange(N_CHAN, dtype=np.float64)
    for det_id in range(N_DET):
        e_low, e_high = gain.channel_bounds_to_energy_bounds(det_id, channels, obs_mode=obs_mode)
        pi_low = np.full(N_PI, np.nan, dtype=np.float64)
        pi_high = np.full(N_PI, np.nan, dtype=np.float64)

        for ch in range(N_CHAN):
            pi_list, _ = e2p.distribution(det_id, ch)
            if len(pi_list) == 0:
                continue
            for pi in np.asarray(pi_list, dtype=np.int64):
                if 0 <= pi < N_PI:
                    if np.isnan(pi_low[pi]):
                        pi_low[pi] = e_low[ch]
                        pi_high[pi] = e_high[ch]
                    else:
                        pi_low[pi] = min(pi_low[pi], e_low[ch])
                        pi_high[pi] = max(pi_high[pi], e_high[ch])

        det_pi_low.append(pi_low)
        det_pi_high.append(pi_high)

    e_min = np.nanmedian(np.vstack(det_pi_low), axis=0)
    e_max = np.nanmedian(np.vstack(det_pi_high), axis=0)

    e_min = _fill_monotonic_edges(e_min)
    e_max = _fill_monotonic_edges(e_max)

    # Enforce ordered, gap-free edges.
    for i in range(N_PI):
        if i == 0:
            if e_max[i] <= e_min[i]:
                e_max[i] = e_min[i] + 1e-6
        else:
            if e_min[i] < e_max[i - 1]:
                e_min[i] = e_max[i - 1]
            if e_max[i] <= e_min[i]:
                e_max[i] = e_min[i] + 1e-6

    if zero_based:
        pi_chan = np.arange(N_PI, dtype=np.int16)
    else:
        pi_chan = np.arange(1, N_PI + 1, dtype=np.int16)

    return pi_chan, e_min.astype(np.float32), e_max.astype(np.float32)


# -----------------------------------------------------------------------------
# FITS writers
# -----------------------------------------------------------------------------
def _rewrite_hdulist_preserve_rest(infile: str, new_evt_hdu: fits.BinTableHDU):
    with fits.open(infile) as hdul_old:
        primary = fits.PrimaryHDU(header=hdul_old[0].header)
        rest = [hdu.copy() for hdu in hdul_old[2:]]
    fits.HDUList([primary, new_evt_hdu] + rest).writeto(infile, overwrite=True)



def add_pi_column(infile: str,
                  gainfile: str | Path,
                  e2pfile: str | Path,
                  obs_mode: str = 'Normal',
                  seed: int = 1,
                  zero_based: bool = True,
                  detid_col: str = 'Det_ID',
                  channel_col: str = 'Channel'):
    """
    Add/overwrite the PI column in an HE event file.

    PI generation policy
    --------------------
    - preferred: detector-wise random sampling from e2p redistribution;
    - fallback: if a channel has no redistribution entry, compute energy from the
      detector-wise E-C relation and convert with the handbook PI-E relation.
    """
    gain = HEGainCalibration(gainfile)
    e2p = HEE2PCalibration(e2pfile)
    rng = np.random.default_rng(int(seed))

    with fits.open(infile) as hdul_old:
        evt_hdu = hdul_old[1]
        evt = evt_hdu.data
        evt_hdr = evt_hdu.header.copy()

        if detid_col not in evt.names:
            raise KeyError(f"Cannot find detector column '{detid_col}' in {infile}")
        if channel_col not in evt.names:
            raise KeyError(f"Cannot find channel column '{channel_col}' in {infile}")

        det = np.asarray(evt[detid_col], dtype=np.int16)
        chan = np.asarray(evt[channel_col], dtype=np.int16)

        pi0 = np.full(len(evt), -1, dtype=np.int16)
        energy = np.full(len(evt), np.nan, dtype=np.float64)
        piece = np.full(len(evt), -9, dtype=np.int16)

        for det_id in np.unique(det):
            idx = np.where(det == det_id)[0]
            e, seg = gain.channel_to_energy(int(det_id), chan[idx], obs_mode=obs_mode, return_piece=True)
            energy[idx] = e
            piece[idx] = seg
            pi0[idx] = e2p.sample_pi(int(det_id), chan[idx], rng)

        fallback = pi0 < 0
        if np.any(fallback):
            pi0[fallback] = energy_to_pi_simple(energy[fallback], zero_based=True).astype(np.int16)

        pi_write = pi0.copy()
        if not zero_based:
            pi_write = (pi_write + 1).astype(np.int16)

        # rebuild columns, replacing PI if it exists.
        new_cols = []
        pi_replaced = False
        for col in evt_hdu.columns:
            if col.name == 'PI':
                new_cols.append(fits.Column(name='PI', array=pi_write, format='I'))
                pi_replaced = True
            else:
                new_cols.append(col)

        if not pi_replaced:
            new_cols.append(fits.Column(name='PI', array=pi_write, format='I'))

        new_evt_hdu = fits.BinTableHDU.from_columns(new_cols, name=evt_hdu.name)
        _copy_nonstructural_header(evt_hdr, new_evt_hdu.header)
        new_evt_hdu.header['PIALG'] = ('E2P_RAND', 'PI from detector-wise e2p redistribution')
        new_evt_hdu.header['E2PFILE'] = (Path(e2pfile).name[:68], 'Channel-to-PI redistribution file')
        new_evt_hdu.header['GAINFILE'] = (Path(gainfile).name[:68], 'HE gain / E-C calibration file')
        new_evt_hdu.header['PISEED'] = (int(seed), 'Random seed used for PI redistribution')
        new_evt_hdu.header['WORKMODE'] = (str(obs_mode), 'Energy-calibration mode')
        new_evt_hdu.header['ECFORM'] = ('PWISE_Q2', 'Normal: 3-piece quadratic; LowGain: linear')

    _rewrite_hdulist_preserve_rest(infile, new_evt_hdu)



def add_ebounds_hdu(infile: str,
                    gainfile: str | Path,
                    e2pfile: str | Path,
                    obs_mode: str = 'Normal',
                    zero_based: bool = True):
    """Append or replace an OGIP-style EBOUNDS HDU."""
    pi_chan, e_min, e_max = build_ebounds(
        gainfile=gainfile,
        e2pfile=e2pfile,
        obs_mode=obs_mode,
        zero_based=zero_based,
    )

    with fits.open(infile) as hdul:
        keep = [hdu.copy() for hdu in hdul if hdu.name.upper() != 'EBOUNDS']

    cols = [
        fits.Column(name='PI', array=pi_chan, format='I'),
        fits.Column(name='E_MIN', array=e_min, format='E', unit='keV'),
        fits.Column(name='E_MAX', array=e_max, format='E', unit='keV'),
    ]
    ebounds = fits.BinTableHDU.from_columns(cols, name='EBOUNDS')
    ebounds.header['HDUCLASS'] = ('OGIP', 'format conforms to OGIP standard')
    ebounds.header['HDUCLAS1'] = ('RESPONSE', 'dataset relates to spectral response')
    ebounds.header['HDUCLAS2'] = ('EBOUNDS', 'PI to energy channel bounds')
    ebounds.header['GAINFILE'] = (Path(gainfile).name[:68], 'HE gain / E-C calibration file')
    ebounds.header['E2PFILE'] = (Path(e2pfile).name[:68], 'Channel-to-PI redistribution file')
    ebounds.header['WORKMODE'] = (str(obs_mode), 'Energy-calibration mode')
    ebounds.header['ECFORM'] = ('PWISE_Q2', 'Normal: 3-piece quadratic; LowGain: linear')

    fits.HDUList(keep + [ebounds]).writeto(infile, overwrite=True)
