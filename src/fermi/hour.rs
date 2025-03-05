use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::env;
use std::error::Error;
use std::path::Path;
use std::sync::LazyLock;

use itertools::Itertools;
use regex::Regex;

use crate::lightning::Lightning;
use crate::search::algorithms::{search, SearchConfig};
use crate::types::{Epoch, Event as _, Interval, Signal, TimeUnits};

use super::detector::Detector;
use super::event::EventInterval;
use super::event::EventPha;
use super::file::{self, File};
use super::position::PositionRow;
use super::{Fermi, Position};

pub(crate) struct Hour {
    files: Vec<File>,
    position: Position,
}

impl Hour {
    pub(crate) fn new(data: &[&str], position: &str) -> Result<Self, fitsio::errors::Error> {
        let detectors = (0..14)
            .map(|i| {
                if i < 12 {
                    Detector::Nai(i)
                } else {
                    Detector::Bgo(i - 12)
                }
            })
            .collect::<Vec<_>>();
        let files = data
            .iter()
            .zip(detectors.iter())
            .map(|(filename, detector)| File::new(filename, *detector))
            .collect::<Result<Vec<_>, _>>()?;
        let position = Position::new(position)?;
        Ok(Self { files, position })
    }

    pub(crate) fn from_epoch(epoch: &hifitime::Epoch) -> Result<Self, Box<dyn Error>> {
        static GBM_DAILY_PATH: LazyLock<String> = LazyLock::new(|| {
            env::var("GBM_DAILY_PATH").unwrap_or_else(|_| {
                "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily".to_string()
            })
        });
        let (y, m, d, h, ..) = epoch.to_gregorian_utc();
        let folder = Path::new(&*GBM_DAILY_PATH)
            .join(format!("{:04}/{:02}/{:02}/current", y, m, d))
            .into_os_string();
        let filenames: Vec<String> = (0..14)
            .map(|i| -> Result<String, Box<dyn Error>> {
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
                    })
                    .ok_or_else(|| format!("No files found matching pattern for detector {}", i))?;
                Ok(Path::new(&folder)
                    .join(max_file)
                    .into_os_string()
                    .into_string()
                    .unwrap())
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
        let position_pattern = format!(
            "glg_poshist_all_{:02}{:02}{:02}_v\\d{{2}}\\.fit",
            y % 100,
            m,
            d
        );
        let position_max_file = std::fs::read_dir(&folder)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str().map(String::from))
            })
            .filter(|name| Regex::new(&position_pattern).unwrap().is_match(name))
            .max_by(|a, b| {
                let extract_version = |name: &str| {
                    name.split('_')
                        .last()
                        .and_then(|s| s.strip_prefix('v'))
                        .and_then(|s| s.strip_suffix(".fit"))
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0)
                };
                extract_version(a).cmp(&extract_version(b))
            })
            .ok_or_else(|| {
                format!(
                    "No position file found for {} and dir {}",
                    position_pattern,
                    folder.clone().into_string().unwrap()
                )
            })?;
        let position_filename = Path::new(&folder)
            .join(position_max_file)
            .into_os_string()
            .into_string()
            .unwrap();
        Ok(Self::new(
            &filenames.iter().map(|f| f.as_str()).collect::<Vec<_>>(),
            &position_filename,
        )?)
    }

    pub(crate) fn gti(&self) -> Vec<Interval<Epoch<Fermi>>> {
        self.files
            .iter()
            .map(|file| file.gti())
            .reduce(|a, b| {
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
            })
            .unwrap()
    }

    pub(crate) fn search(&self) -> Result<Vec<Signal<EventInterval, PositionRow>>, Box<dyn Error>> {
        let events: Vec<EventPha> = self
            .into_iter()
            .dedup_by_with_count(|a, b| b.time() - a.time() < 0.3e-6.seconds())
            .filter(|(count, _)| *count == 1)
            .map(|(_, event)| event)
            .filter(|event| match event.detector() {
                Detector::Nai(_) => event.energy >= 30 && event.energy <= 124,
                Detector::Bgo(_) => event.energy >= 19 && event.energy <= 126,
            })
            .collect();
        let gti = self.gti();

        let intervals: Result<Vec<(Interval<Epoch<Fermi>>, f64)>, Box<dyn Error>> = Ok(gti
            .iter()
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
            .collect());

        let ebounds_min = &self.files[0].ebounds_e_min;
        let ebounds_max = &self.files[0].ebounds_e_max;
        let signals = intervals?
            .into_iter()
            .map(|(interval, fp_year)| {
                let start = interval.start;
                let stop = interval.stop;
                let extend = 1.0.milliseconds();
                let start_extended = start - extend;
                let stop_extended = stop + extend;
                let events = self
                    .into_iter()
                    .filter(|event| event.time() >= start_extended && event.time() <= stop_extended)
                    .map(|event| event.to_interval(ebounds_min, ebounds_max))
                    .collect::<Vec<_>>();
                let position = self.position.get_row(start);
                let lightnings = match &position {
                    Some(pos) => {
                        let lat = pos.sc_lat;
                        let lon = pos.sc_lon;

                        let time_tolerance = hifitime::Duration::from_microseconds(5.0);
                        let distance_tolerance = 800_000.0;

                        Some(Lightning::associated_lightning(
                            (start + (stop - start) / 2.0).to_hifitime(),
                            lat as f64,
                            lon as f64,
                            time_tolerance,
                            distance_tolerance,
                        ))
                    }
                    None => None,
                };

                Signal {
                    start,
                    stop,
                    fp_year,
                    events,
                    position,
                    lightnings,
                }
            })
            .collect();

        Ok(signals)
    }
}

impl<'a> IntoIterator for &'a Hour {
    type Item = EventPha;
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
    buffer: BinaryHeap<Reverse<(EventPha, usize)>>,
}

impl Iterator for Iter<'_> {
    type Item = EventPha;

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
