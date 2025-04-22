mod detector;
mod event;
mod instance;
pub mod saturation;
mod time;

use serde::Serialize;

pub(crate) use instance::Instance;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub(crate) struct Hxmt;
