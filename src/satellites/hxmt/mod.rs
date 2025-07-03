mod detector;
pub mod ec;
mod event;
mod instance;
pub mod saturation;
mod time;

use serde::Serialize;

pub use detector::{HxmtDetectorType, HxmtScintillator};
pub use instance::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub struct Hxmt;
