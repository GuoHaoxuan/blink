use ordered_float::NotNan;
use serde::Serialize;

use super::{Ebounds, Epoch, Interval, Satellite};

pub(crate) trait Event: Serialize {
    type Satellite: Satellite;

    fn time(&self) -> Epoch<Self::Satellite>;
    fn to_general(&self, ebounds: &Ebounds) -> GeneralEvent;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Serialize)]
pub(crate) struct GeneralEvent {
    pub(crate) time: hifitime::Epoch,
    pub(crate) energy: Interval<NotNan<f64>>,
    pub(crate) detector: String,
}
