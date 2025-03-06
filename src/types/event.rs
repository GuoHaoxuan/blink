use serde::Serialize;

use super::{Ebounds, Group, Satellite, Time};

pub(crate) trait Event: Serialize + Eq + Ord + Copy + Group {
    type Satellite: Satellite;
    type EnergyType;
    // type DetectorType;

    fn time(&self) -> Time<Self::Satellite>;
    fn energy(&self) -> Self::EnergyType;
    // fn detector(&self) -> Self::DetectorType;
    fn to_general(&self, ebounds: &Ebounds) -> GenericEvent;
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GenericEvent {
    pub(crate) time: hifitime::Epoch,
    pub(crate) energy: [f64; 2],
    pub(crate) detector: String,
}
