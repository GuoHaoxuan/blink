mod detector;
mod event;
mod file;
mod hour;
mod position;
mod time;

use serde::Serialize;

// pub(crate) use detector::Detector;
pub(crate) use hour::Hour;
pub(crate) use position::Position;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub(crate) struct Fermi;
