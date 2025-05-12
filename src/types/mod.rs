mod ebounds;
mod event;
mod group;
mod satellite;
mod signal;
mod time;

pub use ebounds::Ebounds;
pub use event::{Event, GenericEvent};
pub use group::Group;
pub use satellite::Satellite;
pub use signal::{Location, Signal};
pub use time::{Span, Time};

use anyhow::Result;
use chrono::prelude::*;

pub trait Instance {
    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self>
    where
        Self: Sized;
    fn search(&self) -> Result<Vec<Signal>>;
}
