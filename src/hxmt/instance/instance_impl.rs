use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::{prelude::*, TimeDelta};
use itertools::Itertools;

use crate::{
    hxmt::{event::HxmtEvent, Hxmt},
    search::lightcurve::{light_curve, prefix_sum, search_light_curve, Trigger},
    types::{Event, Signal, Span, Time},
};

use super::event_file::{EventFile, Iter};

pub(crate) struct Instance {
    event_file: EventFile,
    span: [Time<Hxmt>; 2],
}

impl Instance {
    pub(crate) fn new(event_file_path: &str, span: [Time<Hxmt>; 2]) -> Result<Self> {
        let event_file = EventFile::new(event_file_path)?;
        Ok(Self { event_file, span })
    }

    pub(crate) fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self> {
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
        Ok(Self {
            event_file,
            span: [
                Time::<Hxmt>::from(*epoch),
                Time::<Hxmt>::from(*epoch + TimeDelta::hours(1)),
            ],
        })
    }

    pub(crate) fn search(&self) -> Result<Vec<Trigger<Hxmt>>> {
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
        Ok(results)
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
