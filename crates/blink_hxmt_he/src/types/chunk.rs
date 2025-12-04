use crate::io::level_1b::{EngFile, SciFile};
use crate::io::level_1k::{AttFile, EventFile, OrbitFile};
use crate::types::{Event, Hxmt};
use blink_core::types::MissionElapsedTime;

mod check_saturation;
mod from_epoch;
mod search;

pub struct Chunk {
    pub event_file: EventFile,
    pub eng_files: [EngFile; 3],
    pub sci_files: [SciFile; 3],
    pub orbit_file: OrbitFile,
    pub att_file: AttFile,
    pub span: [MissionElapsedTime<Hxmt>; 2],
}

impl blink_core::traits::Chunk for Chunk {
    type E = Event;

    fn from_epoch(epoch: &chrono::DateTime<chrono::Utc>) -> Result<Self, blink_core::error::Error>
    where
        Self: Sized,
    {
        from_epoch::from_epoch(epoch)
    }

    fn search(&self) -> Vec<blink_core::types::Signal<Self::E>> {
        search::search(self)
    }
}
