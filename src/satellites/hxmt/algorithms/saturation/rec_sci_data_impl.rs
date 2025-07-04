use super::super::super::{
    data::data_1b::{EngFile, SciFile},
    types::Hxmt,
};
use crate::types::Time;

use super::{crc_check_impl::crc_check, find_stime_impl::find_stime};

#[derive(Debug)]
enum Pack {
    Event {
        ptime: u64,
        // evt_energy: u64,
        // evt_width: u64,
        // evt_channel: u64,
    },
    Second {
        stime: u64,
        ptime: u64,
    },
    Error,
}

pub fn rec_sci_data(time: Time<Hxmt>, eng_data: &EngFile, sci_data: &SciFile) -> bool {
    let (stime_pivot, evt_utc_stamp, _) = match find_stime(eng_data, time) {
        Ok(result) => result,
        Err(_) => return true,
    };

    let evt_range = 2;
    let evt_utc_floor = evt_utc_stamp - evt_range / 2;
    let evt_utc_ceil = evt_utc_stamp + evt_range / 2;

    let packs = sci_data
        .ccsds
        .iter()
        .filter(|x| {
            let bytes = &x[878..882];
            let utc = (bytes[0] as u64)
                + ((bytes[1] as u64) << 8)
                + ((bytes[2] as u64) << 16)
                + ((bytes[3] as u64) << 24);
            utc >= evt_utc_floor - 1 && utc <= evt_utc_ceil + 1
        })
        .collect::<Vec<_>>();

    let packs_resolved = packs
        .into_iter()
        .map(|x| &x[6..878])
        .map(|pack| {
            pack.chunks_exact(8)
                .map(|chunk| {
                    let mut row = [0u64; 8];
                    for (i, byte) in chunk.iter().enumerate() {
                        row[i] = *byte as u64;
                    }
                    row
                })
                .map(|row| {
                    if crc_check(&row) == row[7] & 0x0F {
                        match row[7] & 0x30 {
                            0x00 | 0x20 => {
                                // let evt_energy = row[0]; // 脉冲信号的幅度
                                // let evt_width = row[1]; // 脉冲信号的宽度
                                // let evt_channel = (row[4] & 0x3E) >> 1;
                                let ptime = ((row[4] & 1) << 18)
                                    + (row[5] << 10)
                                    + (row[6] << 2)
                                    + ((row[7] & 0xC0) >> 6);
                                Pack::Event {
                                    ptime,
                                    // evt_energy,
                                    // evt_width,
                                    // evt_channel,
                                }
                            }
                            0x10 => {
                                let stime =
                                    (row[0] << 24) + (row[1] << 16) + (row[2] << 8) + row[3];
                                let ptime = ((row[4] & 1) << 18)
                                    + (row[5] << 10)
                                    + (row[6] << 2)
                                    + ((row[7] & 0b1100_0000) >> 6);
                                Pack::Second { stime, ptime }
                            }
                            _ => Pack::Error,
                        }
                    } else {
                        Pack::Error
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut start_index: Option<usize> = None;
    let mut stop_index: Option<usize> = None;

    for (index, val) in packs_resolved.iter().enumerate() {
        for pack in val {
            if let Pack::Second { stime, .. } = pack {
                if *stime == stime_pivot as u64 - evt_range / 2 {
                    start_index = Some(index);
                }
                if *stime == stime_pivot as u64 + evt_range / 2 {
                    stop_index = Some(index);
                }
            }
        }
    }

    if start_index.is_none() || stop_index.is_none() {
        return true;
    }

    let packs_filtered = packs_resolved
        .iter()
        .skip(start_index.unwrap())
        .take(stop_index.unwrap() - start_index.unwrap() + 1)
        .collect::<Vec<_>>();

    let times = packs_filtered
        .iter()
        .scan(stime_pivot as u64 - evt_range / 2, |stime_cache, packs| {
            let times = packs
                .iter()
                .filter_map(|pack| match pack {
                    Pack::Event { ptime, .. } => Some(*stime_cache as f64 + *ptime as f64 * 2e-6),
                    Pack::Second { stime, ptime } => {
                        *stime_cache = *stime;
                        Some(*stime_cache as f64 + *ptime as f64 * 2e-6)
                    }
                    Pack::Error => None,
                })
                .collect::<Vec<_>>();
            Some((
                *times
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap(),
                *times
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap(),
            ))
        })
        .collect::<Vec<_>>();

    let gaps = times
        .windows(2)
        .map(|window| window[1].0 - window[0].1)
        .collect::<Vec<_>>();

    gaps.iter().any(|gap| *gap > 6.9e-3)
}
