use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ffi::OsStr;
use std::path::Path;

use chrono::{Duration, prelude::*};
use file::File;
use itertools::Itertools;
use position::Position;

use crate::env::GBM_DAILY_PATH;
use crate::lightning::{associated_lightning, coincidence_prob};
use crate::search::algorithms::{SearchConfig, search};
use crate::types::{Event as _, Instance as InstanceTrait, Signal, Span, Time};

use super::Fermi;
use super::detector::FermiDetectorType;
use super::event::FermiEvent;

mod file;
pub mod position;

pub struct Instance {
    files: Vec<File>,
    position: Position,
    span: [Time<Fermi>; 2],
}

impl Instance {
    pub fn new(
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

    pub fn gti(&self) -> Vec<[Time<Fermi>; 2]> {
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
}

impl InstanceTrait for Instance {
    fn from_epoch(epoch: &DateTime<Utc>) -> anyhow::Result<Self> {
        let y = epoch.year();
        let m = epoch.month();
        let d = epoch.day();
        let h = epoch.hour();
        let folder = Path::new(&*GBM_DAILY_PATH)
            .join(format!("{:04}/{:02}/{:02}/current", y, m, d))
            .into_os_string();
        let filenames: Vec<String> = (0..14)
            .map(|i| -> anyhow::Result<String> {
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
            .collect::<anyhow::Result<Vec<_>>>()?;
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

    fn search(&self) -> anyhow::Result<Vec<Signal>> {
        let events: Vec<FermiEvent> = self
            .into_iter()
            .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
            .filter(|(count, _)| *count == 1)
            .map(|(_, event)| event)
            .filter(|event| match event.detector() {
                FermiDetectorType::Nai(_) => event.energy() >= 30 && event.energy() <= 124,
                FermiDetectorType::Bgo(_) => event.energy() >= 19 && event.energy() <= 126,
            })
            .collect();
        let gti = self.gti();

        let intervals: anyhow::Result<Vec<([Time<Fermi>; 2], f64)>> = Ok(gti
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
                let extend = Span::milliseconds(1.0);
                let start_extended = start - extend;
                let stop_extended = stop + extend;
                let events = self
                    .into_iter()
                    .filter(|event| event.time() >= start_extended && event.time() <= stop_extended)
                    .map(|event| {
                        event.to_general(|event| {
                            [
                                ebounds[event.energy() as usize][0],
                                ebounds[event.energy() as usize][1],
                            ]
                        })
                    })
                    .collect::<Vec<_>>();
                let position = self.position.get_row(start);
                let time_tolerance = Duration::milliseconds(5);
                let distance_tolerance = 800_000.0;
                let lightnings = associated_lightning(
                    (start + (stop - start) / 2.0).to_chrono(),
                    position.sc_lat as f64,
                    position.sc_lon as f64,
                    altitude(&position.pos) as f64,
                    time_tolerance,
                    distance_tolerance,
                    Duration::minutes(2),
                );

                Signal::new(
                    start.to_chrono(),
                    start.to_chrono(),
                    stop.to_chrono(),
                    stop.to_chrono(),
                    fp_year,
                    0,
                    0,
                    0,
                    0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    events,
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    position.sc_lon as f64,
                    position.sc_lat as f64,
                    altitude(&position.pos) as f64,
                    0.0,
                    0.0,
                    0.0,
                    vec![],
                    vec![],
                    0.0,
                )
            })
            .collect();

        Ok(signals)
    }
}

impl<'a> IntoIterator for &'a Instance {
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

pub struct Iter<'a> {
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

fn get_file(folder: &OsStr, prefix: &str) -> anyhow::Result<String> {
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
                    .next_back()
                    .and_then(|s| s.strip_prefix('v'))
                    .and_then(|s| s.get(..2))
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            };
            extract_version(a).cmp(&extract_version(b))
        })
        .ok_or_else(|| {
            anyhow::anyhow!("No files found matching pattern for detector {}", prefix)
        })?;
    Ok(Path::new(&folder)
        .join(name)
        .into_os_string()
        .into_string()
        .unwrap())
}

fn altitude(coord: &[f32]) -> f32 {
    // Parameters of the World Geodetic System 1984
    // semi-major axis
    let wgs84_a = 6378137.0; // meters
    // reciprocal of flattening
    let wgs84_1overf = 298.257_23;

    let rho = (coord[0].powi(2) + coord[1].powi(2)).sqrt();
    let f: f32 = 1.0 / wgs84_1overf;
    let e_sq = 2.0 * f - f.powi(2);

    // should completely converge in 3 iterations
    let n_iter = 3;
    let mut kappa = vec![0.0; n_iter + 1];
    kappa[0] = 1.0 / (1.0 - e_sq);

    for i in 1..=n_iter {
        let c = (rho.powi(2) + (1.0 - e_sq) * coord[2].powi(2) * kappa[i - 1].powi(2)).powf(1.5)
            / (wgs84_a * e_sq);
        kappa[i] = (c + (1.0 - e_sq) * coord[2].powi(2) * kappa[i - 1].powi(3)) / (c - rho.powi(2));
    }

    (1.0 / kappa[n_iter] - 1.0 / kappa[0])
        * (rho.powi(2) + coord[2].powi(2) * kappa[n_iter].powi(2)).sqrt()
        / e_sq
}
