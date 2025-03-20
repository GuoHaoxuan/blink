use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::{prelude::*, TimeDelta};

use crate::{hxmt::Hxmt, types::Time};

use super::event_file::EventFile;

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
        .inspect(|name| {
            println!("Found file: {}", name);
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
