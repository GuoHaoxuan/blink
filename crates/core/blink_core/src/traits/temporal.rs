use crate::{traits::Instrument, types::MissionElapsedTime};
use std::ops::Add;
use std::ops::Sub;

pub trait Temporal:
    PartialOrd + Copy + Add<Self::Duration, Output = Self> + Sub<Self::Duration, Output = Self>
{
    type Duration: Copy;

    fn lerp_factor(self, start: Self, end: Self) -> f64;
}

impl<I: Instrument> Temporal for MissionElapsedTime<I> {
    type Duration = uom::si::f64::Time;

    fn lerp_factor(self, start: Self, end: Self) -> f64 {
        let duration_total = end.time() - start.time();
        let duration_part = self.time() - start.time();
        (duration_part / duration_total).get::<uom::si::ratio::ratio>()
    }
}

impl Temporal for chrono::DateTime<chrono::Utc> {
    type Duration = chrono::Duration;

    fn lerp_factor(self, start: Self, end: Self) -> f64 {
        let duration_total = end.signed_duration_since(start);
        let duration_part = self.signed_duration_since(start);
        duration_part.num_nanoseconds().unwrap() as f64
            / duration_total.num_nanoseconds().unwrap() as f64
    }
}
