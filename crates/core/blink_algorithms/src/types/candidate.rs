use crate::poisson;
use blink_core::{traits::Instrument, types::MissionElapsedTime};
use uom::si::f64::*;

#[derive(Clone)]
pub struct Candidate<I: Instrument> {
    pub start: MissionElapsedTime<I>,
    pub stop: MissionElapsedTime<I>,
    pub bin_size_min: Time,
    pub bin_size_max: Time,
    pub bin_size_best: Time,
    pub delay: Time,
    pub count: u32,
    pub mean: f64,
}

impl<I: Instrument> Candidate<I> {
    pub fn new(
        start: MissionElapsedTime<I>,
        stop: MissionElapsedTime<I>,
        count: u32,
        mean: f64,
    ) -> Candidate<I> {
        let bin_size = stop - start;
        Candidate {
            start,
            stop,
            bin_size_min: bin_size,
            bin_size_max: bin_size,
            bin_size_best: bin_size,
            delay: Time::new::<uom::si::time::second>(0.0),
            count,
            mean,
        }
    }

    pub fn sf(&self) -> f64 {
        poisson::sf(self.mean, self.count)
    }

    pub fn false_positive_per_year(&self) -> f64 {
        poisson::false_positive_per_year(self.sf(), self.bin_size_best)
    }

    pub fn mergeable(&self, other: &Self, vision: f64) -> bool {
        self.stop + self.bin_size_max.max(other.bin_size_max) * vision >= other.start
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut res = self.clone();
        res = Candidate {
            stop: res.stop.max(other.stop),
            bin_size_min: res.bin_size_min.min(other.bin_size_min),
            bin_size_max: res.bin_size_max.max(other.bin_size_max),
            ..res
        };
        if other.false_positive_per_year() < res.false_positive_per_year() {
            res = Candidate {
                count: other.count,
                mean: other.mean,
                bin_size_best: other.bin_size_best,
                delay: other.start - res.start,
                ..res
            };
        }
        res
    }
}
