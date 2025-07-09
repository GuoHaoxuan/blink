use blink::satellites::hxmt::instance::Instance;
use blink::types::Instance as _;
use chrono::{TimeDelta, prelude::*};

fn main() {
    let start_time_snapshot = DateTime::parse_from_rfc3339("2025-06-01T00:00:00+00:00")
        .unwrap()
        .to_utc();
    let mut total_number: u64 = 0;
    let mut saturation_number: u64 = 0;
    for i in 0..23 {
        let mut start_time = start_time_snapshot + TimeDelta::hours(i);
        let ins = Instance::from_epoch(&start_time).unwrap();
        let time_delta = TimeDelta::milliseconds(100);
        let stop_time = start_time + TimeDelta::hours(1);
        // enum time
        let mut sa = false;
        while start_time <= stop_time {
            let time = start_time;
            let saturation = ins.check_saturation(time.into());
            if saturation {
                saturation_number += 1;
            }
            if saturation && !sa {
                println!("Saturation at: {}", time);
                sa = true;
            }
            if !saturation && sa {
                sa = false;
            }
            start_time += time_delta;
            total_number += 1;
        }
    }
    println!("Total number of checks: {}", total_number);
    println!("Total number of saturations: {}", saturation_number);
    println!(
        "Saturation percentage: {:.2}%",
        if total_number > 0 {
            saturation_number as f64 / total_number as f64 * 100.0
        } else {
            0.0
        }
    );
}
