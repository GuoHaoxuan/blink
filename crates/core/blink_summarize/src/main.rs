use std::thread::sleep;

use blink_core::traits::Satellite;
use blink_hxmt_he::types::Hxmt;
use blink_task::process;
use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

fn main() {
    process::<Hxmt, _, _>(None, None, process_day, 1, 0);
}

fn process_day(day: NaiveDate, _multi_progress: &MultiProgress) {
    // println!("Processing day: {}", day);
    sleep(std::time::Duration::from_millis(1));
}
