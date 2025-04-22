mod crc_check_impl;
mod find_stime_impl;
mod rec_sci_data_impl;

use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::prelude::*;

use crate::env::HXMT_1B_DIR;

pub use rec_sci_data_impl::rec_sci_data;

pub fn find_filename(type_: &str, time: DateTime<Utc>, serial_num: &str) -> Option<String> {
    let code = match (type_, serial_num) {
        ("eng", "A") => "0766",
        ("eng", "B") => "1009",
        ("eng", "C") => "1781",
        ("sci", "A") => "0642",
        ("sci", "B") => "0922",
        ("sci", "C") => "1686",
        _ => panic!("Invalid type or serial number"),
    };

    let mut path = None;
    let mut version = -1;

    let folder_path = Path::new(HXMT_1B_DIR.as_str())
        .join(format!("{}", time.year()))
        .join(format!(
            "{}{:02}{:02}",
            time.year(),
            time.month(),
            time.day()
        ))
        .join(code);

    if !folder_path.exists() {
        return None;
    }

    let prefix = format!(
        "HXMT_1B_{}_{}{:02}{:02}T{:02}",
        code,
        time.year(),
        time.month(),
        time.day(),
        time.hour()
    );

    // 读取目录内容
    if let Ok(entries) = std::fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let filename = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if filename.len() >= 40
                    && filename.starts_with(&prefix)
                    && filename
                        .chars()
                        .nth(39)
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                {
                    let ver = filename
                        .chars()
                        .nth(39)
                        .and_then(|c| c.to_digit(10))
                        .map(|d| d as i32)
                        .unwrap_or(-1);

                    if ver > version {
                        version = ver;
                        path = Some(entry_path.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    path
}

pub fn get_all_filenames(time: DateTime<Utc>) -> Result<[[String; 3]; 2]> {
    let types = ["eng", "sci"];
    let serial_nums = ["A", "B", "C"];

    Ok([
        [
            find_filename(types[0], time, serial_nums[0]).ok_or_else(|| {
                anyhow!(
                    "Failed to find eng file for {} with serial {}",
                    time.format("%Y-%m-%d %H:%M:%S"),
                    serial_nums[0]
                )
            })?,
            find_filename(types[0], time, serial_nums[1]).ok_or_else(|| {
                anyhow!(
                    "Failed to find eng file for {} with serial {}",
                    time.format("%Y-%m-%d %H:%M:%S"),
                    serial_nums[1]
                )
            })?,
            find_filename(types[0], time, serial_nums[2]).ok_or_else(|| {
                anyhow!(
                    "Failed to find eng file for {} with serial {}",
                    time.format("%Y-%m-%d %H:%M:%S"),
                    serial_nums[2]
                )
            })?,
        ],
        [
            find_filename(types[1], time, serial_nums[0]).ok_or_else(|| {
                anyhow!(
                    "Failed to find sci file for {} with serial {}",
                    time.format("%Y-%m-%d %H:%M:%S"),
                    serial_nums[0]
                )
            })?,
            find_filename(types[1], time, serial_nums[1]).ok_or_else(|| {
                anyhow!(
                    "Failed to find sci file for {} with serial {}",
                    time.format("%Y-%m-%d %H:%M:%S"),
                    serial_nums[1]
                )
            })?,
            find_filename(types[1], time, serial_nums[2]).ok_or_else(|| {
                anyhow!(
                    "Failed to find sci file for {} with serial {}",
                    time.format("%Y-%m-%d %H:%M:%S"),
                    serial_nums[2]
                )
            })?,
        ],
    ])
}
