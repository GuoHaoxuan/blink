use crate::error::BlinkError;
use crate::traits::Satellite;
use crate::types::Signal;
use chrono::prelude::*;

pub trait Chunk {
    type Satellite: Satellite;

    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self, BlinkError>
    where
        Self: Sized;
    fn search(&self) -> Result<Vec<Signal>, BlinkError>;
}
