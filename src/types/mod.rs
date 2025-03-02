mod event;
mod interval;
mod signal;
mod time;

pub(crate) use event::{Event, Group};
pub(crate) use interval::Interval;
pub(crate) use signal::Signal;
pub(crate) use time::{Duration, Epoch, TimeUnits};

pub(crate) trait Satellite: Ord + Copy {
    fn ref_time() -> &'static hifitime::Epoch;
}
