use blink_core::traits::Satellite;
use chrono::prelude::*;
use std::{str::FromStr, sync::OnceLock};

/// Space-based multi-band astronomical Variable Objects Monitor (SVOM)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Svom;

impl Satellite for Svom {
    fn ref_time() -> &'static DateTime<Utc> {
        static REF_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();
        REF_TIME
            .get_or_init(|| DateTime::<Utc>::from_str("2017-01-01T00:00:00.000000000 UTC").unwrap())
    }
}
