mod crc_check_impl;
mod find_stime_impl;
mod rec_sci_data_impl;

use std::path::Path;

use chrono::prelude::*;
use chrono::TimeDelta;

use crate::env::HXMT_1B_DIR;

pub use rec_sci_data_impl::rec_sci_data;
pub use rec_sci_data_impl::SerialNum;

pub fn find_filename(type_: &str, time: DateTime<Utc>, serial_num: &str) -> String {
    let code = match (type_, serial_num) {
        ("eng", "A") => "0766",
        ("eng", "B") => "1009",
        ("eng", "C") => "1781",
        ("sci", "A") => "0642",
        ("sci", "B") => "0922",
        ("sci", "C") => "1686",
        _ => panic!("Invalid type or serial number"),
    };

    let mut path = String::new();
    let mut version = -1;

    // 创建前一天和后一天的时间
    let one_day_before = time - TimeDelta::days(1);
    let one_day_after = time + TimeDelta::days(1);

    // 遍历三个时间点
    for loop_time in [one_day_before, time, one_day_after].iter() {
        let folder_path = Path::new(HXMT_1B_DIR.as_str())
            .join(format!("{}{:02}", loop_time.year(), loop_time.month()))
            .join(format!("{:02}", loop_time.day()))
            .join(code);

        println!(
            "[DEBUG] Searching in folder: {}",
            folder_path.to_string_lossy()
        );
        if !folder_path.exists() {
            continue;
        }

        // 读取目录内容
        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                println!("[DEBUG] Checking entry: {}", entry_path.to_string_lossy());
                if entry_path.is_dir() {
                    let folder_name = entry_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    let prefix = format!(
                        "HXMT_1B_{}_{}{:02}{:02}T{:02}",
                        code,
                        loop_time.year(),
                        loop_time.month(),
                        loop_time.day(),
                        loop_time.hour()
                    );

                    println!("[DEBUG] Checking folder name: {}", folder_name);

                    if folder_name.len() >= 40
                        && folder_name.starts_with(&prefix)
                        && folder_name
                            .chars()
                            .nth(39)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                    {
                        let ver = folder_name
                            .chars()
                            .nth(39)
                            .and_then(|c| c.to_digit(10))
                            .map(|d| d as i32)
                            .unwrap_or(-1);

                        if ver > version {
                            version = ver;
                            path = entry_path
                                .join(format!("{}.fits", folder_name))
                                .to_string_lossy()
                                .into_owned();
                        }
                    }
                }
            }
        }
    }

    path
}

pub fn get_all_filenames(time: DateTime<Utc>) -> [[String; 3]; 2] {
    let types = ["eng", "sci"];
    let serial_nums = ["A", "B", "C"];

    [
        [
            find_filename(types[0], time, serial_nums[0]),
            find_filename(types[0], time, serial_nums[1]),
            find_filename(types[0], time, serial_nums[2]),
        ],
        [
            find_filename(types[1], time, serial_nums[0]),
            find_filename(types[1], time, serial_nums[1]),
            find_filename(types[1], time, serial_nums[2]),
        ],
    ]
}
