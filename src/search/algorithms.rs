use std::f32::consts::E;
use std::str::FromStr;

use super::event::Event;
use super::interval::Interval;
use super::poisson::poisson_isf_cached;

use hifitime::prelude::*;

pub fn search(
    data: &[Event],
    detector_count: usize,
    start: Epoch,
    stop: Epoch,
    max_duration: Duration,
    neighbor: Duration,
    fp_year: f64,
    min_count: u32,
) -> Vec<Interval> {
    let mut result: Vec<Interval> = Vec::new();
    let mut cache = vec![0; 100_000];

    let mut cursor = match data.binary_search_by(|event| event.time.cmp(&start)) {
        Ok(index) => index,
        Err(index) => index,
    };

    if cursor == data.len() {
        return result;
    }

    let mut average_start_base = cursor;
    let mut average_stop_base = cursor;
    let mut average_counts_base: Vec<u32> = vec![0; detector_count];
    average_counts_base[data[cursor].detector] = 1;
    while average_stop_base + 1 < data.len()
        && data[average_stop_base + 1].time - data[cursor].time < neighbor / 2
    {
        average_stop_base += 1;
        average_counts_base[data[average_stop_base].detector] += 1;
    }

    loop {
        let mut step = 0;
        let mut counts: Vec<u32> = vec![0; detector_count];
        counts[data[cursor].detector] = 1;
        let mut average_stop = average_stop_base;
        let mut average_counts = average_counts_base.clone();

        loop {
            if (0..detector_count)
                .map(|detector| {
                    let count = counts[detector];
                    if count < min_count {
                        return false;
                    }
                    let duration = data[cursor + step].time - data[cursor].time;
                    let average_count = average_counts[detector] - count;
                    let average_start_time = (data[cursor].time - neighbor / 2).max(start);
                    let average_stop_time = (data[cursor + step].time + neighbor / 2).min(stop);
                    let average_duration = (average_stop_time - average_start_time) - duration;
                    let average_percent = duration.to_seconds() / average_duration.to_seconds();
                    let average = average_count as f64 * average_percent;
                    let threshold = poisson_isf_cached(
                        fp_year / (3600.0 * 24.0 * 365.0 / duration.to_seconds()),
                        average,
                        &mut cache,
                    );
                    // if data[cursor].time
                    //     > Epoch::from_str("2023-01-01T00:02:42.484000000 UTC").unwrap()
                    //     && data[cursor].time
                    //         < Epoch::from_str("2023-01-01T00:02:42.485000000 UTC").unwrap()
                    // {
                    //     println!(
                    //         "Now search at {} with count {} average {:.2} threshold {}",
                    //         data[cursor].time, count, average, threshold
                    //     );
                    // }
                    count as u32 >= threshold
                })
                .filter(|flag| *flag)
                .count()
                >= 1
            {
                let new_interval = Interval {
                    start: data[cursor].time,
                    stop: data[cursor + step].time,
                };
                if let Some(last) = result.last_mut() {
                    if last.stop >= new_interval.start {
                        last.stop = new_interval.stop;
                    } else {
                        result.push(new_interval);
                    }
                } else {
                    result.push(new_interval);
                }
            }

            step += 1;

            if cursor + step >= data.len()
                || data[cursor + step].time - data[cursor].time >= max_duration
                || data[cursor + step].time >= stop
            {
                break;
            }

            counts[data[cursor + step].detector] += 1;
            while average_stop + 1 < data.len()
                && data[average_stop + 1].time - data[cursor + step].time < neighbor / 2
            {
                average_stop += 1;
                average_counts[data[average_stop].detector] += 1;
            }
        }

        cursor += 1;

        if cursor >= data.len() || data[cursor].time >= stop {
            break;
        }

        while average_start_base + 1 < data.len()
            && data[cursor].time - data[average_start_base + 1].time > neighbor / 2
        {
            average_counts_base[data[average_start_base].detector] -= 1;
            average_start_base += 1;
        }
        while average_stop_base + 1 < data.len()
            && data[average_stop_base + 1].time - data[cursor].time < neighbor / 2
        {
            average_stop_base += 1;
            average_counts_base[data[average_stop_base].detector] += 1;
        }
    }

    result
}
