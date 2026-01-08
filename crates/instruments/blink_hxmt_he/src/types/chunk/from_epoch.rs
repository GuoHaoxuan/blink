use crate::{
    io::{
        level_1b::{EngFile, SciFile, get_all_filenames},
        level_1k::{AttFile, EventFile, OrbitFile},
    },
    types::Hxmt,
};

use super::Chunk;
use blink_core::{error::Error, types::MissionElapsedTime};
use chrono::{TimeDelta, prelude::*};

pub(super) fn from_epoch(epoch: &DateTime<Utc>) -> Result<Chunk, Error> {
    let event_file = EventFile::from_epoch(epoch)?;
    let orbit_file = OrbitFile::from_epoch(epoch)?;
    let att_file = AttFile::from_epoch(epoch)?;

    let [eng_files, sci_files] = get_all_filenames(*epoch)?;
    let eng_files = [
        EngFile::new(&eng_files[0])?,
        EngFile::new(&eng_files[1])?,
        EngFile::new(&eng_files[2])?,
    ];
    let sci_files = [
        SciFile::new(&sci_files[0])?,
        SciFile::new(&sci_files[1])?,
        SciFile::new(&sci_files[2])?,
    ];

    Ok(Chunk {
        event_file,
        eng_files,
        sci_files,
        orbit_file,
        att_file,
        span: [
            MissionElapsedTime::<Hxmt>::from(*epoch),
            MissionElapsedTime::<Hxmt>::from(*epoch + TimeDelta::hours(1)),
        ],
    })
}
