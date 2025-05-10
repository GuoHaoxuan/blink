use chrono::prelude::*;
use serde::Serialize;

use crate::lightning::LightningAssociation;

use super::GenericEvent;

#[derive(Debug, Serialize)]
pub(crate) struct Location {
    pub(crate) longitude: f64,
    pub(crate) latitude: f64,
    pub(crate) altitude: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct Signal {
    pub(crate) start: DateTime<Utc>,
    pub(crate) stop: DateTime<Utc>,
    pub(crate) duration: f64,
    pub(crate) best_start: DateTime<Utc>,
    pub(crate) best_stop: DateTime<Utc>,
    pub(crate) best_duration: f64,
    pub(crate) fp_year: f64,
    pub(crate) count: u32,
    pub(crate) best_count: u32,
    pub(crate) count_all: u32,
    pub(crate) background: f64, // counts per second
    pub(crate) flux: f64,       // counts per second
    pub(crate) flux_best: f64,  // counts per second
    pub(crate) flux_all: f64,   // counts per second
    pub(crate) mean_energy: f64,
    pub(crate) veto_ratio: f64,
    pub(crate) events: Vec<GenericEvent>,
    pub(crate) light_curve_1s: Vec<u32>,
    pub(crate) light_curve_1s_filtered: Vec<u32>,
    pub(crate) light_curve_100ms: Vec<u32>,
    pub(crate) light_curve_100ms_filtered: Vec<u32>,
    pub(crate) longitude: f64,
    pub(crate) latitude: f64,
    pub(crate) altitude: f64,
    pub(crate) q1: f64,
    pub(crate) q2: f64,
    pub(crate) q3: f64,
    pub(crate) orbit: Vec<Location>,
    pub(crate) lightnings: Vec<LightningAssociation>,
    pub(crate) associated_lightning_count: u32,
    pub(crate) coincidence_probability: f64,
}
