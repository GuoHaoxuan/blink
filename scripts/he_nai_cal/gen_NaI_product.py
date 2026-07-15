#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Generate Insight-HXMT / HE NaI products with a full CALDB-driven energy scale:

    raw Channel -> detector-wise E-C -> PI (via e2p redistribution) -> EBOUNDS

Compared with the user-provided CsI workflow, this NaI version keeps the same
1K search / cut / dead-time generation framework, and changes only the
energy-calibration path:

1) retain NaI physical events only (Pulse_Width window + calibration-event veto);
2) use `hxmt_he_gain_*.fits` for detector-wise piecewise E-C in Normal mode;
3) use `hxmt_he_e2p_*.fits` only for CHAN->PI redistribution;
4) rebuild EBOUNDS from the E-C relation plus the redistribution mapping.
"""

from __future__ import annotations

import argparse
import glob
import os
import re
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
from astropy.io import fits
from astropy.table import Table, vstack
from astropy.time import Time
import astropy.units as u

import DeadTime
import nai_pha2pi


# -----------------------------------------------------------------------------
# Time helpers
# -----------------------------------------------------------------------------
def utc_to_met(utc: str) -> float:
    utc_t0 = Time('2012-01-01T00:00:00.000', scale='utc')
    return (Time(utc, scale='utc') - utc_t0).sec



def met_to_utc(met: float) -> str:
    met_t0 = Time('2012-01-01T00:00:00.000', scale='utc')
    return (met_t0 + float(met) * u.s).isot



def parse_trigger_time(trig_t: str) -> tuple[float, str]:
    try:
        trig_met = utc_to_met(trig_t)
        trig_utc = met_to_utc(trig_met)
        return trig_met, trig_utc
    except Exception:
        trig_met = float(trig_t)
        trig_utc = met_to_utc(trig_met)
        return trig_met, trig_utc



def get_full_hours_between(utc_time1: str, utc_time2: str):
    time1 = datetime.strptime(utc_time1, "%Y-%m-%dT%H:%M:%S.%f")
    time2 = datetime.strptime(utc_time2, "%Y-%m-%dT%H:%M:%S.%f")

    start_hour = time1.replace(minute=0, second=0, microsecond=0)
    end_hour = time2.replace(minute=0, second=0, microsecond=0)

    full_hours = []
    while start_hour <= end_hour:
        full_hours.append(start_hour.strftime("%Y-%m-%dT%H:%M:%S.%f"))
        start_hour += timedelta(hours=1)
    return full_hours


# -----------------------------------------------------------------------------
# File search / merge
# -----------------------------------------------------------------------------
def search_file(full_hours, file_key):
    file_list = []
    for utc in full_hours:
        utctime_start = Time(utc, scale='utc')
        utctime2 = Time(utctime_start.mjd, scale='utc', format='mjd')
        utctime_str = utctime2.isot

        search_path = (
            '/hxmtfs2/work/HXMT-DATA/1K/'
            + 'Y' + utctime_str[0:4] + utctime_str[5:7]
            + '/' + utctime_str[0:4] + utctime_str[5:7] + utctime_str[8:10] + '-*'
            + '/HXMT_' + utctime_str[0:4] + utctime_str[5:7] + utctime_str[8:13]
        )

        print(f'search rule: {search_path}_{file_key}_*.FITS')
        all_files = sorted(glob.glob(search_path + f'_{file_key}_*.FITS'))
        print(f'search result: {all_files}')

        if not all_files:
            print(f'No file found for {file_key} at {utc}')
            continue

        def extract_version(fname):
            m = re.search(r'_V(\d+)_', fname)
            return int(m.group(1)) if m else -1

        all_files.sort(key=extract_version)
        file_list.append(all_files[-1])

    print(f'gen product: {file_list}')
    return file_list



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



def cut_1K_fits(fits_files, met_pre, met_aft, output_file):
    fits_data = [fits.open(file) for file in fits_files]
    hdu_names = [hdu.name for hdu in fits_data[0][1:]]

    hdu_list = fits.HDUList([fits.PrimaryHDU(header=fits_data[0][0].header)])

    for hdu_name in hdu_names:
        all_data = []
        header = None

        for file_data in fits_data:
            hdu = file_data[hdu_name]
            if not hasattr(hdu, 'columns') or 'Time' not in hdu.columns.names:
                continue
            mask = (hdu.data['Time'] >= met_pre) & (hdu.data['Time'] <= met_aft)
            all_data.append(hdu.data[mask])
            if header is None:
                header = hdu.header.copy()

        if len(all_data) == 0:
            continue

        table_list = [Table(data) for data in all_data]
        combined_table = vstack(table_list)
        first_col = combined_table.colnames[0]
        _, idx = np.unique(combined_table[first_col], return_index=True)
        data_unique = combined_table[np.sort(idx)]

        new_hdu = fits.BinTableHDU(data=data_unique.as_array(), name=hdu_name)
        _copy_nonstructural_header(header, new_hdu.header)
        hdu_list.append(new_hdu)

    hdu_list.writeto(output_file, overwrite=True)

    for file_data in fits_data:
        file_data.close()


# -----------------------------------------------------------------------------
# FITS header helpers
# -----------------------------------------------------------------------------
def change_keyword(infile, keyword, value, comment=None, after=None):
    with fits.open(infile, mode='update') as hdl:
        for i in range(len(hdl)):
            hdl[i].header.set(keyword, value, comment, after=after)
        hdl.flush()



def change_common_keywords(out_file_name, utc_pre, utc_aft, met_aft, met_pre, file_key_id, heb_name):
    if 'ID913' not in file_key_id:
        change_keyword(out_file_name, 'DATLEVEL', '1G', after='INSTRUME')
        change_keyword(out_file_name, 'DATE-OBS', utc_pre, after='DATLEVEL')
        change_keyword(out_file_name, 'DATE-END', utc_aft, after='DATE-OBS')
        change_keyword(out_file_name, 'TELAPSE', round(met_aft - met_pre, 2), after='TSTOP')
        change_keyword(out_file_name, 'DATAID', file_key_id, after='DATLEVEL')
        change_keyword(out_file_name, 'HEBID', heb_name, after='DATAID')
        change_keyword(out_file_name, 'TIMEREF', 'LOCAL', after='TELAPSE')
        change_keyword(out_file_name, 'TASSIGN', 'SATELLITE', after='TIMEREF')
        change_keyword(out_file_name, 'TIMEUNIT', 's', after='TASSIGN')
        change_keyword(out_file_name, 'TIMEZERO', 0, after='TIMEUNIT')
        change_keyword(out_file_name, 'CLOCKAPP', False, after='TIMEZERO')

    change_keyword(out_file_name, 'TSTART', round(met_pre, 2))
    change_keyword(out_file_name, 'TSTOP', round(met_aft, 2))


# -----------------------------------------------------------------------------
# Working mode / NaI event selection
# -----------------------------------------------------------------------------
def judge_work_mode(file, met_time):
    """
    Keep the same mode judgement used in the user-provided CsI script.

    Event_Type == 1 denotes internal 241Am events. If such events exist near the
    trigger time, use Normal mode; otherwise LowGain.
    """
    with fits.open(file) as hdl:
        tbt = hdl[1].data
        if 'Event_Type' not in tbt.names or 'Time' not in tbt.names:
            return 'Normal'
        sel_mode = tbt[
            (tbt['Event_Type'] == 1)
            & (tbt['Time'] >= met_time - 10)
            & (tbt['Time'] <= met_time + 20)
        ]
    return 'Normal' if sel_mode.shape[0] > 0 else 'LowGain'



def _get_existing_colname(names, *candidates):
    lowered = {name.lower(): name for name in names}
    for cand in candidates:
        if cand.lower() in lowered:
            return lowered[cand.lower()]
    return None



def keep_only_nai_xray_events(infile, min_pulse_width=54, max_pulse_width=70):
    """
    Keep only NaI physical X-ray events.

    Rules used:
    - Pulse_Width within [54, 70] -> NaI window from the HXMT handbook.
    - If Event_Type exists, keep Event_Type == 0 only, i.e. reject internal
      241Am calibration events.
    """
    with fits.open(infile) as hdul_old:
        evt_hdu = hdul_old[1]
        evt = evt_hdu.data
        evt_hdr = evt_hdu.header.copy()
        names = list(evt.names)

        pw_col = _get_existing_colname(names, 'Pulse_Width', 'PulseWidth', 'PULSE_WIDTH')
        et_col = _get_existing_colname(names, 'Event_Type', 'EventType', 'EVENT_TYPE')

        mask = np.ones(len(evt), dtype=bool)
        if pw_col is not None:
            mask &= (evt[pw_col] >= min_pulse_width) & (evt[pw_col] <= max_pulse_width)
        else:
            print('WARNING: no Pulse_Width column found; NaI/CsI separation was not applied.')

        if et_col is not None:
            mask &= (evt[et_col] == 0)
        else:
            print('WARNING: no Event_Type column found; calibration-event rejection was not applied.')

        filtered = evt[mask]
        cols = []
        for col in evt_hdu.columns:
            cols.append(fits.Column(name=col.name, array=filtered[col.name], format=col.format))

        new_evt_hdu = fits.BinTableHDU.from_columns(cols, name=evt_hdu.name)
        _copy_nonstructural_header(evt_hdr, new_evt_hdu.header)

        primary = fits.PrimaryHDU(header=hdul_old[0].header)
        rest = [hdu.copy() for hdu in hdul_old[2:]]

    fits.HDUList([primary, new_evt_hdu] + rest).writeto(infile, overwrite=True)


# -----------------------------------------------------------------------------
# Main workflow
# -----------------------------------------------------------------------------
def main(trig_t,
         pathstr='./',
         pre_time=100.0,
         aft_time=200.0,
         gainfile='hxmt_he_gain_20171030_v1.fits',
         e2pfile='hxmt_he_e2p_20190311.fits',
         seed=1,
         pi_zero_based=True):
    trig_met, trig_utc = parse_trigger_time(trig_t)

    met_pre = trig_met - pre_time
    met_aft = trig_met + aft_time
    utc_pre = met_to_utc(met_pre)
    utc_aft = met_to_utc(met_aft)
    full_hour_list = get_full_hours_between(utc_pre, utc_aft)

    print(f'Trigger UTC: {trig_utc}')
    print(f'Trigger MET: {trig_met}')

    file_key_lists = ['Att', 'HE-Cnts', 'HE-DTime', 'HE-PM', 'Orbit', 'HE-HV', 'HE-TH', 'HE-Evt']
    file_key_ids = ['ID912', 'ID908', 'ID909', 'ID911', 'ID913', 'ID910', 'ID907', 'ID901']

    heb_name = (
        'HEB' + trig_utc[2:4] + trig_utc[5:7] + trig_utc[8:10] + '_'
        + trig_utc[11:13] + trig_utc[14:16] + trig_utc[17:19]
    )
    burst_folder = os.path.join(pathstr, heb_name)
    os.makedirs(burst_folder, exist_ok=True)
    print(f'burst_folder: {burst_folder}')

    for file_index, file_key in enumerate(file_key_lists):
        file_lists = search_file(full_hour_list, file_key)
        outfile = f'{burst_folder}/{heb_name}_{file_key}.fits'
        print(f'Cut files into: {file_key}')

        if len(file_lists) == 0:
            print(f'No file was found in {file_key}')
            continue

        cut_1K_fits(file_lists, met_pre, met_aft, outfile)
        change_common_keywords(outfile, utc_pre, utc_aft, met_aft, met_pre, file_key_ids[file_index], heb_name)

        if file_key == 'HE-Evt':
            dtime_binsize = 0.005
            overload_flag, bit_start, bit_stop = DeadTime.dead_time(
                trig_met, pre_time, aft_time, dtime_binsize, burst_folder + '/'
            )

            mode = judge_work_mode(outfile, trig_met)

            change_keyword(outfile, 'TRIGMET', round(trig_met, 2), 'Trigger time in MET format')
            change_keyword(outfile, 'TRIGUTC', trig_utc, 'Trigger time in UTC format')
            change_keyword(outfile, 'MJDREFI', 55927, after='TSTOP')
            change_keyword(outfile, 'MJDREFF', 7.6601852000000E-04, after='MJDREFI')
            change_keyword(outfile, 'OVERLOAD', overload_flag)
            change_keyword(outfile, 'WORKMODE', mode, 'HE work mode near trigger')
            change_keyword(outfile, 'HETYPE', 'NaI', 'NaI-only product after pulse-width selection')
            change_keyword(outfile, 'GAINFILE', Path(gainfile).name, 'HE gain / E-C calibration file')
            change_keyword(outfile, 'E2PFILE', Path(e2pfile).name, 'HE channel-to-PI redistribution file')
            change_keyword(outfile, 'PISEED', int(seed), 'Random seed used for PI redistribution')
            change_keyword(outfile, 'ECFORM', 'PWISE_Q2', 'Normal: 3-piece quadratic; LowGain: linear')

            if overload_flag == 1:
                bit_start = np.round(bit_start + trig_met, 2)
                bit_stop = np.round(bit_stop + trig_met, 2)
                change_keyword(outfile, 'OLSTART', ','.join(map(str, bit_start)), 'overload time start in MET format')
                change_keyword(outfile, 'OLSTOP', ','.join(map(str, bit_stop)), 'overload time stop in MET format')

            # 1) keep NaI X-ray events only
            keep_only_nai_xray_events(outfile, min_pulse_width=54, max_pulse_width=70)

            # 2) add PI from detector-wise E-C + detector-wise e2p redistribution
            nai_pha2pi.add_pi_column(
                outfile,
                gainfile=gainfile,
                e2pfile=e2pfile,
                obs_mode=mode,
                seed=seed,
                zero_based=pi_zero_based,
            )

            # 3) append EBOUNDS rebuilt from E-C + e2p
            nai_pha2pi.add_ebounds_hdu(
                outfile,
                gainfile=gainfile,
                e2pfile=e2pfile,
                obs_mode=mode,
                zero_based=pi_zero_based,
            )


if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description='Generate HXMT/HE NaI products with CALDB-driven Channel->PI->EBOUNDS.'
    )
    parser.add_argument('--trigger_time', dest='trigger_time', type=str,
                        default='2018-09-01T18:22:00.000',
                        help='Trigger time, UTC isot string or MET seconds.')
    parser.add_argument('--out_path', dest='out_path', type=str, default='./',
                        help='Output directory.')
    parser.add_argument('--pre_time', dest='pre_time', type=float, default=100.0,
                        help='Relative cut time before trigger time (s).')
    parser.add_argument('--aft_time', dest='aft_time', type=float, default=100.0,
                        help='Relative cut time after trigger time (s).')
    parser.add_argument('--gainfile', dest='gainfile', type=str,
                        default='hxmt_he_gain_20171030_v1.fits',
                        help='Path to hxmt_he_gain_*.fits')
    parser.add_argument('--e2pfile', dest='e2pfile', type=str,
                        default='hxmt_he_e2p_20190311.fits',
                        help='Path to hxmt_he_e2p_*.fits')
    parser.add_argument('--seed', dest='seed', type=int, default=1,
                        help='Random seed for PI redistribution.')
    parser.add_argument('--pi_one_based', action='store_true',
                        help='Use 1-based PI numbering (default: False, i.e. 0-based PI).')
    args = parser.parse_args()

    main(args.trigger_time,
         args.out_path,
         args.pre_time,
         args.aft_time,
         gainfile=args.gainfile,
         e2pfile=args.e2pfile,
         seed=args.seed,
         pi_zero_based=(not args.pi_one_based))
