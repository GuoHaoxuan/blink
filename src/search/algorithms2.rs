use super::poisson::poisson_isf_cached;

use hifitime::prelude::*;

#[derive(Clone)]
struct Event {
    time: Epoch,
    pi: u32,
    index: usize,
}

struct Trigger {
    start: Epoch,
    stop: Epoch,
    events: Vec<Event>,
}

fn search(
    data: Vec<Event>,
    start: Epoch,
    stop: Epoch,
    max_duration: Duration,
    neighbor: Duration,
    fp_year: f64,
    min_count: u32,
) -> Vec<Trigger> {
    let mut result = Vec::new();
    let mut cache = vec![0; 100_000];

    let mut cursor = 0;
    let mut average_start = 0;
    let mut average_stop = 0;
    while cursor < data.len() {
        let mut march = 0;
        while march < data.len() && data[cursor + march].time - data[cursor].time < max_duration {
            while average_stop + 1 < data.len()
                && data[average_stop + 1].time - data[cursor + march].time < neighbor / 2
            {
                average_stop += 1;
            }

            if march as u32 >= min_count {
                let duration = data[cursor + march].time - data[cursor].time;
                let average_count = (average_stop - average_start + 1) - march;
                let average_start_time = (data[cursor].time - neighbor / 2).max(start);
                let average_stop_time = (data[cursor + march].time + neighbor / 2).min(stop);
                let average_duration = (average_stop_time - average_start_time) - duration;
                let average_percent = duration.to_seconds() / average_duration.to_seconds();
                let average = average_count as f64 * average_percent;
                let threshold = poisson_isf_cached(
                    fp_year / (3600.0 * 24.0 * 365.0 / duration.to_seconds()),
                    average,
                    &mut cache,
                );
                if march as u32 >= threshold {
                    result.push(Trigger {
                        start: data[cursor].time,
                        stop: data[cursor + march].time,
                        events: data[cursor..cursor + march].to_vec(),
                    });
                }
            }

            march += 1;
            while data[cursor].time - data[average_start].time > neighbor / 2 {
                average_start += 1;
            }
        }
        cursor += 1;
    }

    result
}
