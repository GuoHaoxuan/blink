use serde::Serialize;

use super::{Ebounds, Time, Satellite};

pub(crate) trait Event: Serialize {
    type Satellite: Satellite;

    fn time(&self) -> Time<Self::Satellite>;
    fn to_general(&self, ebounds: &Ebounds) -> GeneralEvent;
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GeneralEvent {
    pub(crate) time: hifitime::Epoch,
    pub(crate) energy: [f64; 2],
    pub(crate) detector: String,
}
