use blink_core::types::MissionElapsedTime;

use crate::io::file::{find_att_by_time, find_evt_by_time, find_orb_by_time};
use crate::io::{AttFile, EvtFile, OrbFile};
use crate::types::event::Event;
use crate::types::satellite::Svom;
use chrono::TimeDelta;

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
        let att_filename = find_att_by_time(epoch)?;
        let att_file = AttFile::from_fits_file(att_filename.to_str().unwrap())?;
        let evt_filename = find_evt_by_time(epoch)?;
        let evt_file = EvtFile::from_fits_file(evt_filename.to_str().unwrap())?;
        let orb_filename = find_orb_by_time(epoch)?;
        let orb_file = OrbFile::from_fits_file(orb_filename.to_str().unwrap())?;
        Ok(Chunk {
            span: [
                MissionElapsedTime::<Svom>::from(*epoch),
                MissionElapsedTime::<Svom>::from(*epoch + TimeDelta::hours(1)),
            ],
            att_file,
            evt_file,
            orb_file,
        })
    }

    fn search(&self) -> Vec<blink_core::types::Signal<Self::E>> {
        unimplemented!()
    }
}
