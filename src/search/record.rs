use hifitime::prelude::*;
use hifitime::{Duration, Epoch};
use std::fmt::Debug;

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
    pub fp_year: f64,
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
            fp_year: trigger.fp_year,
        }
    }
}

impl Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!(
            "{} + {}, {}>{:.2}",
            self.start.to_isoformat(),
            (self.stop - self.start),
            self.count,
            self.average,
        )
        .fmt(f)
    }
}
