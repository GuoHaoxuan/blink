use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::error::Error;

use itertools::Itertools;
use regex::Regex;

use crate::search::algorithms::{search, SearchConfig};
use crate::types::{Epoch, Event as _, Interval, Signal, TimeUnits};

use super::detector::Detector;
use super::event::Event;
use super::file::{self, File};
use super::Fermi;

pub(crate) struct Hour {
    files: Vec<File>,
}

impl Hour {
    pub(crate) fn new(data: &[(&str, Detector)]) -> Result<Self, fitsio::errors::Error> {
        let files = data
            .iter()
            .map(|(filename, detector)| File::new(filename, *detector))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { files })
    }

    pub(crate) fn from_epoch(epoch: &hifitime::Epoch) -> Result<Self, Box<dyn Error>> {
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
            let detector = if i < 12 {
                Detector::Nai(i)
            } else {
                Detector::Bgo(i - 12)
            };
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
                filenames.push((format!("{}/{}", folder, file), detector));
            }
        }
        if filenames.is_empty() {
            return Err("No matching files found".into());
        }
        Self::new(
            &filenames
                .iter()
                .map(|(s, d)| (s.as_str(), *d))
                .collect::<Vec<_>>(),
        )
        .map_err(|e| e.into())
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

    pub(crate) fn search(&self) -> Result<Vec<Signal<Fermi>>, Box<dyn Error>> {
        let events: Vec<Event> = self
            .into_iter()
            .dedup_by_with_count(|a, b| b.time() - a.time() < 0.3e-6.seconds())
            .filter(|(count, _)| *count == 1)
            .map(|(_, event)| event)
            .filter(|event| match event.detector() {
                Detector::Nai(_) => event.pha() >= 30 && event.pha() <= 124,
                Detector::Bgo(_) => event.pha() >= 19 && event.pha() <= 126,
            })
            .collect();
        let gti = self.gti();

        let intervals: Result<Vec<Interval<usize>>, Box<dyn Error>> = Ok(gti
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
            .collect());

        let signals = intervals?
            .into_iter()
            .map(|interval| {
                let start = events[interval.start].time();
                let stop = events[interval.stop].time();
                let extend_time = 1.0.milliseconds();
                let start_before = start - extend_time;
                let stop_after = stop + extend_time;
                let start_index = events
                    .binary_search_by(|event| event.time().cmp(&start_before))
                    .unwrap_or_else(|index| index)
                    .min(interval.start);
                let stop_index = events
                    .binary_search_by(|event| event.time().cmp(&stop_after))
                    .unwrap_or_else(|index| index - 1)
                    .max(interval.stop);
                Signal {
                    start,
                    stop,
                    events: events[start_index..=stop_index].to_vec(),
                }
            })
            .collect();

        Ok(signals)
    }
}

impl<'a> IntoIterator for &'a Hour {
    type Item = Event;
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
    buffer: BinaryHeap<Reverse<(Event, usize)>>,
}

impl Iterator for Iter<'_> {
    type Item = Event;

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
