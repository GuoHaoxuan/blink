use chrono::{DateTime, Utc};

use super::Event;

pub trait Satellite: Ord + Copy {
    type Event: Event<Satellite = Self>;

    fn ref_time() -> &'static DateTime<Utc>;
}
