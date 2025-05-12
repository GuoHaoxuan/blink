use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Group, Satellite, Time};

pub trait Event: Serialize + Eq + Ord + Copy + Group {
    type Satellite: Satellite;
    type EnergyType;
    // type DetectorType;

    fn time(&self) -> Time<Self::Satellite>;
    fn energy(&self) -> Self::EnergyType;
    // fn detector(&self) -> Self::DetectorType;
    fn to_general(&self, ec_function: impl Fn(&Self) -> [f64; 2]) -> GenericEvent;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent {
    pub time: DateTime<Utc>,
    pub energy: [f64; 2],
    pub detector: serde_json::Value,
}
