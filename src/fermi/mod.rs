mod detector;
mod event;
mod instance;
mod time;

use serde::Serialize;

pub use instance::Instance;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub struct Fermi;
