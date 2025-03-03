use serde::Serialize;

use crate::lightning::Lightning;

use super::{Epoch, Event};

#[derive(Debug, Serialize)]
pub(crate) struct Signal<E: Event, P: Serialize> {
    pub(crate) start: Epoch<E::Satellite>,
    pub(crate) stop: Epoch<E::Satellite>,
    pub(crate) fp_year: f64,
    pub(crate) events: Vec<E>,
    pub(crate) position: Option<P>,
    pub(crate) lightnings: Option<Vec<Lightning>>,
}
