use chrono::prelude::*;
use nav_types::WGS84;
use serde::Serialize;

use crate::lightning::Lightning;

use super::GenericEvent;

#[derive(Debug, Serialize)]
pub(crate) struct Signal {
    pub(crate) start: DateTime<Utc>,
    pub(crate) stop: DateTime<Utc>,
    pub(crate) fp_year: f64,
    pub(crate) events: Vec<GenericEvent>,
    pub(crate) position: WGS84<f64>,
    pub(crate) position_debug: String,
    pub(crate) lightnings: Vec<Lightning>,
}
