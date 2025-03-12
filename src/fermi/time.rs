use std::str::FromStr;
use std::sync::OnceLock;

use chrono::prelude::*;

use crate::types::Satellite;

use super::event::FermiEvent;
use super::Fermi;

impl Satellite for Fermi {
    type Event = FermiEvent;

    fn ref_time() -> &'static DateTime<Utc> {
        static REF_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();
        REF_TIME
            .get_or_init(|| DateTime::<Utc>::from_str("2001-01-01T00:00:00.000000000 UTC").unwrap())
    }
}
