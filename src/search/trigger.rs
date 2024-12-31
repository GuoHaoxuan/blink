use super::poisson::poisson_isf;
use statrs::distribution::{DiscreteCDF, Poisson};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct Trigger {
    pub start: f64,
    pub stop: f64,
    pub bin_size_min: f64,
    pub bin_size_max: f64,
    pub bin_size_best: f64,
    pub delay: f64,
    pub count: u32,
    pub average: f64,
    pub fp_year: f64,
}

impl Trigger {
    pub fn new(start: f64, stop: f64, count: u32, average: f64, fp_year: f64) -> Trigger {
        let bin_size = stop - start;
        Trigger {
            start,
            stop,
            bin_size_min: bin_size,
            bin_size_max: bin_size,
            bin_size_best: bin_size,
            delay: 0.0,
            count,
            average,
            fp_year,
        }
    }

    pub fn sf(&self) -> f64 {
        Poisson::new(self.average).unwrap().sf(self.count as u64)
    }

    pub fn mergeable(&self, other: &Trigger, vision: u32) -> bool {
        self.stop + self.bin_size_max.max(other.bin_size_max) * (vision as f64) > other.start
    }

    pub fn merge(&self, other: &Trigger) -> Trigger {
        let mut res = self.clone();
        res = Trigger {
            stop: res.stop.max(other.stop),
            bin_size_min: res.bin_size_min.min(other.bin_size_min),
            bin_size_max: res.bin_size_max.max(other.bin_size_max),
            ..res
        };
        if other.sf() < res.sf() {
            res = Trigger {
                count: other.count,
                average: other.average,
                bin_size_best: other.bin_size_best,
                fp_year: other.fp_year,
                delay: other.start - res.start,
                ..res
            };
        }
        res
    }

    pub fn threshold(&self) -> u32 {
        poisson_isf(
            self.fp_year / (3600.0 * 24.0 * 365.0 / (self.stop - self.start)),
            self.average,
        )
    }
}
