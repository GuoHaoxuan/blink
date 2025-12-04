use crate::error::Error;
use crate::traits::Satellite;
use crate::types::Signal;
use chrono::prelude::*;

pub trait Chunk {
    type Satellite: Satellite;

    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self, Error>
    where
        Self: Sized;
    fn search(&self) -> Result<Vec<Signal>, Error>;
}
