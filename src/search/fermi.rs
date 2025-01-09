use core::str::FromStr;
use fitsio::{hdu::FitsHdu, FitsFile};
use hifitime::prelude::*;
use itertools::Itertools;
use regex::Regex;

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

fn date_obs(fptrs: &mut [FitsFile], events: &[FitsHdu]) -> Epoch {
    events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_key::<String>(fptr, "DATE-OBS").unwrap())
        .map(|date_obs| Epoch::from_str(&date_obs).unwrap())
        .max()
        .unwrap()
}

fn t_start(fptrs: &mut [FitsFile], events: &[FitsHdu], date_obs: Epoch) -> f64 {
    let mjd_ref_i = events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "MJDREFI").unwrap())
        .collect::<Vec<_>>();
    assert!(mjd_ref_i
        .iter()
        .all(|x| (x - mjd_ref_i[0]).abs() < f64::EPSILON));
    let mjd_ref_i = mjd_ref_i[0];
    let mjd_ref_f = events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "MJDREFF").unwrap())
        .collect::<Vec<_>>();
    assert!(mjd_ref_f
        .iter()
        .all(|x| (x - mjd_ref_f[0]).abs() < f64::EPSILON));
    let mjd_ref_f = mjd_ref_f[0];
    let time_ref = Epoch::from_mjd_tai(mjd_ref_i + mjd_ref_f) - 32.184.seconds();
    let time_ref = Epoch::from_mjd_utc(time_ref.to_mjd_utc_days());
    (date_obs - time_ref).to_seconds()
}

fn gti(fptrs: &mut [FitsFile]) -> Vec<Interval> {
    fptrs
        .iter_mut()
        .map(|fptr| (fptr.hdu("GTI").unwrap(), fptr))
        .map(|(gti, fptr)| {
            let start = gti.read_col::<f64>(fptr, "START").unwrap();
            let stop = gti.read_col::<f64>(fptr, "STOP").unwrap();
            start
                .into_iter()
                .zip(stop)
                .map(|(start, stop)| Interval { start, stop })
                .collect::<Vec<_>>()
        })
        .reduce(interval_intersection)
        .unwrap()
}

pub fn calculate_fermi_nai(filenames: &[&str]) -> Vec<Record> {
    let mut fptrs = fptrs(filenames);
    let events = events(&mut fptrs);
    let time = time(&mut fptrs, &events);
    let pha = pha(&mut fptrs, &events);

    let gti = gti(&mut fptrs);
    let date_obs = date_obs(&mut fptrs, &events);
    let t_start = t_start(&mut fptrs, &events, date_obs);
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

    gti.iter()
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
        .collect::<Vec<_>>()
}

fn events(fptrs: &mut [FitsFile]) -> Vec<FitsHdu> {
    fptrs
        .iter_mut()
        .map(|fptr| fptr.hdu("EVENTS").unwrap())
        .collect::<Vec<_>>()
}

fn fptrs(filenames: &[&str]) -> Vec<FitsFile> {
    filenames
        .iter()
        .map(|filename| FitsFile::open(filename).unwrap())
        .collect::<Vec<_>>()
}

fn pha(fptrs: &mut [FitsFile], events: &[FitsHdu]) -> Vec<Vec<i16>> {
    events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_col::<i16>(fptr, "PHA").unwrap())
        .collect::<Vec<_>>()
}

fn time(fptrs: &mut [FitsFile], events: &[FitsHdu]) -> Vec<Vec<f64>> {
    events
        .iter()
        .zip(fptrs.iter_mut())
        .map(|(events, fptr)| events.read_col::<f64>(fptr, "TIME").unwrap())
        .collect::<Vec<_>>()
}

fn get_fermi_nai_filenames(epoch: &Epoch) -> Vec<String> {
    let (y, m, d, h, ..) = epoch.to_gregorian_utc();
    let folder = format!(
        "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily/{:04}/{:02}/{:02}/current",
        y, m, d
    );
    (0..12)
        .map(|i| {
            format!(
                "glg_tte_n{:x}_{:02}{:02}{:02}_{:02}z_v\\d{{2}}\\.fit\\.gz",
                i,
                y % 100,
                m,
                d,
                h,
            )
        })
        .map(|x| Regex::new(&x).unwrap())
        .map(|re| {
            std::fs::read_dir(&folder)
                .unwrap()
                .map(|x| x.unwrap())
                .map(|x| x.path())
                .filter(|x| x.is_file())
                .map(|x| x.file_name().unwrap().to_str().unwrap().to_string())
                .filter(|x| re.is_match(x))
                .max_by(|a, b| {
                    let extract = |x: &str| {
                        x.split('_')
                            .last()
                            .unwrap()
                            .strip_prefix('v')
                            .unwrap()
                            .strip_suffix(".fit.gz")
                            .unwrap()
                            .parse::<u32>()
                            .unwrap()
                    };
                    extract(a).cmp(&extract(b))
                })
                .unwrap()
        })
        .map(|x| format!("{}/{}", folder, x))
        .collect::<Vec<_>>()
}

pub fn process(epoch: &Epoch) -> Vec<Record> {
    let filenames = get_fermi_nai_filenames(&epoch);
    let filenames_str: Vec<&str> = filenames.iter().map(|s| s.as_str()).collect();
    calculate_fermi_nai(&filenames_str)
}
