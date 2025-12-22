use chrono::prelude::*;

pub trait Satellite: Clone + Copy + PartialEq + Eq + PartialOrd + Ord {
    fn ref_time() -> &'static DateTime<Utc>;
}
