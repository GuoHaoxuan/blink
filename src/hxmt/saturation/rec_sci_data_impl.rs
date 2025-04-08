use crate::{
    hxmt::{
        instance::{EngFile, SciFile},
        Hxmt,
    },
    types::Time,
};

use super::{crc_check_impl::crc_check, find_stime_impl::find_stime};

#[derive(Debug)]
enum Pack {
    Science {
        ptime: u64,
        // evt_energy: u64,
        // evt_width: u64,
        // evt_channel: u64,
    },
    Gps {
        // stime: u64,
        // ptime: u64,
    },
    Cal,
    Error,
}

pub fn rec_sci_data(time: Time<Hxmt>, eng_data: &EngFile, sci_data: &SciFile) -> bool {
    let (_, evt_utc_stamp, _) = match find_stime(eng_data, time) {
        Ok(result) => result,
        Err(_) => return true,
    };

    let evt_range = 2;
    let evt_utc_floor = evt_utc_stamp - evt_range / 2;
    let evt_utc_ceil = evt_utc_stamp + evt_range / 2;

    let out_of_thresholds = sci_data
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
        .map(|x| &x[6..878])
        .flat_map(|pack| {
            pack.chunks_exact(8)
                .map(|chunk| {
                    let mut row = [0u64; 8];
                    for (i, byte) in chunk.iter().enumerate() {
                        row[i] = *byte as u64;
                    }
                    row
                })
                .collect::<Vec<_>>()
        })
        .map(|row| {
            if crc_check(&row) == row[7] & 0x0F {
                match row[7] & 0x30 {
                    0 => {
                        // let evt_energy = row[0]; // 脉冲信号的幅度
                        // let evt_width = row[1]; // 脉冲信号的宽度
                        // let evt_channel = (row[4] & 0x3E) >> 1;
                        let ptime = ((row[4] & 1) << 18)
                            + (row[5] << 10)
                            + (row[6] << 2)
                            + ((row[7] & 0xC0) >> 6);
                        Pack::Science {
                            ptime,
                            // evt_energy,
                            // evt_width,
                            // evt_channel,
                        }
                    }
                    16 => {
                        // let stime = (row[0] << 24) + (row[1] << 16) + (row[2] << 8) + row[3];
                        // let ptime = ((row[4] & 1) << 18)
                        //     + (row[5] << 10)
                        //     + (row[6] << 2)
                        //     + ((row[7] & 0b1100_0000) >> 6);
                        // Pack::Gps { stime, ptime }
                        Pack::Gps {}
                    }
                    32 => Pack::Cal,
                    _ => Pack::Error,
                }
            } else {
                Pack::Error
            }
        })
        .filter(|pack| matches!(pack, Pack::Science { .. }))
        .scan(0, |last_ptime, pack| match pack {
            Pack::Science { ptime, .. } => {
                let mut ptime = ptime;
                while ptime < *last_ptime {
                    ptime += 0x80000;
                }
                let gap = ptime - *last_ptime;
                *last_ptime = ptime;
                Some(gap)
            }
            _ => None,
        })
        .skip(1)
        .map(|x| x as f64 * 2.0 / 1000.0 / 1000.0)
        .filter(|x| *x > 6.9 / 1000.0)
        .count();

    out_of_thresholds > 0
}
