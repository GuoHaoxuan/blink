use core::str::FromStr;
use fitsio::{hdu::FitsHdu, FitsFile};
use hifitime::prelude::*;
use itertools::Itertools;
use regex::Regex;
use std::error::Error;
use std::iter::zip;

use crate::search::event::PackedEvent;

use super::algorithms::{search, SearchConfig};
use super::event::Event;
use super::interval::Interval;

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

fn gti(fits_files: &mut [FitsFile], time_ref: Epoch) -> Result<Vec<Interval>, Box<dyn Error>> {
    let intervals = fits_files
        .iter_mut()
        .map(|fits_file| {
            let gti = fits_file.hdu("GTI")?;
            let start = gti.read_col::<f64>(fits_file, "START")?;
            let stop = gti.read_col::<f64>(fits_file, "STOP")?;
            Ok::<Vec<Interval>, fitsio::errors::Error>(
                start
                    .into_iter()
                    .zip(stop)
                    .map(|(start, stop)| Interval {
                        start: time_ref + start.seconds(),
                        stop: time_ref + stop.seconds(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    intervals
        .into_iter()
        .reduce(interval_intersection)
        .ok_or_else(|| "No intervals found".into())
}

fn events(fits_files: &mut [FitsFile]) -> Result<Vec<FitsHdu>, fitsio::errors::Error> {
    fits_files
        .iter_mut()
        .map(|fits_file| fits_file.hdu("EVENTS"))
        .collect()
}

fn fits_files(filenames: &[&str]) -> Result<Vec<FitsFile>, fitsio::errors::Error> {
    filenames.iter().map(FitsFile::open).collect()
}

fn pha(
    fits_files: &mut [FitsFile],
    events: &[FitsHdu],
) -> Result<Vec<Vec<i16>>, fitsio::errors::Error> {
    events
        .iter()
        .zip(fits_files.iter_mut())
        .map(|(events, fits_file)| events.read_col::<i16>(fits_file, "PHA"))
        .collect()
}

fn time(
    fits_files: &mut [FitsFile],
    events: &[FitsHdu],
) -> Result<Vec<Vec<f64>>, fitsio::errors::Error> {
    events
        .iter()
        .zip(fits_files.iter_mut())
        .map(|(events, fits_file)| events.read_col::<f64>(fits_file, "TIME"))
        .collect()
}

pub fn calculate_fermi(filenames: &[&str]) -> Result<Vec<Interval>, Box<dyn Error>> {
    let time_ref = Epoch::from_str("2001-01-01T00:00:00.000000000 UTC")?;

    let mut fits_files = fits_files(filenames)?;
    let events = events(&mut fits_files)?;
    let time = time(&mut fits_files, &events)?;
    let pha = pha(&mut fits_files, &events)?;
    drop(events);
    let gti = gti(&mut fits_files, time_ref)?;
    drop(fits_files);
    let events = zip(time, pha)
        .enumerate()
        .flat_map(|(i, (time, pha))| {
            zip(time, pha)
                .map(|(time, pha)| PackedEvent {
                    time,
                    pi: pha,
                    detector: i as u8,
                })
                .collect::<Vec<_>>()
        })
        .sorted_by(|a, b| a.time.partial_cmp(&b.time).unwrap())
        .dedup_by_with_count(|a, b| (a.time - b.time).abs() < 0.3e-6)
        .filter(|(count, _)| *count == 1)
        .map(|(_, event)| event)
        .filter(|event| event.pi >= 30 && event.pi <= 124)
        .map(|event| Event {
            time: time_ref + event.time.seconds(),
            // pi is unused now
            // pi: event.pi as u32,
            detector: event.detector,
            group: match event.detector {
                0..=2 => 0,
                3..=5 => 1,
                6..=8 => 2,
                9..=11 => 3,
                12 => 4,
                13 => 5,
                _ => 6, // unreachable
            },
        })
        .collect::<Vec<_>>();

    Ok(gti
        .into_iter()
        .flat_map(|interval| {
            search(
                &events,
                6,
                interval.start,
                interval.stop,
                SearchConfig {
                    ..Default::default()
                },
            )
        })
        .collect())
}

fn get_fermi_filenames(epoch: &Epoch) -> Result<Vec<String>, Box<dyn Error>> {
    let (y, m, d, h, ..) = epoch.to_gregorian_utc();
    let folder = format!(
        "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily/{:04}/{:02}/{:02}/current",
        y, m, d
    );
    let mut filenames = Vec::new();
    for i in 0..12 {
        let pattern = format!(
            "glg_tte_{}_{:02}{:02}{:02}_{:02}z_v\\d{{2}}\\.fit\\.gz",
            if i < 12 {
                format!("n{:x}", i)
            } else {
                format!("b{:x}", i - 12)
            },
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

pub fn process(epoch: &Epoch) -> Result<Vec<Interval>, Box<dyn Error>> {
    let filenames = get_fermi_filenames(epoch)?;
    let filenames_str: Vec<&str> = filenames.iter().map(|s| s.as_str()).collect();
    calculate_fermi(&filenames_str)
}
