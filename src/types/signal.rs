use serde::Serialize;

use super::{Epoch, Event};

#[derive(Debug, Serialize)]
pub(crate) struct Signal<E: Event, P> {
    pub(crate) start: Epoch<E::Satellite>,
    pub(crate) stop: Epoch<E::Satellite>,
    pub(crate) events: Vec<E>,
    pub(crate) position: P,
}
