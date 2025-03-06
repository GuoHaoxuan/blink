use hifitime::prelude::*;
use nav_types::WGS84;
use serde::Serialize;

use crate::lightning::Lightning;

use super::GeneralEvent;

#[derive(Debug, Serialize)]
pub(crate) struct Signal {
    pub(crate) start: Epoch,
    pub(crate) stop: Epoch,
    pub(crate) fp_year: f64,
    pub(crate) events: Vec<GeneralEvent>,
    pub(crate) position: WGS84<f64>,
    pub(crate) lightnings: Vec<Lightning>,
}
