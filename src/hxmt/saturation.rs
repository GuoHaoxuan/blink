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

pub fn rec_sci_data_calc(
    stime_a: u64,
    evt_utc_stamp: f64,
    serial_num: &str,
    evt_data: &[Vec<u64>],
) -> String {
    let evt_range = 2.0; // 选取evt发生点前后多长时间的数据,evtRange/2，单位是秒
    let evt_utc_floor = evt_utc_stamp - evt_range / 2.0;
    let evt_utc_ceil = evt_utc_stamp + evt_range / 2.0;

    // 处理A单元的数据
    let mut pack_utc = Vec::with_capacity(evt_data.len());
    let mut pack_interval_index = Vec::with_capacity(evt_data.len());

    // 计算pack_utc并标记在时间间隔内的条目
    for row in evt_data.iter() {
        let utc = row[878] + (row[879] << 8) + (row[880] << 16) + (row[881] << 24);
        pack_utc.push(utc);
        if utc >= (evt_utc_floor as u64 - 1) && utc <= (evt_utc_ceil as u64 + 1) {
            pack_interval_index.push(true);
        } else {
            pack_interval_index.push(false);
        }
    }

    // 提取在时间间隔内的数据包
    let mut pack_data = Vec::new();
    for (i, in_range) in pack_interval_index.iter().enumerate() {
        if *in_range {
            pack_data.push(evt_data[i][6..878].to_vec());
        }
    }

    let pack_size = pack_data.len();
    let mut burst_found = 0;
    let mut start_sec = vec![0; 4];
    let mut stop_sec = vec![0; 4];

    for i in 0..pack_size {
        // 将数据行重塑为109×8的形状
        let mut row_data = Vec::with_capacity(109);
        for j in 0..109 {
            let start_idx = j * 8;
            let end_idx = start_idx + 8;
            if start_idx < pack_data[i].len() && end_idx <= pack_data[i].len() {
                row_data.push(pack_data[i][start_idx..end_idx].to_vec());
            }
        }

        for j in 0..row_data.len() {
            if row_data[j].len() == 8 && (row_data[j][7] & 0b00110000) == 16 {
                // 代表GPS事例
                let crc_out = row_data[j][7] & 0x0F; // CRC校验正确，&0x0F取出后四位
                let stime = (row_data[j][0] << 24)
                    + (row_data[j][1] << 16)
                    + (row_data[j][2] << 8)
                    + row_data[j][3];
                let ptime = ((row_data[j][4] & 1) << 18)
                    + (row_data[j][5] << 10)
                    + (row_data[j][6] << 2)
                    + ((row_data[j][7] & 0b11000000) >> 6);

                if stime == stime_a - (evt_range / 2.0) as u64 {
                    start_sec = vec![stime, ptime, i as u64, j as u64];
                    burst_found |= 1;
                } else if stime == stime_a + (evt_range / 2.0) as u64 {
                    burst_found |= 2;
                    stop_sec = vec![stime, ptime, i as u64, j as u64];
                    break;
                }
            }
        }

        if burst_found == 3 {
            break;
        }
    }

    if burst_found != 3 {
        return "Saturation".to_string();
    }

    let mut evt_index: i64 = -1;
    let mut evt_list = Vec::new();
    let mut gps_list = Vec::new();
    let mut crc_err_list = Vec::new();

    for i in start_sec[2]..=stop_sec[2] {
        // 将数据行重塑为109×8的形状
        let mut row_data = Vec::with_capacity(109);
        for j in 0..109 {
            let start_idx = j * 8;
            let end_idx = start_idx + 8;
            if start_idx < pack_data[i as usize].len() && end_idx <= pack_data[i as usize].len() {
                row_data.push(pack_data[i as usize][start_idx..end_idx].to_vec());
            }
        }

        for j in 0..row_data.len() {
            if row_data[j].len() == 8 {
                let data_array: [u8; 8] = row_data[j]
                    .iter()
                    .map(|&x| x as u8)
                    .collect::<Vec<u8>>()
                    .try_into()
                    .unwrap_or([0; 8]);
                let crc_out = crc_check(&data_array);

                if crc_out == (row_data[j][7] & 0x0F) as u8 {
                    // CRC校验正确
                    evt_index += 1;
                    if (row_data[j][7] & 0b00110000) == 0 {
                        // 0: science evt, 16: GPS evt, 32: cal evt
                        let evt_energy = row_data[j][0]; // 脉冲信号的幅度
                        let evt_width = row_data[j][1]; // 脉冲信号的宽度
                        let evt_channel = (row_data[j][4] & 0b00111110) >> 1;
                        let ptime = ((row_data[j][4] & 1) << 18)
                            + (row_data[j][5] << 10)
                            + (row_data[j][6] << 2)
                            + ((row_data[j][7] & 0b11000000) >> 6);

                        evt_list.push(vec![
                            ptime as f64,
                            i as f64,
                            j as f64,
                            evt_index as f64,
                            -1.0,
                            evt_energy as f64,
                            evt_width as f64,
                            evt_channel as f64,
                        ]);
                    } else if (row_data[j][7] & 0b00110000) == 16 {
                        let stime = (row_data[j][0] << 24)
                            + (row_data[j][1] << 16)
                            + (row_data[j][2] << 8)
                            + row_data[j][3];
                        let ptime = ((row_data[j][4] & 1) << 18)
                            + (row_data[j][5] << 10)
                            + (row_data[j][6] << 2)
                            + ((row_data[j][7] & 0b11000000) >> 6);

                        gps_list.push(vec![
                            stime as f64,
                            ptime as f64,
                            i as f64,
                            j as f64,
                            evt_index as f64,
                        ]);
                    }
                } else {
                    crc_err_list.push((row_data[j].clone(), crc_out));
                }
            }
        }
    }

    if gps_list.is_empty() {
        return "Saturation".to_string();
    }

    let mut start_sec_index = 0.0;
    let mut start_sec_ptime = 0.0;

    // 找到与stime_a匹配的GPS数据
    let mut found_matching_stime = false;
    for gps_item in &gps_list {
        if gps_item[0] == stime_a as f64 {
            start_sec_index = gps_item[4];
            start_sec_ptime = gps_item[1];
            found_matching_stime = true;
            break;
        }
    }

    if !found_matching_stime && !evt_list.is_empty() {
        start_sec_index = 0.0;
        start_sec_ptime = evt_list[0][0];
    } else if !found_matching_stime {
        return "Saturation".to_string();
    }

    let mut last_ptime = start_sec_ptime;
    let mut gps_carry = 0;

    // 处理正向索引
    for i in (start_sec_index as usize + 1)..evt_list.len() {
        let mut real_time;
        if evt_list[i][0] - last_ptime < -1000.0 {
            gps_carry += 1;
            real_time = evt_list[i][0] + (gps_carry as f64 * 524288.0) - start_sec_ptime;
        } else {
            real_time = evt_list[i][0] + (gps_carry as f64 * 524288.0) - start_sec_ptime;
        }
        last_ptime = evt_list[i][0];
        evt_list[i][4] = real_time * 2.0 / 1000.0 / 1000.0; // 转换为秒
    }

    // 处理反向索引
    for i in (0..=(start_sec_index as usize)).rev() {
        let mut real_time;
        if evt_list[i][0] - last_ptime > 10000.0 {
            gps_carry -= 1;
            real_time = evt_list[i][0] + (gps_carry as f64 * 524288.0) - start_sec_ptime;
        } else {
            real_time = evt_list[i][0] + (gps_carry as f64 * 524288.0) - start_sec_ptime;
        }
        last_ptime = evt_list[i][0];
        evt_list[i][4] = real_time * 2.0 / 1000.0 / 1000.0; // 转换为秒
    }

    // 准备工作完成，进行物理事件筛选
    let mut evt_physics_list: Vec<Vec<f64>> = evt_list.clone();
    evt_physics_list.retain(|x| x[5] >= 15.0 && x[5] <= 250.0); // 能量筛选
    evt_physics_list.retain(|x| x[6] >= 40.0 && x[6] <= 80.0); // 脉冲宽度筛选

    if serial_num == "C" {
        evt_physics_list.retain(|x| x[7] != 4.0);
    }

    // 恢复部分缺失数据点
    let mut unique_pack_indices = Vec::new();
    for event in &evt_physics_list {
        if !unique_pack_indices.contains(&event[1]) {
            unique_pack_indices.push(event[1]);
        }
    }
    unique_pack_indices.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut rawx_err = vec![-1.0; unique_pack_indices.len()];
    let mut raw_time = vec![-1.0; unique_pack_indices.len()];
    let mut raw_cnt_rate = vec![-1.0; unique_pack_indices.len()];
    let mut pack_time_l = vec![-1.0; unique_pack_indices.len()];
    let mut pack_time_r = vec![-1.0; unique_pack_indices.len()];
    let mut cur_pack_all_cnt = vec![-1.0; unique_pack_indices.len()];
    let mut cur_pack_phy_cnt = vec![-1.0; unique_pack_indices.len()];

    let mut raw_all_cnt_rate = std::collections::HashMap::new();
    let mut temp_index = -1;

    // 对每个唯一的数据包索引进行处理
    for &pack_idx in &unique_pack_indices {
        temp_index += 1;

        // 找出属于当前数据包的所有事例
        let mut cur_pack_index = Vec::new();
        for (i, event) in evt_list.iter().enumerate() {
            if event[1] == pack_idx {
                cur_pack_index.push(i);
            }
        }

        let mut cur_pack_ptime = Vec::new();
        for &idx in &cur_pack_index {
            cur_pack_ptime.push(evt_list[idx][4]);
        }

        if !cur_pack_ptime.is_empty() {
            let min_time = *cur_pack_ptime
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let max_time = *cur_pack_ptime
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let time_range = max_time - min_time;

            rawx_err[temp_index as usize] = time_range / 2.0; // unit in second, 当前pack的时间宽度的一半
            raw_time[temp_index as usize] = (max_time + min_time) / 2.0;
            raw_all_cnt_rate.insert(temp_index, cur_pack_ptime.len() as f64 / time_range); // 当前pack包的计数率

            pack_time_l[temp_index as usize] = min_time;
            pack_time_r[temp_index as usize] = max_time;

            // 找出属于当前数据包的物理事例
            let mut cur_pack_phy_idx = Vec::new();
            for (i, event) in evt_physics_list.iter().enumerate() {
                if event[1] == pack_idx {
                    cur_pack_phy_idx.push(i);
                }
            }

            cur_pack_phy_cnt[temp_index as usize] = cur_pack_phy_idx.len() as f64;
            cur_pack_all_cnt[temp_index as usize] = cur_pack_index.len() as f64;
            raw_cnt_rate[temp_index as usize] = cur_pack_phy_idx.len() as f64 / time_range;
            // 当前pack包的计数率
        }
    }

    // 找出相邻pack的时间差，如果过大说明有丢数
    let mut pack_time_gap = Vec::with_capacity(pack_time_r.len() - 1);
    for i in 0..(pack_time_r.len() - 1) {
        pack_time_gap.push(pack_time_r[i] - pack_time_l[i + 1]);
    }

    let phy_ratio: Vec<f64> = cur_pack_phy_cnt
        .iter()
        .zip(cur_pack_all_cnt.iter())
        .map(|(&phy, &all)| if all != 0.0 { phy / all } else { 0.0 })
        .collect();

    let gap_thr = 5.0 / 1000.0; // 设置一个相邻pack时间差的阈值，大于这个值的认为丢数了

    for &gap in &pack_time_gap {
        if gap > gap_thr {
            return "True".to_string();
        }
    }

    "False".to_string()
}
