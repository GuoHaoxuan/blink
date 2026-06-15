use crate::io::level_1b::{SciFile, get_sci_filenames};
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
    pub sci_files: Vec<(String, SciFile)>, // (box_name, file)
    pub stime_offsets: Vec<(String, f64)>, // (box_name, offset)
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
        let sci_last_modifieds: Vec<DateTime<Utc>> = get_sci_filenames(*epoch)
            .iter()
            .map(|(_, filename)| {
                let last_modified = std::fs::metadata(filename)?.modified()?;
                let datetime: DateTime<Utc> = last_modified.into();
                Ok::<DateTime<Utc>, Error>(datetime)
            })
            .collect::<Result<Vec<DateTime<Utc>>, Error>>()?;

        let other_last_modifieds: Vec<DateTime<Utc>> = vec![
            EventFile::last_modified(epoch)?,
            OrbitFile::last_modified(epoch)?,
            AttFile::last_modified(epoch)?,
        ];

        let last_modifieds: Vec<DateTime<Utc>> = sci_last_modifieds
            .into_iter()
            .chain(other_last_modifieds)
            .collect();

        let max_last_modified = last_modifieds
            .into_iter()
            .max()
            .ok_or_else(|| Error::FileNotFound("No files found".to_string()))?;

        Ok(max_last_modified)
    }
}
