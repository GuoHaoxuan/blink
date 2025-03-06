mod ebounds;
mod event;
mod group;
mod signal;
mod time;

pub(crate) use ebounds::Ebounds;
pub(crate) use event::{Event, GeneralEvent};
pub(crate) use group::Group;
pub(crate) use signal::Signal;
pub(crate) use time::{Duration, Epoch, TimeUnits};

pub(crate) trait Satellite: Ord + Copy {
    fn ref_time() -> &'static hifitime::Epoch;
}
