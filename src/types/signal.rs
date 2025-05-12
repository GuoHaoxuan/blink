use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::lightning::LightningAssociation;

use super::GenericEvent;

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
}

#[derive(Debug, Serialize)]
pub struct Signal {
    pub start: DateTime<Utc>,
    pub stop: DateTime<Utc>,
    pub duration: f64,
    pub start_best: DateTime<Utc>,
    pub stop_best: DateTime<Utc>,
    pub duration_best: f64,
    pub fp_year: f64,
    pub count: u32,
    pub count_best: u32,
    pub count_filtered: u32,
    pub count_filtered_best: u32,
    pub background: f64,
    pub flux: f64,
    pub flux_best: f64,
    pub flux_filtered: f64,
    pub flux_filtered_best: f64,
    pub mean_energy: f64,
    pub mean_energy_best: f64,
    pub mean_energy_filtered: f64,
    pub mean_energy_filtered_best: f64,
    pub veto_ratio: f64,
    pub veto_ratio_best: f64,
    pub veto_ratio_filtered: f64,
    pub veto_ratio_filtered_best: f64,
    pub events: Vec<GenericEvent>,
    pub light_curve_1s: Vec<u32>,
    pub light_curve_1s_filtered: Vec<u32>,
    pub light_curve_100ms: Vec<u32>,
    pub light_curve_100ms_filtered: Vec<u32>,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
    pub orbit: Vec<Location>,
    pub lightnings: Vec<LightningAssociation>,
    pub associated_lightning_count: u32,
    pub coincidence_probability: f64,
}
