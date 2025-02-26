mod detector;
mod event;
mod file;
mod hour;
mod time;

pub(crate) use detector::Detector;
pub(crate) use event::Event;
pub(crate) use hour::Hour;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub(crate) struct Fermi;
