use anyhow::Result;
use chrono::prelude::*;

use crate::{hxmt::Hxmt, types::Time};

use super::event_file::EventFile;

pub(crate) struct Instance {
    event_file: EventFile,
    span: [Time<Hxmt>; 2],
}

impl Instance {
    pub(crate) fn new(event_file_path: &str, span: [Time<Hxmt>; 2]) -> Result<Self> {
        let event_file = EventFile::new(event_file_path)?;
        Ok(Self { event_file, span })
    }

    pub(crate) fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self> {
        let year = epoch.year();
        let month = epoch.month();
        let day = epoch.day();
        let hour = epoch.hour();
        let min = epoch.minute();
        let sec = epoch.second();
        let num = (*epoch - Utc.with_ymd_and_hms(2017, 6, 15, 0, 0, 0).unwrap()).num_days() + 1;
        let folder = format!(
            "/hxmt/work/HXMT-DATA/1K/Y{year:04}{month:02}/{year:04}{month:02}{day:02}-{num:04}",
            year = year,
            month = month,
            day = day,
            num = num
        );
    }
}
