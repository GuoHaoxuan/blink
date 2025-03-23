// 默认 的1B级数据文件都存在当前目录下，使用程序前需要针对性修改
// 工程数据文件A/B/C: 0766/1009/1781
// 物理数据文件A/B/C: 0642/0922/1686

use std::path::Path;

use chrono::{prelude::*, TimeDelta};

use crate::env::HXMT_1B_DIR;

pub fn crc_check(data: &[u8]) -> u8 {
    // 对输入数据进行CRC校验
    // data是8个字节的evt package，生成4bit的CRC code
    let crc_table: [u8; 16] = [0, 3, 6, 5, 12, 15, 10, 9, 11, 8, 13, 14, 7, 4, 1, 2];
    let mut crct: u8 = 0;
    let mut cpdata: u8;
    let mut cdata: u8 = 0;
    for j in 0..(data.len() * 2 - 1) {
        if j % 2 == 0 {
            cdata = data[j / 2];
            cpdata = (cdata & 0b11110000) >> 4;
        } else {
            cpdata = cdata & 15;
        }
        crct = crc_table[(crct ^ cpdata) as usize];
    }
    crct
}

pub fn find_stime_calc(
    bus_time_code: &[&[u64]],
    stime: &[i32],
    time_stamp: u64,
) -> (i32, u64, i32) {
    // 计算bus_time
    let mut bus_time = vec![0u64; bus_time_code.len()];
    for (i, code) in bus_time_code.iter().enumerate() {
        bus_time[i] = ((code[5] & 127) << 24) + (code[4] << 16) + (code[3] << 8) + code[2];
    }

    let mut stime_type = -1;

    // 检查时间戳是否在bus_time中
    if bus_time.contains(&time_stamp) {
        stime_type = 0; // 代表GPS正常状态
    } else {
        // 处理GPS失锁状态
        for i in 0..bus_time.len() - 1 {
            if bus_time[i + 1] == bus_time[i] {
                bus_time[i + 1] = bus_time[i] + 1;
            }
        }
        stime_type = 1; // 代表GPS失锁状态
    }

    // 查找匹配的时间戳索引
    let mut index = Vec::new();
    for (i, &time) in bus_time.iter().enumerate() {
        if time == time_stamp {
            index.push(i);
        }
    }

    // 如果没有找到匹配的时间戳
    if index.is_empty() {
        return (-1, 0, -1);
    }

    let stime_a = stime[index[0]];

    (stime_a, time_stamp, stime_type)
}

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

        if !folder_path.exists() {
            continue;
        }

        // 读取目录内容
        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
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

pub fn get_all_filenames(time: DateTime<Utc>) -> Vec<String> {
    let mut all_files = Vec::new();
    let types = ["eng", "sci"];
    let serial_nums = ["A", "B", "C"];

    for &type_ in &types {
        for &serial_num in &serial_nums {
            let filename = find_filename(type_, time, serial_num);
            if !filename.is_empty() {
                all_files.push(filename);
            }
        }
    }

    all_files
}
