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
    pub(crate) start_best: DateTime<Utc>,
    pub(crate) stop_best: DateTime<Utc>,
    pub(crate) duration_best: f64,
    pub(crate) fp_year: f64,
    pub(crate) count: u32,
    pub(crate) count_best: u32,
    pub(crate) count_filtered: u32,
    pub(crate) count_filtered_best: u32,
    pub(crate) background: f64,
    pub(crate) flux: f64,
    pub(crate) flux_best: f64,
    pub(crate) flux_filtered: f64,
    pub(crate) flux_filtered_best: f64,
    pub(crate) mean_energy: f64,
    pub(crate) mean_energy_best: f64,
    pub(crate) mean_energy_filtered: f64,
    pub(crate) mean_energy_filtered_best: f64,
    pub(crate) veto_ratio: f64,
    pub(crate) veto_ratio_best: f64,
    pub(crate) veto_ratio_filtered: f64,
    pub(crate) veto_ratio_filtered_best: f64,
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
