use super::{Epoch, Satellite};

pub(crate) trait Event {
    type Satellite: Satellite<Event = Self>;

    fn time(&self) -> Epoch<Self::Satellite>;
}

pub(crate) trait Group {
    fn group(&self) -> u8;
}
