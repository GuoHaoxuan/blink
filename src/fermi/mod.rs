mod detector;
mod event;
mod file;
mod group;
mod time;

pub(crate) use detector::Detector;
pub(crate) use event::Event;
pub(crate) use group::Group;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub(crate) struct Fermi;
