// use blink_filter::run;
use blink_core::{traits::Chunk as _, types::MissionElapsedTime};
use blink_hxmt_he::types::{Chunk, HxmtHe};
use chrono::prelude::*;
use clap::Parser;

fn main() {
    let start = "2020-04-15T08:48:03.564Z".parse::<DateTime<Utc>>().unwrap();
    let stop = "2020-04-15T08:48:07.564Z".parse::<DateTime<Utc>>().unwrap();
    let chunk = Chunk::from_epoch(&start).unwrap();
    // check for every 0.1s
    let mut cursor = start;
    while cursor < stop {
        let met: MissionElapsedTime<HxmtHe> = cursor.into();
        let saturated = chunk.check_saturation(met);
        println!("{saturated}");
        let next = cursor + chrono::Duration::milliseconds(1);
        cursor = next;
    }
}
