use serde::Serialize;

use super::{Epoch, Satellite};

pub(crate) trait Event: Serialize {
    type Satellite: Satellite<Event = Self>;

    fn time(&self) -> Epoch<Self::Satellite>;
}

pub(crate) trait Group {
    fn group(&self) -> u8;
}
