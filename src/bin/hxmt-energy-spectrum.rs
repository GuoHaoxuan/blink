use blink::{hxmt::Instance, types::Instance as _};
use chrono::prelude::*;

fn main() {
    let mut result = [0u64; 256];
    let start_time = DateTime::parse_from_rfc3339("2017-06-15T00:00:00Z")
        .expect("Failed to parse start time")
        .to_utc();
    let stop_time = DateTime::parse_from_rfc3339("2024-12-31T23:59:59Z")
        .expect("Failed to parse stop time")
        .to_utc();
    let hours_count = (stop_time - start_time).num_hours();
    for (i, hour) in (0..=hours_count).enumerate() {
        let current_time = start_time + chrono::Duration::hours(hour);
        println!("Processing hour {}: {}", i, current_time.to_rfc3339());

        let instance = Instance::from_epoch(&current_time);
        if let Ok(instance) = instance {
            for channel in instance.event_file.channel {
                result[channel as usize] += 1;
            }
        }
    }
    println!("All hours processed successfully.");
    println!("Energy spectrum result: {:?}", result);
    println!("Total events: {}", result.iter().sum::<u64>());
    println!("Energy spectrum:");
    for (i, count) in result.iter().enumerate() {
        if *count > 0 {
            println!("Energy {}: {}", i, count);
        }
    }
    println!("Done.");
}
