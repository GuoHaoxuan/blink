use uom::si::f64::*;

use crate::{
    traits::Event,
    types::{Attitude, MissionElapsedTime, Position, TemporalState, Trajectory},
};

pub struct Signal<E: Event> {
    pub start: MissionElapsedTime<E::Satellite>,
    pub stop: MissionElapsedTime<E::Satellite>,
    pub bin_size_min: Time,
    pub bin_size_max: Time,
    pub bin_size_best: Time,
    pub delay: Time,
    pub count: u32,
    pub mean: f64,
    pub false_positive_per_year: f64,
    pub events: Vec<E>,
    pub attitude: TemporalState<MissionElapsedTime<E::Satellite>, Attitude>,
    pub orbit: Trajectory<MissionElapsedTime<E::Satellite>, Position>,
}
