use std::path::Path;

use anyhow::{Context, Result, anyhow};
use chrono::{TimeDelta, prelude::*};
use itertools::Itertools;
use serde::de;
use statrs::statistics::Statistics;

use crate::{
    env::HXMT_1K_DIR,
    hxmt::{
        Hxmt,
        event::HxmtEvent,
        saturation::{get_all_filenames, rec_sci_data},
    },
    lightning::{associated_lightning, coincidence_prob},
    search::{
        algorithms::{SearchConfig, search_new},
        lightcurve::{light_curve, prefix_sum, search_light_curve},
        trigger::Trigger,
    },
    types::{Event, Instance as InstanceTrait, Signal, Span, Time},
};

use super::{
    att_file::AttFile,
    ec::HxmtEc,
    eng_file::EngFile,
    event_file::{EventFile, Iter},
    orbit_file::OrbitFile,
    sci_file::SciFile,
};

pub struct Instance {
    pub event_file: EventFile,
    eng_files: [EngFile; 3],
    sci_files: [SciFile; 3],
    pub orbit_file: OrbitFile,
    pub att_file: AttFile,
    pub hxmt_ec: HxmtEc,
    span: [Time<Hxmt>; 2],
}

impl Instance {
    pub fn check_saturation(&self, time: Time<Hxmt>) -> bool {
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
        let att_prefix = format!(
            "HXMT_{:04}{:02}{:02}T{:02}_Att_FFFFFF_V",
            epoch.year(),
            epoch.month(),
            epoch.day(),
            epoch.hour()
        );
        let att_file_path = get_file(&folder, &att_prefix)
            .with_context(|| format!("Failed to get attitude file: {}", att_prefix))?;
        let att_file = AttFile::new(&att_file_path)
            .with_context(|| format!("Failed to create AttFile from file: {}", att_file_path))?;
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
            att_file,
            hxmt_ec,
            span: [
                Time::<Hxmt>::from(*epoch),
                Time::<Hxmt>::from(*epoch + TimeDelta::hours(1)),
            ],
        })
    }

    fn search(&self) -> Result<Vec<Signal>> {
        const CHANNEL_THRESHOLD: u16 = 38;
        let events = self
            .into_iter()
            .filter(|event| !event.detector.am241)
            // .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
            // .filter(|(count, _)| *count == 1)
            // .map(|(_, event)| event)
            .filter(|event| event.energy() >= CHANNEL_THRESHOLD)
            // .filter(|event| !event.detector().acd.iter().any(|acd| *acd))
            // .map(|event| event.time())
            .collect::<Vec<_>>();
        let events_time = events.iter().map(|event| event.time()).collect::<Vec<_>>();

        println!("Number of events: {}", events.len());
        println!(
            "Time range: {} - {}",
            self.span[0].to_chrono(),
            self.span[1].to_chrono()
        );
        let results = search_new(
            &events,
            1,
            self.span[0],
            self.span[1],
            SearchConfig {
                max_duration: Span::milliseconds(1.0),
                neighbor: Span::seconds(1.0),
                hollow: Span::milliseconds(10.0),
                fp_year: 20.0,
                min_number: 8,
            },
        );
        println!("Number of triggers: {}", results.len());

        let results = continuous(results, Span::seconds(10.0), Span::seconds(1.0), 10);
        let results = results
            .into_iter()
            .filter(|trigger| !self.check_saturation(trigger.start))
            .collect::<Vec<_>>();
        let signals = results
            .into_iter()
            .filter_map(|trigger| {
                let extend = Span::milliseconds(1.0);
                let original_events_extended = self
                    .into_iter()
                    .filter(|event| {
                        event.time() >= trigger.start - extend
                            && event.time() <= trigger.stop + extend
                    })
                    .collect::<Vec<_>>();
                let filtered_events_extended = original_events_extended
                    .iter()
                    .filter(|event| event.energy() >= CHANNEL_THRESHOLD)
                    .collect::<Vec<_>>();
                if filtered_events_extended.len() >= 100000 {
                    eprintln!(
                        "Too many events({}) in signal: {} - {}",
                        filtered_events_extended.len(),
                        trigger.start.to_chrono(),
                        trigger.stop.to_chrono()
                    );
                    return None;
                }
                let (longitude, latitude, altitude) = self
                    .orbit_file
                    .interpolate(trigger.start.time.into_inner())
                    .unwrap_or((0.0, 0.0, 0.0));
                let (q1, q2, q3) = self
                    .att_file
                    .interpolate(trigger.start.time.into_inner())
                    .unwrap_or((0.0, 0.0, 0.0));
                let time_tolerance = TimeDelta::milliseconds(5);
                let distance_tolerance = 800_000.0;
                let lightning_window = TimeDelta::minutes(2);
                let lightnings = associated_lightning(
                    (trigger.start + trigger.delay + trigger.bin_size_best / 2.0).to_chrono(),
                    latitude,
                    longitude,
                    altitude,
                    time_tolerance,
                    distance_tolerance,
                    lightning_window,
                );
                fn to_general(event: &HxmtEvent) -> [f64; 2] {
                    [event.energy() as f64, event.energy() as f64]
                }
                let original_events = original_events_extended
                    .iter()
                    .filter(|event| event.time() >= trigger.start && event.time() <= trigger.stop)
                    .collect::<Vec<_>>();
                let original_events_best = original_events
                    .iter()
                    .filter(|event| {
                        event.time() >= trigger.start + trigger.delay
                            && event.time() <= trigger.start + trigger.delay + trigger.bin_size_best
                    })
                    .collect::<Vec<_>>();
                let filtered_events = filtered_events_extended
                    .iter()
                    .filter(|event| event.time() >= trigger.start && event.time() <= trigger.stop)
                    .collect::<Vec<_>>();
                let filtered_events_best = filtered_events
                    .iter()
                    .filter(|event| {
                        event.time() >= trigger.start + trigger.delay
                            && event.time() <= trigger.start + trigger.delay + trigger.bin_size_best
                    })
                    .collect::<Vec<_>>();
                let count = original_events.len() as u32;
                let count_best = original_events_best.len() as u32;
                let count_filtered = filtered_events.len() as u32;
                let count_filtered_best = filtered_events_best.len() as u32;
                if true {
                    Some(Signal::new(
                        trigger.start.to_chrono(),
                        (trigger.start + trigger.delay).to_chrono(),
                        trigger.stop.to_chrono(),
                        (trigger.start + trigger.delay + trigger.bin_size_best).to_chrono(),
                        trigger.fp_year(),
                        count,
                        count_best,
                        count_filtered,
                        count_filtered_best,
                        trigger.mean / trigger.bin_size_best.to_seconds(),
                        original_events
                            .iter()
                            .map(|event| event.to_general(to_general).energy[0])
                            .mean(),
                        original_events_best
                            .iter()
                            .map(|event| event.to_general(to_general).energy[0])
                            .mean(),
                        filtered_events
                            .iter()
                            .map(|event| event.to_general(to_general).energy[0])
                            .mean(),
                        filtered_events_best
                            .iter()
                            .map(|event| event.to_general(to_general).energy[0])
                            .mean(),
                        original_events
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / original_events.len() as f64,
                        original_events_best
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / original_events_best.len() as f64,
                        filtered_events
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / filtered_events.len() as f64,
                        filtered_events_best
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / filtered_events_best.len() as f64,
                        original_events_extended
                            .iter()
                            .map(|event| event.to_general(to_general))
                            .collect(),
                        light_curve(
                            &self
                                .into_iter()
                                .map(|event| event.time())
                                .collect::<Vec<_>>(),
                            trigger.start - Span::milliseconds(500.0),
                            trigger.start + Span::milliseconds(500.0),
                            Span::milliseconds(10.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        light_curve(
                            &events_time,
                            trigger.start - Span::milliseconds(500.0),
                            trigger.start + Span::milliseconds(500.0),
                            Span::milliseconds(10.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        light_curve(
                            &self
                                .into_iter()
                                .map(|event| event.time())
                                .collect::<Vec<_>>(),
                            trigger.start - Span::milliseconds(50.0),
                            trigger.start + Span::milliseconds(50.0),
                            Span::milliseconds(1.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        light_curve(
                            &events_time,
                            trigger.start - Span::milliseconds(50.0),
                            trigger.start + Span::milliseconds(50.0),
                            Span::milliseconds(1.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        longitude,
                        latitude,
                        altitude,
                        q1,
                        q2,
                        q3,
                        self.orbit_file
                            .window(trigger.start.time.into_inner(), 1000.0),
                        lightnings,
                        coincidence_prob(
                            (trigger.start + trigger.delay + trigger.bin_size_best / 2.0)
                                .to_chrono(),
                            latitude,
                            longitude,
                            altitude,
                            time_tolerance,
                            distance_tolerance,
                            lightning_window,
                        ),
                    ))
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

pub fn continuous(
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
