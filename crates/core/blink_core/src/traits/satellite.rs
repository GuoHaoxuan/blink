use chrono::prelude::*;

pub trait Satellite: Clone + Copy + PartialEq + Eq + PartialOrd + Ord {
    type Chunk: crate::traits::Chunk;

    fn ref_time() -> &'static DateTime<Utc>;
    fn launch_day() -> NaiveDate;
    fn name() -> &'static str;
}
