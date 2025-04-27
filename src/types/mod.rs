mod ebounds;
mod event;
mod group;
mod satellite;
mod signal;
mod time;

pub(crate) use ebounds::Ebounds;
pub(crate) use event::{Event, GenericEvent};
pub(crate) use group::Group;
pub(crate) use satellite::Satellite;
pub(crate) use signal::{Location, Signal};
pub(crate) use time::{Span, Time};

use anyhow::Result;
use chrono::prelude::*;

pub trait Instance {
    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self>
    where
        Self: Sized;
    fn search(&self) -> Result<Vec<Signal>>;
}
