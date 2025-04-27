use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::{prelude::*, Duration, TimeDelta};
use itertools::Itertools;

use crate::{
    env::HXMT_1K_DIR,
    hxmt::{
        event::HxmtEvent,
        saturation::{get_all_filenames, rec_sci_data},
        Hxmt,
    },
    lightning::{associated_lightning, coincidence_prob},
    search::lightcurve::{light_curve, prefix_sum, search_light_curve, Trigger},
    types::{Event, Instance as InstanceTrait, Signal, Span, Time},
};

use super::{
    ec::HxmtEc,
    eng_file::EngFile,
    event_file::{EventFile, Iter},
    orbit_file::OrbitFile,
    sci_file::SciFile,
};

pub(crate) struct Instance {
    event_file: EventFile,
    eng_files: [EngFile; 3],
    sci_files: [SciFile; 3],
    pub(crate) orbit_file: OrbitFile,
    pub(crate) hxmt_ec: HxmtEc,
    span: [Time<Hxmt>; 2],
}

impl Instance {
    pub(crate) fn check_saturation(&self, time: Time<Hxmt>) -> bool {
        rec_sci_data(time, &self.eng_files[0], &self.sci_files[0])
            || rec_sci_data(time, &self.eng_files[1], &self.sci_files[1])
            || rec_sci_data(time, &self.eng_files[2], &self.sci_files[2])
    }
}

impl InstanceTrait for Instance {
    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self> {
        let num = (*epoch - Utc.with_ymd_and_hms(2017, 6, 15, 0, 0, 0).unwrap()).num_days() + 1;
        let folder = format!(
            "{HXMT_1K_DIR}/Y{year:04}{month:02}/{year:04}{month:02}{day:02}-{num:04}",
            HXMT_1K_DIR = HXMT_1K_DIR.as_str(),
            year = epoch.year(),
            month = epoch.month(),
            day = epoch.day(),
            num = num
        );
        let prefix = format!(
            "HXMT_{:04}{:02}{:02}T{:02}_HE-Evt_FFFFFF_V",
            epoch.year(),
            epoch.month(),
            epoch.day(),
            epoch.hour()
        );
        let event_file_path = get_file(&folder, &prefix)?;
        let event_file = EventFile::new(&event_file_path).with_context(|| {
            format!("Failed to create EventFile from file: {}", event_file_path)
        })?;
        let orbit_prefix = format!(
            "HXMT_{:04}{:02}{:02}T{:02}_Orbit_FFFFFF_V",
            epoch.year(),
            epoch.month(),
            epoch.day(),
            epoch.hour()
        );
        let orbit_file_path = get_file(&folder, &orbit_prefix)
            .with_context(|| format!("Failed to get orbit file: {}", orbit_prefix))?;
        let orbit_file = OrbitFile::new(&orbit_file_path).with_context(|| {
            format!("Failed to create OrbitFile from file: {}", orbit_file_path)
        })?;
        let [eng_files, sci_files] = get_all_filenames(*epoch)?;
        let eng_files = [
            EngFile::new(&eng_files[0])?,
            EngFile::new(&eng_files[1])?,
            EngFile::new(&eng_files[2])?,
        ];
        let sci_files = [
            SciFile::new(&sci_files[0])?,
            SciFile::new(&sci_files[1])?,
            SciFile::new(&sci_files[2])?,
        ];
        let hxmt_ec = HxmtEc::from_datetime(epoch)
            .with_context(|| format!("Failed to create HxmtEc from datetime: {}", epoch))?;
        Ok(Self {
            event_file,
            eng_files,
            sci_files,
            orbit_file,
            hxmt_ec,
            span: [
                Time::<Hxmt>::from(*epoch),
                Time::<Hxmt>::from(*epoch + TimeDelta::hours(1)),
            ],
        })
    }

