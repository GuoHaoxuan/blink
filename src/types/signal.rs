use super::{Epoch, Satellite};

#[derive(Debug)]
pub(crate) struct Signal<S: Satellite> {
    pub(crate) start: Epoch<S>,
    pub(crate) stop: Epoch<S>,
    pub(crate) events: Vec<S::Event>,
}
