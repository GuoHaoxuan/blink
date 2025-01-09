use hifitime::prelude::*;
use rusqlite::{Connection, Result};

use super::trigger::Trigger;

pub struct Record {
    pub start: Epoch,
    pub stop: Epoch,
    pub bin_size_min: Duration,
    pub bin_size_max: Duration,
    pub bin_size_best: Duration,
    pub delay: Duration,
    pub count: u32,
    pub average: f64,
}

impl Record {
    pub fn new(trigger: &Trigger, date_obs: Epoch) -> Record {
        Record {
            start: date_obs + (trigger.start * 1e6).round().microseconds(),
            stop: date_obs + (trigger.stop * 1e6).round().microseconds(),
            bin_size_min: (trigger.bin_size_min * 1e6).round().microseconds(),
            bin_size_max: (trigger.bin_size_max * 1e6).round().microseconds(),
            bin_size_best: (trigger.bin_size_best * 1e6).round().microseconds(),
            delay: (trigger.delay * 1e6).round().microseconds(),
            count: trigger.count,
            average: trigger.average,
        }
    }

    pub fn save(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "
                INSERT INTO
                    records (
                        start,
                        stop,
                        bin_size_min,
                        bin_size_max,
                        bin_size_best,
                        delay,
                        count,
                        average
                    )
                VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);
            ",
            (
                self.start.to_string(),
                self.stop.to_string(),
                self.bin_size_min.total_nanoseconds() as u32 / 1000,
                self.bin_size_max.total_nanoseconds() as u32 / 1000,
                self.bin_size_best.total_nanoseconds() as u32 / 1000,
                self.delay.total_nanoseconds() as u32 / 1000,
                self.count,
                self.average,
            ),
        )
        .map(|_| ())
    }
}
