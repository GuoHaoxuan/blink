use super::{Epoch, Satellite};

pub(crate) trait Event {
    type Satellite: Satellite;

    fn time(&self) -> Epoch<Self::Satellite>;
}

pub(crate) trait Group {
    fn group(&self) -> u8;
}
