use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use chrono::{prelude::*, Duration};
use itertools::Itertools;
use nav_types::{ECEF, WGS84};

use crate::env::GBM_DAILY_PATH;
use crate::lightning::Lightning;
use crate::search::algorithms::{search, SearchConfig};
use crate::types::{Event as _, Signal, Time, TimeUnits};

use super::detector::FermiDetectorType;
use super::event::FermiEvent;
use super::file::{self, File};
use super::{Fermi, Position};

pub(crate) struct Hour {
    files: Vec<File>,
    position: Position,
    span: [Time<Fermi>; 2],
}

impl Hour {
    pub(crate) fn new(
        data: &[&str],
        position: &str,
        span: [Time<Fermi>; 2],
    ) -> Result<Self, fitsio::errors::Error> {
        let detectors = (0..14)
            .map(|i| {
                if i < 12 {
                    FermiDetectorType::Nai(i)
                } else {
                    FermiDetectorType::Bgo(i - 12)
                }
            })
            .collect::<Vec<_>>();
        let files = data
            .iter()
            .zip(detectors.iter())
            .map(|(filename, detector)| File::new(filename, *detector))
            .collect::<Result<Vec<_>, _>>()?;
        let position = Position::new(position)?;
        Ok(Self {
            files,
            position,
            span,
        })
    }

    pub(crate) fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self, Box<dyn Error>> {
        let y = epoch.year();
        let m = epoch.month();
        let d = epoch.day();
        let h = epoch.hour();
        let folder = Path::new(&*GBM_DAILY_PATH)
            .join(format!("{:04}/{:02}/{:02}/current", y, m, d))
            .into_os_string();
        let filenames: Vec<String> = (0..14)
            .map(|i| -> Result<String, Box<dyn Error>> {
                let prefix = format!(
                    "glg_tte_{}_{:02}{:02}{:02}_{:02}z_v",
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
                get_file(&folder, &prefix)
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
        let position_prefix = format!("glg_poshist_all_{:02}{:02}{:02}_v", y % 100, m, d);
        let position_file = get_file(&folder, &position_prefix)?;
        Ok(Self::new(
            &filenames.iter().map(|f| f.as_str()).collect::<Vec<_>>(),
            &position_file,
            [
                Time::<Fermi>::from(*epoch),
                Time::<Fermi>::from(*epoch + Duration::hours(1)),
            ],
        )?)
    }

    pub(crate) fn gti(&self) -> Vec<[Time<Fermi>; 2]> {
        self.files
            .iter()
            .map(|file| file.gti())
            .chain(std::iter::once(vec![self.span]))
            .reduce(|a, b| {
                let mut res = Vec::new();
                let mut a_iter = a.into_iter().peekable();
                let mut b_iter = b.into_iter().peekable();

                while let (Some(a), Some(b)) = (a_iter.peek(), b_iter.peek()) {
                    let start = a[0].max(b[0]);
                    let stop = a[1].min(b[1]);
                    if start < stop {
                        res.push([start, stop]);
                    }
                    if a[1] < b[1] {
                        a_iter.next();
                    } else {
                        b_iter.next();
                    }
                }
                res
            })
            .unwrap()
    }

    pub(crate) fn search(&self) -> Result<Vec<Signal>, Box<dyn Error>> {
        let events: Vec<FermiEvent> = self
            .into_iter()
            .dedup_by_with_count(|a, b| b.time() - a.time() < 0.3e-6.seconds())
            .filter(|(count, _)| *count == 1)
            .map(|(_, event)| event)
            .filter(|event| match event.detector() {
                FermiDetectorType::Nai(_) => event.energy() >= 30 && event.energy() <= 124,
                FermiDetectorType::Bgo(_) => event.energy() >= 19 && event.energy() <= 126,
            })
            .collect();
        let gti = self.gti();

        let intervals: Result<Vec<([Time<Fermi>; 2], f64)>, Box<dyn Error>> = Ok(gti
            .iter()
            .flat_map(|interval| {
                search(
                    &events,
                    6,
                    interval[0],
                    interval[1],
                    SearchConfig {
                        ..Default::default()
                    },
                )
            })
            .collect());

        let ebounds = self.files[0].ebounds();
        let signals = intervals?
            .into_iter()
            .map(|(interval, fp_year)| {
                let start = interval[0];
                let stop = interval[1];
                let extend = 1.0.milliseconds();
                let start_extended = start - extend;
                let stop_extended = stop + extend;
                let events = self
                    .into_iter()
                    .filter(|event| event.time() >= start_extended && event.time() <= stop_extended)
                    .map(|event| event.to_general(&ebounds))
                    .collect::<Vec<_>>();
                let position = self.position.get_row(start);
                let ecef = ECEF::new(
                    position.pos[0] as f64,
                    position.pos[1] as f64,
                    position.pos[2] as f64,
                );
                let wgs84 = WGS84::from(ecef);
                let time_tolerance = Duration::microseconds(5);
                let distance_tolerance = 800_000.0;
                let lightnings = Lightning::associated_lightning(
                    (start + (stop - start) / 2.0).to_hifitime(),
                    wgs84.latitude_degrees(),
                    wgs84.longitude_degrees(),
                    time_tolerance,
                    distance_tolerance,
                );

                Signal {
                    start: start.to_hifitime(),
                    stop: stop.to_hifitime(),
                    fp_year,
                    events,
                    position: wgs84,
                    lightnings,
                }
            })
            .collect();

        Ok(signals)
    }
}

impl<'a> IntoIterator for &'a Hour {
    type Item = FermiEvent;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let mut file_iters = self
            .files
            .iter()
            .map(|file| file.into_iter())
            .collect::<Vec<_>>();
        let mut buffer = BinaryHeap::new();
        for (index, file_iter) in file_iters.iter_mut().enumerate() {
            if let Some(event) = file_iter.next() {
                buffer.push(Reverse((event, index)));
            }
        }
        Iter { file_iters, buffer }
    }
}

pub(crate) struct Iter<'a> {
    file_iters: Vec<file::Iter<'a>>,
    buffer: BinaryHeap<Reverse<(FermiEvent, usize)>>,
}

impl Iterator for Iter<'_> {
    type Item = FermiEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(Reverse((event, index))) = self.buffer.pop() {
            if let Some(next_event) = self.file_iters[index].next() {
                self.buffer.push(Reverse((next_event, index)));
            }
            Some(event)
        } else {
            None
        }
    }
}

fn get_file(folder: &OsStr, prefix: &str) -> Result<String, Box<dyn Error>> {
    let name = std::fs::read_dir(folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str().map(String::from))
        })
        .filter(|name| name.starts_with(prefix))
        .max_by(|a, b| {
            let extract_version = |name: &str| {
                name.split('_')
                    .last()
                    .and_then(|s| s.strip_prefix('v'))
                    .and_then(|s| s.get(..2))
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            };
            extract_version(a).cmp(&extract_version(b))
        })
        .ok_or_else(|| format!("No files found matching pattern for detector {}", prefix))?;
    Ok(Path::new(&folder)
        .join(name)
        .into_os_string()
        .into_string()
        .unwrap())
}