    fn search(&self) -> Result<Vec<Signal>> {
        let events = self
            .into_iter()
            .filter(|event| !event.detector.am241)
            // .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
            // .filter(|(count, _)| *count == 1)
            // .map(|(_, event)| event)
            .filter(|event| event.energy() >= 38)
            // .filter(|event| !event.detector().acd.iter().any(|acd| *acd))
            .map(|event| event.time())
            .collect::<Vec<_>>();
        let fp_year = 20.0;
        let min_count = 8;
        let mut bin_size = Span::seconds(10e-6);
        let mut results = Vec::new();

        while bin_size < Span::seconds(1e-3) {
            results.extend((0..4).flat_map(|shift| {
                let shift = bin_size * shift as f64 / 4.0;
                let lc = light_curve(&events, self.span[0] + shift, self.span[1], bin_size);
                let prefix_sum = prefix_sum(&lc);
                search_light_curve(
                    &prefix_sum,
                    self.span[0] + shift,
                    bin_size,
                    fp_year,
                    min_count,
                )
            }));
            bin_size *= 2.0;
        }
        results.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        let results = results
            .into_iter()
            .coalesce(|prev, next| {
                if prev.mergeable(&next, 0.0) {
                    Ok(prev.merge(&next))
                } else {
                    Err((prev, next))
                }
            })
            .collect::<Vec<_>>();
        let results = continuous(results, Span::seconds(10.0), Span::seconds(1.0), 10);
        let results = results
            .into_iter()
            .filter(|trigger| !self.check_saturation(trigger.start))
            .collect::<Vec<_>>();
        let signals = results
            .into_iter()
            .filter_map(|trigger| {
                let start = trigger.start.to_chrono();
                let stop = trigger.stop.to_chrono();
                let middle =
                    start + trigger.delay.to_chrono() + trigger.bin_size_best.to_chrono() / 2;
                let extend = Span::milliseconds(1.0);
                let start_extended = trigger.start - extend;
                let stop_extended = trigger.stop + extend;
                let events_record = self
                    .into_iter()
                    .filter(|event| event.time() >= start_extended && event.time() <= stop_extended)
                    .map(|event| {
                        event.to_general(|event| {
                            // let k = self.hxmt_ec.rows[event.detector().id as usize].k;
                            // let b = self.hxmt_ec.rows[event.detector().id as usize].b;
                            // let k_err = self.hxmt_ec.rows[event.detector().id as usize].k_err;
                            // let b_err = self.hxmt_ec.rows[event.detector().id as usize].b_err;
                            // let energy_lower = (k - k_err) * event.energy() as f64 + (b - b_err);
                            // let energy_upper = (k + k_err) * event.energy() as f64 + (b + b_err);
                            // [energy_lower, energy_upper]
                            [event.energy() as f64, event.energy() as f64]
                        })
                    })
                    .collect::<Vec<_>>();
                if events_record.len() >= 100000 {
                    eprintln!(
                        "Too many events({}) in signal: {} - {}",
                        events_record.len(),
                        start,
                        stop
                    );
                    return None;
                }
                let (longitude, latitude, altitude) = self
                    .orbit_file
                    .interpolate(trigger.start.time.into_inner())
                    .unwrap_or((0.0, 0.0, 0.0));
                let time_tolerance = Duration::milliseconds(5);
                let distance_tolerance = 800_000.0;
                let lightning_window = TimeDelta::minutes(2);

                let fp_year = trigger.fp_year();
                let lightnings = associated_lightning(
                    middle,
                    latitude,
                    longitude,
                    altitude,
                    time_tolerance,
                    distance_tolerance,
                    lightning_window,
                );
                let associated_lightning_count = lightnings
                    .iter()
                    .filter(|lightning| lightning.is_associated)
                    .count() as u32;
                if true {
                    Some(Signal {
                        start,
                        stop,
                        best_start: start
                            + TimeDelta::nanoseconds(trigger.delay.to_nanoseconds() as i64),
                        best_stop: start
                            + TimeDelta::nanoseconds(
                                trigger.delay.to_nanoseconds() as i64
                                    + trigger.bin_size_best.to_nanoseconds() as i64,
                            ),
                        fp_year,
                        count: events
                            .iter()
                            .filter(|event| **event >= trigger.start && **event <= trigger.stop)
                            .count() as u32,
                        best_count: trigger.count,
                        background: trigger.average / trigger.bin_size_best.to_seconds(),
                        events: events_record,
                        light_curve: light_curve(
                            &self
                                .into_iter()
                                .map(|event| event.time())
                                .collect::<Vec<_>>(),
                            trigger.start - Span::seconds(0.5),
                            trigger.start + Span::seconds(0.5),
                            Span::seconds(1e-3),
                        ),
                        light_curve_filtered: light_curve(
                            &events,
                            trigger.start - Span::seconds(0.5),
                            trigger.start + Span::seconds(0.5),
                            Span::seconds(1e-3),
                        ),
                        longitude,
                        latitude,
                        altitude,
                        orbit: self.orbit_file.window(
                            trigger.start.time.into_inner(),
                            lightning_window.num_nanoseconds().unwrap() as f64 / 1e9,
                        ),
                        lightnings: associated_lightning(
                            middle,
                            latitude,
                            longitude,
                            altitude,
                            time_tolerance,
                            distance_tolerance,
                            lightning_window,
                        ),
                        associated_lightning_count,
                        coincidence_probability: coincidence_prob(
                            middle,
                            latitude,
                            longitude,
                            altitude,
                            time_tolerance,
                            distance_tolerance,
                            lightning_window,
                        ),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(signals)
    }
}

fn get_file(folder: &str, prefix: &str) -> Result<String> {
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
                name.strip_prefix(prefix)
                    .and_then(|s| s.get(..1))
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            };
            extract_version(a).cmp(&extract_version(b))
        })
        .ok_or_else(|| anyhow!("No files found matching pattern for detector {}", prefix))?;
    Ok(Path::new(&folder)
        .join(name)
        .into_os_string()
        .into_string()
        .unwrap())
}

impl<'a> IntoIterator for &'a Instance {
    type Item = HxmtEvent;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.event_file.into_iter()
    }
}

pub(crate) fn continuous(
    triggers: Vec<Trigger<Hxmt>>,
    interval: Span<Hxmt>,
    duration: Span<Hxmt>,
    count: i32,
) -> Vec<Trigger<Hxmt>> {
    if triggers.is_empty() {
        return triggers;
    }
    let mut veto = vec![false; triggers.len()];
    let mut last_time = triggers[0].start;
    let mut begin = 0;
    for i in 1..triggers.len() {
        let time = triggers[i].start;
        if (time - last_time) > interval || i == triggers.len() - 1 {
            if ((last_time - triggers[begin].start) > duration) || i - begin >= count as usize {
                veto[begin..i].fill(true);
            }
            begin = i;
        }
        last_time = time;
    }
    veto.into_iter()
        .zip(triggers)
        .filter(|(c, _)| !(*c))
        .map(|(_, t)| t)
        .collect()
}
