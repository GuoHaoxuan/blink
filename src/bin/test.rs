use blink::satellites::hxmt::instance::Instance;
use blink::types::Instance as _;
use chrono::{TimeDelta, prelude::*};

fn main() {
    let start_time_snapshot = DateTime::parse_from_rfc3339("2025-06-01T00:00:00+00:00")
        .unwrap()
        .to_utc();
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
            if saturation && !sa {
                println!("Saturation at: {}", time);
                sa = true;
            }
            if !saturation && sa {
                sa = false;
            }
            start_time += time_delta;
        }
    }
}
