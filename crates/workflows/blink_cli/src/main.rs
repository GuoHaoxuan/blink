// use blink_filter::run;
use blink_core::{traits::Chunk as _, types::MissionElapsedTime};
use blink_hxmt_he::types::{Chunk, HxmtHe};
use chrono::prelude::*;
use clap::Parser;

fn main() {
    let start = "2022-10-09T13:20:00.000Z".parse::<DateTime<Utc>>().unwrap();
    let stop = "2022-10-09T13:22:00.000Z".parse::<DateTime<Utc>>().unwrap();
    let chunk = Chunk::from_epoch(&start).unwrap();
    // check for every 0.1s
    let mut cursor = start;
    while cursor < stop {
        let met: MissionElapsedTime<HxmtHe> = cursor.into();
        let saturated = chunk.check_saturation(met);
        println!("{saturated}");
        let next = cursor + chrono::Duration::milliseconds(100);
        cursor = next;
    }
}
