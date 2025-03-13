use chrono::prelude::*;
use serde::Serialize;

use crate::lightning::Lightning;

use super::GenericEvent;

#[derive(Debug, Serialize)]
pub(crate) struct Signal {
    pub(crate) start: DateTime<Utc>,
    pub(crate) stop: DateTime<Utc>,
    pub(crate) fp_year: f64,
    pub(crate) events: Vec<GenericEvent>,
    pub(crate) longitude: f64,
    pub(crate) latitude: f64,
    pub(crate) altitude: f64,
    pub(crate) position_debug: String,
    pub(crate) lightnings: Vec<Lightning>,
}
