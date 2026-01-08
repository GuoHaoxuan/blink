use crate::io::level_1b::EngFile;
use crate::types::HxmtHe;
use blink_core::error::Error;
use blink_core::types::MissionElapsedTime;

pub enum StimeType {
    GpsNormal,
    GpsUnlocked,
}

pub fn find_stime(
    eng_file: &EngFile,
    evt_time: MissionElapsedTime<HxmtHe>,
) -> Result<(i32, u64, StimeType), Error> {
    // 给定时间点和对应的 1B 级工程数据文件名，从其中找出对应的 stime 值
    // 如果没有找到对应的时间点，则返回的 stime 值为 -1。
    // 返回值：1. 对应本模块的 stime 值；2. 对应 HXMT 零点的 UTC 秒数。
    // 输入参数 evt_time 为 UTC 时间，格式是 YYYYMMDDTHH:mm:ss

    let time_stamp = evt_time.time().get::<uom::si::time::second>().round() as u64;

    // --- find burst stime from unit A
    // 获取整列数据，6字节，并转换成十进制数。
    let bus_time_bdc = &eng_file.bus_time_bdc;
    let mut bus_time: Vec<u64> = Vec::from_iter(bus_time_bdc.iter().map(|code| {
        ((code[6 - 1] as u64 & 127) << 24)
            + ((code[5 - 1] as u64) << 16)
            + ((code[4 - 1] as u64) << 8)
            + (code[3 - 1] as u64)
    }));

    let stime = &eng_file.time;
    let stime_type;
    let stime_a;
    if bus_time.contains(&time_stamp) {
        let index = bus_time
            .iter()
            .position(|&x| x == time_stamp)
            .ok_or_else(|| {
                Error::InvalidData("Time stamp not found in bus_time after recovery".to_string())
            })?;
        // println!("burstLoc in this unit (UTC time) is {} row.", index);
        stime_a = stime[index];
        stime_type = StimeType::GpsNormal; // 代表 GPS 正常状态
    // println!("burst stime in this unit is: {}.", stime_a);
    } else {
        let stime_diff = bus_time
            .iter()
            .scan(0, |state, &x| {
                let diff = x - *state;
                *state = x;
                Some(diff)
            })
            .skip(1)
            .collect::<Vec<_>>();
        for i in 0..stime_diff.len() {
            if stime_diff[i] == 0 {
                bus_time[i + 1] = bus_time[i] + 1;
            }
        }
        // println!("NOT found burstLoc in this unit!");
        // println!("Maybe GPS unlocked, Recovered STIME");
        let index = bus_time
            .iter()
            .position(|&x| x == time_stamp)
            .ok_or_else(|| {
                Error::InvalidData("Time stamp not found in bus_time after recovery".to_string())
            })?;
        // println!("burstLoc in this unit (UTC time) is {} row.", index);
        stime_a = stime[index];
        stime_type = StimeType::GpsUnlocked; // 代表 GPS 失锁状态
        // println!("burst stime in this unit is: {}.", stime_a);
    }

    Ok((stime_a, time_stamp, stime_type))
}
