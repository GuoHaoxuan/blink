use std::fmt::Display;

use itertools::Itertools;

use crate::{
    hxmt::{
        instance::{EngFile, SciFile},
        Hxmt,
    },
    types::Time,
};

use super::{crc_check_impl::crc_check, find_stime_impl::find_stime};

pub enum SerialNum {
    A,
    B,
    C,
}

impl Display for SerialNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerialNum::A => write!(f, "A"),
            SerialNum::B => write!(f, "B"),
            SerialNum::C => write!(f, "C"),
        }
    }
}

pub fn rec_sci_data(
    time: Time<Hxmt>,
    serial_num: SerialNum,
    eng_data: &EngFile,
    sci_data: &SciFile,
) -> bool {
    let (stime_a, evt_utc_stamp, _) = match find_stime(eng_data, time) {
        Ok(result) => result,
        Err(_) => return true,
    };

    let evt_range = 2; // 选取 evt 发生点前后多长时间的数据，evt_range / 2，单位是秒
    let evt_utc_floor = evt_utc_stamp - evt_range / 2;
    let evt_utc_ceil = evt_utc_stamp + evt_range / 2;

    // process data from unit a
    let evt_data = &sci_data.ccsds;
    let pack_utc_code: Vec<u64> = evt_data
        .iter()
        .map(|x| {
            let bytes = &x[878..882];
            (bytes[0] as u64)
                + ((bytes[1] as u64) << 8)
                + ((bytes[2] as u64) << 16)
                + ((bytes[3] as u64) << 24)
        })
        .collect();

    // 提取指定时间区间的 UTC time 的序号
    let pack_data = pack_utc_code
        .iter()
        .zip(evt_data.iter())
        .filter(|(utc, _)| **utc >= evt_utc_floor - 1 && **utc <= evt_utc_ceil + 1)
        .map(|(_, data)| &data[6..878])
        .collect::<Vec<_>>();
    let mut burst_found = 0;

    let mut start_sec: [usize; 4] = [0; 4];
    let mut stop_sec: [usize; 4] = [0; 4];
    for (i, pack) in pack_data.iter().enumerate() {
        let mut row_data = [[0u64; 8]; 109];
        for row in 0..109 {
            for col in 0..8 {
                row_data[row][col] = pack[row * 8 + col] as u64;
            }
        }
        for (j, row) in row_data.iter().enumerate() {
            // 代表 GPS 事例
            if row[7] & 0b0011_0000 == 16 {
                let crc_out = crc_check(row);
                // CRC 校验正确，&0x0F取出后四位
                if crc_out == row[7] & 15 {
                    let stime = (row[0] << 24) + (row[1] << 16) + (row[2] << 8) + row[3];
                    let ptime = ((row[4] & 1) << 18)
                        + (row[5] << 10)
                        + (row[6] << 2)
                        + ((row[7] & 0b1100_0000) >> 6);
                    if stime == stime_a as u64 - evt_range / 2 {
                        // 开始的秒事例的位置
                        burst_found |= 1;
                        start_sec = [stime as usize, ptime as usize, i, j];
                    } else if stime == stime_a as u64 + evt_range / 2 {
                        burst_found |= 2;
                        stop_sec = [stime as usize, ptime as usize, i, j];
                        break;
                    } else {
                        continue;
                    }
                }
            }
        }
        if burst_found == 3 {
            // println!("Burst GPS evt found in unit-{}!!", serial_num);
            // println!("Start sec evt: {:?}", start_sec);
            // println!("Stop sec evt: {:?}", stop_sec);
            break;
        }
    }

    if burst_found != 3 {
        // println!("Burst GPS evt not found in unit-{}!!", serial_num);
        return true;
    }

    let mut evt_index = 0;
    let mut evt_list: Vec<(u64, u64, u64, u64, f64, u64, u64, u64)> = Vec::new();
    let mut gps_list = Vec::new();
    let mut crc_err_list = Vec::new();

    for (i, &pack) in pack_data
        .iter()
        .enumerate()
        .take(stop_sec[2] + 1)
        .skip(start_sec[2])
    {
        let mut row_data = [[0u64; 8]; 109];
        for (row_index, row) in row_data.iter_mut().enumerate() {
            for (col_index, col) in row.iter_mut().enumerate() {
                *col = pack[row_index * 8 + col_index] as u64;
            }
        }
        for (j, row) in row_data.iter().enumerate() {
            let crc_out = crc_check(row);
            // CRC 校验正确，&0x0F取出后四位
            if crc_out == row[7] & 15 {
                evt_index += 1;
                // 0: science evt, 16: GPS evt, 32: cal evt
                if row[7] & 0b0011_0000 == 0 {
                    let evt_energy = row[0]; // 脉冲信号的幅度
                    let evt_width = row[1]; // 脉冲信号的宽度
                    let evt_channel = (row[4] & 0b0011_1110) >> 1;
                    let ptime = ((row[4] & 1) << 18)
                        + (row[5] << 10)
                        + (row[6] << 2)
                        + ((row[7] & 0b1100_0000) >> 6);
                    evt_list.push((
                        ptime,
                        i as u64,
                        j as u64,
                        evt_index,
                        0.0,
                        evt_energy,
                        evt_width,
                        evt_channel,
                    ));
                } else if row_data[j][7] & 0b0011_0000 == 16 {
                    let stime = (row_data[j][0] << 24)
                        + (row_data[j][1] << 16)
                        + (row_data[j][2] << 8)
                        + row_data[j][3];
                    let ptime = ((row_data[j][4] & 1) << 18)
                        + (row_data[j][5] << 10)
                        + (row_data[j][6] << 2)
                        + ((row_data[j][7] & 0b1100_0000) >> 6);
                    gps_list.push([stime, ptime, i as u64, j as u64, evt_index]);
                }
            } else {
                crc_err_list.push((row_data[j], crc_out));
            }
        }
    }

    let start_sec_index;
    let start_sec_ptime;
    if !gps_list.is_empty() {
        let temp = gps_list
            .iter()
            .position(|&x| x[0] == stime_a as u64)
            .unwrap();
        start_sec_index = gps_list[temp][4];
        start_sec_ptime = gps_list[temp][1];
    } else {
        start_sec_index = 0;
        if evt_list.is_empty() {
            return true;
        }
        start_sec_ptime = evt_list[0].0;
    }

    // 上一个事例的 ptime
    let mut last_ptime = start_sec_ptime;
    let mut gps_carry: i64 = 0;
    let mut real_time: i64;
    for item in evt_list.iter_mut().skip(start_sec_index as usize + 1) {
        real_time = if (item.0 as i64 - last_ptime as i64) < -1000 {
            gps_carry += 1;
            item.0 as i64 + gps_carry * 524_288 - start_sec_ptime as i64
        } else {
            item.0 as i64 + gps_carry * 524_288 - start_sec_ptime as i64
        };
        last_ptime = item.0;
        item.4 = real_time as f64 * 2.0 / 1000.0 / 1000.0; // unit in second
    }

    last_ptime = start_sec_ptime;
    gps_carry = 0;
    for i in (0..start_sec_index as usize).rev() {
        if (evt_list[i].0 as i64 - last_ptime as i64) > 10_000 {
            gps_carry -= 1;
            real_time = evt_list[i].0 as i64 + gps_carry * 524_288 - start_sec_ptime as i64;
        } else {
            real_time = evt_list[i].0 as i64 + gps_carry * 524_288 - start_sec_ptime as i64;
        }
        last_ptime = evt_list[i].0;
        evt_list[i].4 = real_time as f64 * 2.0 / 1000.0 / 1000.0; // unit in second
    }
    // 准备工作完成

    let evt_phy_list = evt_list
        .iter()
        .filter(|&x| x.5 >= 15 && x.5 <= 250) // 能量筛选
        .filter(|&x| x.6 >= 40 && x.6 <= 80) // 脉冲宽度筛选
        .filter(|&x| !matches!(serial_num, SerialNum::C) || x.7 != 4)
        .collect::<Vec<_>>();

    let pack_index_list = evt_list.iter().map(|x| x.1).unique().collect::<Vec<_>>();
    // 恢复部分缺失数据点
    // let mut rawx_err: Vec<f64> = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];
    // let mut raw_time = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];
    // let mut raw_cnt_rate = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];
    let mut pack_time_l = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];
    let mut pack_time_r = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];
    // let mut cur_pack_all_cnt = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];
    // let mut cur_pack_phy_cnt = vec![-1.0; evt_phy_list.iter().map(|x| x.1).unique().count()];

    // 查看原始数据包的计数情况（全部事例）
    // 找出不重复的每个事例数据包，已排序
    let mut raw_all_cnt_rate = Vec::new();
    for (temp_index, i) in pack_index_list.iter().enumerate() {
        let cur_pack_index = evt_list
            .iter()
            .positions(|&x| x.1 == *i)
            .collect::<Vec<_>>();
        let cur_pack_ptime = cur_pack_index
            .iter()
            .map(|&index| evt_list[index].4)
            .collect::<Vec<_>>();
        // // unit in second, 当前 pack 的时间宽度的一半
        // rawx_err[temp_index] = (cur_pack_ptime
        //     .iter()
        //     .max_by(|a, b| a.partial_cmp(b).unwrap())
        //     .unwrap()
        //     - cur_pack_ptime
        //         .iter()
        //         .min_by(|a, b| a.partial_cmp(b).unwrap())
        //         .unwrap())
        //     / 2.0;
        // raw_time[temp_index] = (cur_pack_ptime
        //     .iter()
        //     .max_by(|a, b| a.partial_cmp(b).unwrap())
        //     .unwrap()
        //     + cur_pack_ptime
        //         .iter()
        //         .min_by(|a, b| a.partial_cmp(b).unwrap())
        //         .unwrap())
        //     / 2.0;
        // 当前 pack 包的计数率
        raw_all_cnt_rate.push(
            cur_pack_index.len() as f64
                / (cur_pack_ptime
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap()
                    - cur_pack_ptime
                        .iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap()),
        );

        pack_time_l[temp_index] = *cur_pack_ptime
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        pack_time_r[temp_index] = *cur_pack_ptime
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        // let cur_pack_phy_index = evt_phy_list
        //     .iter()
        //     .positions(|&x| x.1 == *i)
        //     .collect::<Vec<_>>();
        // cur_pack_phy_cnt[temp_index] = cur_pack_phy_index.len() as f64;
        // cur_pack_all_cnt[temp_index] = cur_pack_index.len() as f64;
        // 当前 pack 包的计数率
        // raw_cnt_rate[temp_index] = cur_pack_phy_index.len() as f64 / cur_pack_ptime.len() as f64;
    }
    // 找出相邻 pack 的时间差，如果过大说明有丢数
    let pack_time_gap = pack_time_r[..pack_time_r.len() - 1]
        .iter()
        .zip(pack_time_l[1..].iter())
        .map(|(r, l)| r - l)
        .collect::<Vec<_>>();

    // let phy_ratio: Vec<f64> = cur_pack_phy_cnt
    //     .iter()
    //     .zip(cur_pack_all_cnt.iter())
    //     .map(|(phy, all)| *phy / *all)
    //     .collect();

    // let cnt_all_rate = raw_all_cnt_rate;
    // let cnt_rate = raw_cnt_rate;
    // 正常情况下PDAU板处理一个数据包的时间为6.9ms
    // let loop_t = 6.9 / 1000.0;
    // 设置一个相邻pack时间差的阈值，大于这个值的认为丢数了
    let gap_thr = 5.0 / 1000.0;

    if pack_time_gap.iter().any(|&gap| gap > gap_thr) {
        return true;
    }

    false
}
