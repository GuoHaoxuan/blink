use super::poission::{
    false_positive_per_year as poission_false_positive_per_year, sf as poisson_sf,
};
use serde::Serialize;
use std::fmt::Debug;

use crate::types::{Satellite, Span, Time};

#[derive(Clone, Debug, Serialize)]
pub struct Trigger<S: Satellite> {
    pub start: Time<S>,
    pub stop: Time<S>,
    pub bin_size_min: Span<S>,
    pub bin_size_max: Span<S>,
    pub bin_size_best: Span<S>,
    pub delay: Span<S>,
    pub count: u32,
    pub mean: f64,
}

impl<S: Satellite> Trigger<S> {
    pub fn new(start: Time<S>, stop: Time<S>, count: u32, mean: f64) -> Trigger<S> {
        let bin_size = stop - start;
        Trigger {
            start,
            stop,
            bin_size_min: bin_size,
            bin_size_max: bin_size,
            bin_size_best: bin_size,
            delay: Span::seconds(0.0),
            count,
            mean,
        }
    }

    pub fn sf(&self) -> f64 {
        poisson_sf(self.mean, self.count)
    }

    pub fn false_positive_per_year(&self) -> f64 {
        poission_false_positive_per_year(self.sf(), self.bin_size_best)
    }

    pub fn mergeable(&self, other: &Self, vision: f64) -> bool {
        self.stop + self.bin_size_max.max(other.bin_size_max) * vision >= other.start
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut res = self.clone();
        res = Trigger {
            stop: res.stop.max(other.stop),
            bin_size_min: res.bin_size_min.min(other.bin_size_min),
            bin_size_max: res.bin_size_max.max(other.bin_size_max),
            ..res
        };
        if other.false_positive_per_year() < res.false_positive_per_year() {
            res = Trigger {
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
