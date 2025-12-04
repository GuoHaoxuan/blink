use std::{str::FromStr, sync::OnceLock};

use blink_core::traits::Satellite;
use chrono::prelude::*;

/// Hard X-ray Modulation Telescope (HXMT)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hxmt;

impl Satellite for Hxmt {
    fn ref_time() -> &'static DateTime<Utc> {
        static REF_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();
        REF_TIME
            .get_or_init(|| DateTime::<Utc>::from_str("2012-01-01T00:00:00.000000000 UTC").unwrap())
    }
}
