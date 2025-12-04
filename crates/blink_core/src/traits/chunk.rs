use crate::error::Error;
use crate::traits::Event;
use crate::types::Signal;
use chrono::prelude::*;

pub trait Chunk {
    type E: Event;

    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self, Error>
    where
        Self: Sized;
    fn search(&self) -> Vec<Signal<Self::E>>;
}
