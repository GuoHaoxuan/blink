use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::{prelude::*, TimeDelta};
use itertools::Itertools;

use crate::{
    hxmt::{
        event::HxmtEvent,
        saturation::{get_all_filenames, rec_sci_data, SerialNum},
        Hxmt,
    },
    search::lightcurve::{light_curve, prefix_sum, search_light_curve, Trigger},
    types::{Event, Instance as InstanceTrait, Signal, Span, Time},
};

use super::{
    eng_file::EngFile,
    event_file::{EventFile, Iter},
    sci_file::SciFile,
};

pub(crate) struct Instance {
    event_file: EventFile,
    eng_files: [EngFile; 3],
    sci_files: [SciFile; 3],
    span: [Time<Hxmt>; 2],
}

impl Instance {
    pub(crate) fn check_saturation(&self, time: Time<Hxmt>) -> bool {
        rec_sci_data(time, SerialNum::A, &self.eng_files[0], &self.sci_files[0])
            || rec_sci_data(time, SerialNum::B, &self.eng_files[1], &self.sci_files[1])
            || rec_sci_data(time, SerialNum::C, &self.eng_files[2], &self.sci_files[2])
    }
}

impl InstanceTrait for Instance {
    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self> {
        let num = (*epoch - Utc.with_ymd_and_hms(2017, 6, 15, 0, 0, 0).unwrap()).num_days() + 1;
        let folder = format!(
            "/hxmt/work/HXMT-DATA/1K/Y{year:04}{month:02}/{year:04}{month:02}{day:02}-{num:04}",
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
        let event_file = EventFile::new(&event_file_path)?;
        let [eng_files, sci_files] = get_all_filenames(*epoch);
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
        Ok(Self {
            event_file,
            eng_files,
            sci_files,
            span: [
                Time::<Hxmt>::from(*epoch),
                Time::<Hxmt>::from(*epoch + TimeDelta::hours(1)),
            ],
        })
    }

    fn search(&self) -> Result<Vec<Signal>> {
        let events = self
            .into_iter()
            .filter(|event| event.energy() >= 38)
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
                    100,
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
                if prev.mergeable(&next, 0) {
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
            .map(|trigger| Signal {
                start: trigger.start.to_chrono(),
                stop: trigger.stop.to_chrono(),
                fp_year: trigger.fp_year,
                events: vec![],                 // TODO
                longitude: 0.0,                 // TODO
                latitude: 0.0,                  // TODO
                altitude: 0.0,                  // TODO
                position_debug: "".to_string(), // TODO
                lightnings: vec![],             // TODO
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
