use crate::{
    io::{
        level_1b::{SciFile, get_eng_filenames, get_sci_filenames, read_stime_offset},
        level_1k::{AttFile, EventFile, OrbitFile},
    },
    types::HxmtHe,
};

use super::Chunk;
use blink_core::{error::Error, types::MissionElapsedTime};
use chrono::{TimeDelta, prelude::*};

pub(super) fn from_epoch(epoch: &DateTime<Utc>) -> Result<Chunk, Error> {
    let event_file = EventFile::from_epoch(epoch)?;
    let orbit_file = OrbitFile::from_epoch(epoch)?;
    let att_file = AttFile::from_epoch(epoch)?;

    let sci_filenames = get_sci_filenames(*epoch)?;
    let sci_files = [
        SciFile::new(&sci_filenames[0])?,
        SciFile::new(&sci_filenames[1])?,
        SciFile::new(&sci_filenames[2])?,
    ];

    let eng_filenames = get_eng_filenames(*epoch)?;
    let stime_offsets = [
        read_stime_offset(&eng_filenames[0])?,
        read_stime_offset(&eng_filenames[1])?,
        read_stime_offset(&eng_filenames[2])?,
    ];

    Ok(Chunk {
        event_file,
        sci_files,
        stime_offsets,
        orbit_file,
        att_file,
        span: [
            MissionElapsedTime::<HxmtHe>::from(*epoch),
            MissionElapsedTime::<HxmtHe>::from(*epoch + TimeDelta::hours(1)),
        ],
    })
}
