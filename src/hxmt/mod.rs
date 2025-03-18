mod detector;
mod event;
mod instance;
mod time;

use serde::Serialize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub(crate) struct Hxmt;
