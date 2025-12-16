use blink_core::types::MissionElapsedTime;

use crate::io::{AttFile, EvtFile, OrbFile};
use crate::types::event::Event;
use crate::types::satellite::Svom;

mod from_epoch;
mod search;

pub struct Chunk {
    pub span: [MissionElapsedTime<Svom>; 2],
    pub att_file: AttFile,
    pub evt_file: EvtFile,
    pub orb_file: OrbFile,
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
