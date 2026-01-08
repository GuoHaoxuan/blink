use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uom::si::f64::*;

use crate::{
    traits::Event,
    types::{Attitude, MissionElapsedTime, Position},
};

#[derive(Serialize, Deserialize)]
pub struct Signal<E: Event> {
    pub start: MissionElapsedTime<E::Satellite>,
    pub stop: MissionElapsedTime<E::Satellite>,
    pub bin_size_min: Time,
    pub bin_size_max: Time,
    pub bin_size_best: Time,
    pub delay: Time,
    pub count: u32,
    pub mean: f64,
    pub sf: f64,
    pub false_positive_per_year: f64,
    pub attitude: Attitude,
    pub position: Position,
}

impl<E: Event> Signal<E> {
    pub fn to_unified(&self) -> UnifiedSignal {
        UnifiedSignal {
            start: self.start.to_utc(),
            stop: self.stop.to_utc(),
            bin_size_min: self.bin_size_min,
            bin_size_max: self.bin_size_max,
            bin_size_best: self.bin_size_best,
            delay: self.delay,
            count: self.count,
            mean: self.mean,
            sf: self.sf,
            false_positive_per_year: self.false_positive_per_year,
            attitude: self.attitude.clone(),
            position: self.position.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UnifiedSignal {
    pub start: DateTime<Utc>,
    pub stop: DateTime<Utc>,
    pub bin_size_min: Time,
    pub bin_size_max: Time,
    pub bin_size_best: Time,
    pub delay: Time,
    pub count: u32,
    pub mean: f64,
    pub sf: f64,
    pub false_positive_per_year: f64,
    pub attitude: Attitude,
    pub position: Position,
}
