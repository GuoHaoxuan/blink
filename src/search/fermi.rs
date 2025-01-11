use core::str::FromStr;
use fitsio::{hdu::FitsHdu, FitsFile};
use hifitime::prelude::*;
use itertools::Itertools;
use regex::Regex;
use std::error::Error;

use super::{algorithms::search_all, record::Record};

struct Interval {
    start: f64,
    stop: f64,
}

fn interval_intersection(a: Vec<Interval>, b: Vec<Interval>) -> Vec<Interval> {
    let mut res = Vec::new();
    let mut a_iter = a.into_iter().peekable();
    let mut b_iter = b.into_iter().peekable();

    while let (Some(a), Some(b)) = (a_iter.peek(), b_iter.peek()) {
        let start = a.start.max(b.start);
        let stop = a.stop.min(b.stop);
        if start < stop {
            res.push(Interval { start, stop });
        }
        if a.stop < b.stop {
            a_iter.next();
        } else {
            b_iter.next();
        }
    }
    res
}

fn date_obs(fptrs: &mut [FitsFile], events: &[FitsHdu]) -> Result<Epoch, Box<dyn Error>> {
    let date_obs = events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_key::<String>(fptr, "DATE-OBS"))
        .collect::<Result<Vec<_>, _>>()?;

    let min_date_obs = date_obs
        .into_iter()
        .map(|date_obs| Epoch::from_str(&date_obs))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .min()
        .ok_or("No valid DATE-OBS found")?;

    Ok(min_date_obs)
}

fn t_start(
    fptrs: &mut [FitsFile],
    events: &[FitsHdu],
    date_obs: Epoch,
) -> Result<f64, Box<dyn Error>> {
    let mjd_ref_i = events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "MJDREFI"))
        .collect::<Result<Vec<_>, _>>()?;
    if !mjd_ref_i
        .iter()
        .all(|x| (x - mjd_ref_i[0]).abs() < f64::EPSILON)
    {
        return Err("MJDREFI values are not consistent".into());
    }
    let mjd_ref_i = mjd_ref_i[0];

    let mjd_ref_f = events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "MJDREFF"))
        .collect::<Result<Vec<_>, _>>()?;
    if !mjd_ref_f
        .iter()
        .all(|x| (x - mjd_ref_f[0]).abs() < f64::EPSILON)
    {
        return Err("MJDREFF values are not consistent".into());
    }
    let mjd_ref_f = mjd_ref_f[0];

    let time_ref = Epoch::from_mjd_tai(mjd_ref_i + mjd_ref_f) - 32.184.seconds();
    let time_ref = Epoch::from_mjd_utc(time_ref.to_mjd_utc_days());
    Ok((date_obs - time_ref).to_seconds())
}

fn gti(fptrs: &mut [FitsFile]) -> Result<Vec<Interval>, Box<dyn Error>> {
    let intervals = fptrs
        .iter_mut()
        .map(|fptr| {
            let gti = fptr.hdu("GTI")?;
            let start = gti.read_col::<f64>(fptr, "START")?;
            let stop = gti.read_col::<f64>(fptr, "STOP")?;
            Ok::<Vec<Interval>, fitsio::errors::Error>(
                start
                    .into_iter()
                    .zip(stop)
                    .map(|(start, stop)| Interval { start, stop })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    intervals
        .into_iter()
        .reduce(interval_intersection)
        .ok_or_else(|| "No intervals found".into())
}

pub fn calculate_fermi_nai(filenames: &[&str]) -> Result<Vec<Record>, Box<dyn Error>> {
    let mut fptrs = fptrs(filenames)?;
    let events = events(&mut fptrs)?;
    let time = time(&mut fptrs, &events)?;
    let pha = pha(&mut fptrs, &events)?;

    let gti = gti(&mut fptrs)?;
    let date_obs = date_obs(&mut fptrs, &events)?;
    let t_start = t_start(&mut fptrs, &events, date_obs)?;
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

    Ok(gti
        .iter()
        .flat_map(|interval| {
            search_all(
                &time,
                interval.start - t_start,
                interval.stop - t_start,
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
        .map(|trigger| Record::new(&trigger, date_obs))
        .collect::<Vec<_>>())
}

fn events(fptrs: &mut [FitsFile]) -> Result<Vec<FitsHdu>, fitsio::errors::Error> {
    fptrs.iter_mut().map(|fptr| fptr.hdu("EVENTS")).collect()
}

fn fptrs(filenames: &[&str]) -> Result<Vec<FitsFile>, fitsio::errors::Error> {
    filenames
        .iter()
        .map(|filename| FitsFile::open(filename))
        .collect()
}

fn pha(fptrs: &mut [FitsFile], events: &[FitsHdu]) -> Result<Vec<Vec<i16>>, fitsio::errors::Error> {
    events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_col::<i16>(fptr, "PHA"))
        .collect()
}

fn time(
    fptrs: &mut [FitsFile],
    events: &[FitsHdu],
) -> Result<Vec<Vec<f64>>, fitsio::errors::Error> {
    events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_col::<f64>(fptr, "TIME"))
        .collect()
}

fn get_fermi_nai_filenames(epoch: &Epoch) -> Result<Vec<String>, Box<dyn Error>> {
    let (y, m, d, h, ..) = epoch.to_gregorian_utc();
    let folder = format!(
        "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily/{:04}/{:02}/{:02}/current",
        y, m, d
    );
    let mut filenames = Vec::new();
    for i in 0..12 {
        let pattern = format!(
            "glg_tte_n{:x}_{:02}{:02}{:02}_{:02}z_v\\d{{2}}\\.fit\\.gz",
            i,
            y % 100,
            m,
            d,
            h,
        );
        let re = Regex::new(&pattern)?;
        let max_file = std::fs::read_dir(&folder)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str().map(String::from))
            })
            .filter(|name| re.is_match(name))
            .max_by(|a, b| {
                let extract_version = |name: &str| {
                    name.split('_')
                        .last()
                        .and_then(|s| s.strip_prefix('v'))
                        .and_then(|s| s.strip_suffix(".fit.gz"))
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0)
                };
                extract_version(a).cmp(&extract_version(b))
            });
        if let Some(file) = max_file {
            filenames.push(format!("{}/{}", folder, file));
        }
    }
    if filenames.is_empty() {
        return Err("No matching files found".into());
    }
    Ok(filenames)
}

pub fn process(epoch: &Epoch) -> Result<Vec<Record>, Box<dyn Error>> {
    let filenames = get_fermi_nai_filenames(&epoch)?;
    let filenames_str: Vec<&str> = filenames.iter().map(|s| s.as_str()).collect();
    calculate_fermi_nai(&filenames_str)
}
