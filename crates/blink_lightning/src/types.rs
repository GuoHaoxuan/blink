use chrono::prelude::*;

pub struct Lightning {
    pub time: DateTime<Utc>,
    pub lat: f64,
    pub lon: f64,
    pub resid: f64,
    pub nstn: u32,
    pub energy: Option<f64>,
    pub energy_uncertainty: Option<f64>,
    pub estn: Option<u32>,
}
