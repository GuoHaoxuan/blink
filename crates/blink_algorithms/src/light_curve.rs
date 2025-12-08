use chrono::TimeDelta;
use chrono::prelude::*;

pub fn light_curve_chrono(
    time: &[DateTime<Utc>],
    start: DateTime<Utc>,
    stop: DateTime<Utc>,
    bin_size: TimeDelta,
) -> Vec<u32> {
    let length = ((stop - start).num_nanoseconds().unwrap() as f64
        / bin_size.num_nanoseconds().unwrap() as f64)
        .ceil() as usize;
    let mut light_curve = vec![0; length];
    time.iter().for_each(|&time| {
        if time >= start && time < stop {
            let index = ((time - start).num_nanoseconds().unwrap() as f64
                / bin_size.num_nanoseconds().unwrap() as f64)
                .floor() as usize;
            light_curve[index] += 1;
        }
    });
    light_curve
}
