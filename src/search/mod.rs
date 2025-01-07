pub mod algorithms;
pub mod light_curve;
pub mod poisson;
pub mod record;
pub mod trigger;

use algorithms::search_all;
use core::str::FromStr;
use fitsio::FitsFile;
use hifitime::prelude::*;
use itertools::Itertools;

pub fn calculate_hxmt(filename: &str) -> Vec<record::Record> {
    let mut fptr = FitsFile::open(filename).unwrap();
    let events = fptr.hdu("EVENTS").unwrap();
    let start: f64 = events.read_key(&mut fptr, "TSTART").unwrap();
    let stop: f64 = events.read_key(&mut fptr, "TSTOP").unwrap();
    let date_obs: String = events.read_key(&mut fptr, "DATE-OBS").unwrap();
    let date_obs = Epoch::from_str(&date_obs).unwrap();
    let channel: Vec<u8> = events.read_col(&mut fptr, "Channel").unwrap();
    let time: Vec<_> = events.read_col::<f64>(&mut fptr, "Time").unwrap();
    let time: Vec<_> = time
        .iter()
        .zip(channel)
        .filter(|&(_, c)| c >= 38)
        .map(|(&t, _)| t - start)
        .dedup_by(|a, b| (a - b).abs() < 10e-9)
        .collect();
    calculate(&time, start, stop, date_obs, 100, 20.0, 8)
}

pub fn calculate(
    time: &[f64],
    start: f64,
    stop: f64,
    date_obs: Epoch,
    num_neighbors: usize,
    fp_year: f64,
    min_count: u32,
) -> Vec<record::Record> {
    let results = search_all(time, 0.0, stop - start, num_neighbors, fp_year, min_count);
    results
        .into_iter()
        .coalesce(|prev, next| {
            if prev.mergeable(&next, 0) {
                Ok(prev.merge(&next))
            } else {
                Err((prev, next))
            }
        })
        .map(|trigger| record::Record::new(&trigger, date_obs))
        .collect()
}

pub fn calculate_fermi_nai(filenames: &[&str]) -> Vec<record::Record> {
    let mut fptr = filenames
        .iter()
        .map(|filename| FitsFile::open(filename).unwrap())
        .collect::<Vec<_>>();
    let events = fptr
        .iter_mut()
        .map(|fptr| fptr.hdu("EVENTS").unwrap())
        .collect::<Vec<_>>();
    let time = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_col::<f64>(fptr, "TIME").unwrap())
        .collect::<Vec<_>>();
    let pha = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_col::<i16>(fptr, "PHA").unwrap())
        .collect::<Vec<_>>();
    let date_obs = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<String>(fptr, "DATE-OBS").unwrap())
        .map(|date_obs| Epoch::from_str(&date_obs).unwrap())
        .collect::<Vec<_>>();
    assert!(date_obs.iter().all(|x| x == &date_obs[0]));
    let date_obs = date_obs[0];
    let mjd_ref_i = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "MJDREFI").unwrap())
        .collect::<Vec<_>>();
    assert!(mjd_ref_i
        .iter()
        .all(|x| (x - mjd_ref_i[0]).abs() < f64::EPSILON));
    let mjd_ref_i = mjd_ref_i[0];
    let mjd_ref_f = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "MJDREFF").unwrap())
        .collect::<Vec<_>>();
    assert!(mjd_ref_f
        .iter()
        .all(|x| (x - mjd_ref_f[0]).abs() < f64::EPSILON));
    let mjd_ref_f = mjd_ref_f[0];
    let time_ref = Epoch::from_mjd_tai(mjd_ref_i + mjd_ref_f) - 32.184.seconds();
    let time_ref = Epoch::from_mjd_utc(time_ref.to_mjd_utc_days());
    let t_start = (date_obs - time_ref).to_seconds();

    let gti = fptr
        .iter_mut()
        .map(|fptr| fptr.hdu("GTI").unwrap())
        .collect::<Vec<_>>();
    let gti_start = gti
        .iter()
        .zip(fptr.iter_mut())
        .map(|(gti, fptr)| gti.read_col::<f64>(fptr, "START").unwrap())
        .collect::<Vec<_>>();
    let gti_stop = gti
        .iter()
        .zip(fptr.iter_mut())
        .map(|(gti, fptr)| gti.read_col::<f64>(fptr, "STOP").unwrap())
        .collect::<Vec<_>>();

    assert!(gti_start.iter().all(|x| x.len() == gti_start[0].len()));
    assert!(gti_stop.iter().all(|x| x.len() == gti_stop[0].len()));

    let gti_start = (0..gti_start[0].len())
        .map(|i| {
            gti_start
                .iter()
                .map(|x| x[i])
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let gti_stop = (0..gti_stop[0].len())
        .map(|i| {
            gti_stop
                .iter()
                .map(|x| x[i])
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();

    assert!(gti_start.len() == gti_stop.len());

    let time = time
        .into_iter()
        .flatten()
        .zip(pha.into_iter().flatten())
        .filter(|(_, pha)| *pha >= 30 && *pha <= 124)
        .map(|(time, _)| time)
        .sorted_by(|a, b| a.partial_cmp(b).unwrap())
        .dedup_by(|a, b| (a - b).abs() < 10e-9)
        .map(|time| time - t_start)
        .collect::<Vec<_>>();

    (0..gti_start.len())
        .flat_map(|i| {
            search_all(
                &time,
                gti_start[i] - t_start,
                gti_stop[i] - t_start,
                100,
                1.0,
                8,
            )
            .into_iter()
            .sorted_by(|a, b| a.start.partial_cmp(&b.start).unwrap())
            .coalesce(|prev, next| {
                if prev.mergeable(&next, 0) {
                    Ok(prev.merge(&next))
                } else {
                    Err((prev, next))
                }
            })
        })
        .map(|trigger| record::Record::new(&trigger, date_obs))
        .collect::<Vec<_>>()
}
