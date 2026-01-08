use crate::io::level_1b::{EngFile, SciFile, get_all_filenames};
use crate::io::level_1k::{AttFile, EventFile, OrbitFile};
use crate::types::{Event, HxmtHe};
use blink_core::error::Error;
use blink_core::types::MissionElapsedTime;
use chrono::prelude::*;

mod check_saturation;
mod from_epoch;
mod search;

pub struct Chunk {
    pub event_file: EventFile,
    pub eng_files: [EngFile; 3],
    pub sci_files: [SciFile; 3],
    pub orbit_file: OrbitFile,
    pub att_file: AttFile,
    pub span: [MissionElapsedTime<HxmtHe>; 2],
}

impl blink_core::traits::Chunk for Chunk {
    type Event = Event;

    fn from_epoch(epoch: &chrono::DateTime<chrono::Utc>) -> Result<Self, blink_core::error::Error>
    where
        Self: Sized,
    {
        from_epoch::from_epoch(epoch)
    }

    fn search(&self) -> Vec<blink_core::types::Signal<Self::Event>> {
        search::search(self)
    }

    fn last_modified(epoch: &DateTime<Utc>) -> Result<DateTime<Utc>, Error> {
        let last_modifieds1: Vec<DateTime<Utc>> = get_all_filenames(*epoch)?
            .iter()
            .flatten()
            .map(|filename| {
                let last_modified = std::fs::metadata(filename)?.modified()?;
                let datetime: DateTime<Utc> = last_modified.into();
                Ok::<DateTime<Utc>, Error>(datetime)
            })
            .collect::<Result<Vec<DateTime<Utc>>, Error>>()?;

        let last_modifieds2: Vec<DateTime<Utc>> = vec![
            EventFile::last_modified(epoch)?,
            OrbitFile::last_modified(epoch)?,
            AttFile::last_modified(epoch)?,
        ];

        let last_modifieds: Vec<DateTime<Utc>> =
            last_modifieds1.into_iter().chain(last_modifieds2).collect();

        let max_last_modified = last_modifieds
            .into_iter()
            .max()
            .ok_or_else(|| Error::FileNotFound("No files found".to_string()))?;

        Ok(max_last_modified)
    }
}
