use chrono::{DateTime, Utc};
use serde::Serialize;

use super::{Group, Satellite, Time};

pub(crate) trait Event: Serialize + Eq + Ord + Copy + Group {
    type Satellite: Satellite;
    type EnergyType;
    // type DetectorType;

    fn time(&self) -> Time<Self::Satellite>;
    fn energy(&self) -> Self::EnergyType;
    // fn detector(&self) -> Self::DetectorType;
    fn to_general(&self, ec_function: impl Fn(&Self) -> [f64; 2]) -> GenericEvent;
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GenericEvent {
    pub(crate) time: DateTime<Utc>,
    pub(crate) energy: [f64; 2],
    pub(crate) detector: serde_json::Value,
}
