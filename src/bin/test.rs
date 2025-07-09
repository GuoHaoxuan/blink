use blink::satellites::hxmt::instance::Instance;
use blink::types::Instance as _;
use chrono::{TimeDelta, prelude::*};

fn main() {
    let mut start_time = DateTime::parse_from_rfc3339("2025-07-06T16:45:22.626+00:00")
        .unwrap()
        .to_utc()
        - TimeDelta::seconds(10);
    let ins = Instance::from_epoch(&start_time).unwrap();
    let time_delta = TimeDelta::milliseconds(100);
    let stop_time = start_time + TimeDelta::seconds(50);
    // enum time
    while start_time <= stop_time {
        let time = start_time;
        let saturation = ins.check_saturation(time.into());
        println!("{}", saturation);
        start_time += time_delta;
    }
}
