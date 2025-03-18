use std::str::FromStr;
use std::sync::OnceLock;

use chrono::prelude::*;

use crate::types::Satellite;

use super::event::HxmtEvent;
use super::Hxmt;

impl Satellite for Hxmt {
    type Event = HxmtEvent;

    fn ref_time() -> &'static DateTime<Utc> {
        static REF_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();
        REF_TIME
            .get_or_init(|| DateTime::<Utc>::from_str("2012-01-01T00:00:00.000000000 UTC").unwrap())
    }
}
