use serde::Serialize;

use super::{Epoch, Satellite};

pub(crate) trait Event: Serialize {
    type Satellite: Satellite;

    fn time(&self) -> Epoch<Self::Satellite>;
}

pub(crate) trait Group {
    fn group(&self) -> u8;
}
