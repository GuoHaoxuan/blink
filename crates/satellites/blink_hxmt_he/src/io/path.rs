use blink_core::error::Error;
use chrono::prelude::*;
use std::{env, path::Path, sync::LazyLock};

static HXMT_1K_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var("HXMT_1K_DIR").unwrap_or_else(|_| "/hxmt/work/HXMT-DATA/1K".to_string())
});

fn get_file(folder: &str, prefix: &str) -> Result<String, Error> {
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
        .ok_or_else(|| {
            Error::FileNotFound(format!(
                "No files found matching pattern for detector {}",
                prefix
            ))
        })?;
    Ok(Path::new(&folder)
        .join(name)
        .into_os_string()
        .into_string()
        .unwrap())
}

pub fn get_path(epoch: &DateTime<Utc>, file_type: &str) -> Result<String, Error> {
    if file_type != "Evt" && file_type != "Orbit" && file_type != "Att" {
        panic!("Invalid file type: {}", file_type);
    }

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
        "HXMT_{:04}{:02}{:02}T{:02}_HE-{}_FFFFFF_V",
        epoch.year(),
        epoch.month(),
        epoch.day(),
        epoch.hour(),
        file_type
    );
    get_file(&folder, &prefix)
}
