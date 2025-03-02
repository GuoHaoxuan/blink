mod detector;
mod event;
mod file;
mod hour;
mod time;

use serde::Serialize;

pub(crate) use detector::Detector;
pub(crate) use event::Event;
pub(crate) use hour::Hour;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub(crate) struct Fermi;
