use std::path::Path;

use super::{
    super::{
        data::{
            data_1b::{EngFile, SciFile, get_all_filenames},
            data_1k::{AttFile, EventFile, OrbitFile},
        },
        types::Hxmt,
    },
    define::Instance,
};
use crate::{env::HXMT_1K_DIR, types::Time};
use anyhow::{Context, Result, anyhow};
use chrono::{TimeDelta, prelude::*};

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

pub(super) fn instance_from_epoch(epoch: &DateTime<Utc>) -> Result<Instance> {
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
    let event_file = EventFile::new(&event_file_path)
        .with_context(|| format!("Failed to create EventFile from file: {}", event_file_path))?;
    let orbit_prefix = format!(
        "HXMT_{:04}{:02}{:02}T{:02}_Orbit_FFFFFF_V",
        epoch.year(),
        epoch.month(),
        epoch.day(),
        epoch.hour()
    );
    let orbit_file_path = get_file(&folder, &orbit_prefix)
        .with_context(|| format!("Failed to get orbit file: {}", orbit_prefix))?;
    let orbit_file = OrbitFile::new(&orbit_file_path)
        .with_context(|| format!("Failed to create OrbitFile from file: {}", orbit_file_path))?;
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
    Ok(Instance {
        event_file,
        eng_files,
        sci_files,
        orbit_file,
        att_file,
        span: [
            Time::<Hxmt>::from(*epoch),
            Time::<Hxmt>::from(*epoch + TimeDelta::hours(1)),
        ],
    })
}
