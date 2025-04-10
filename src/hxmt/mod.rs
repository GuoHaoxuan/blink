mod detector;
mod event;
mod geo_acd;
mod instance;
pub mod saturation;
mod time;

use serde::Serialize;

pub(crate) use geo_acd::interpolate_point;
pub(crate) use instance::Instance;
pub(crate) use instance::{EngFile, SciFile};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub(crate) struct Hxmt;
