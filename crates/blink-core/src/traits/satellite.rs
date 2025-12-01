use crate::traits::Event;
use chrono::prelude::*;

pub trait Satellite {
    type Event: Event<Satellite = Self>;

    fn ref_time() -> &'static DateTime<Utc>;
}
