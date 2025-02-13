use super::event::Event;
use super::interval::Interval;
use super::poisson::poisson_isf_cached;
use itertools::Itertools;
use statrs::distribution::{Discrete, DiscreteCDF, Poisson};
use statrs::prec;
use statrs::statistics::Distribution;

use hifitime::prelude::*;

pub struct SearchConfig {
    pub max_duration: Duration,
    pub neighbor: Duration,
    pub fp_year: f64,
    pub detector_weight: Box<dyn Fn(u8) -> u32>,
    pub min_detector: u32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_duration: 1.0.milliseconds(),
            neighbor: 1.0.seconds(),
            fp_year: 10000.0,
            detector_weight: Box::new(|_| 1),
            min_detector: 3,
        }
    }
}

pub fn search(
    data: &[Event],
    detector_count: usize,
    start: Epoch,
    stop: Epoch,
    config: SearchConfig,
) -> Vec<Interval> {
    let mut result: Vec<Interval> = Vec::new();
    let mut cache = vec![vec![None; 10]; 10000];

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
    average_counts_base[data[cursor].detector as usize] = 1;
    while average_stop_base + 1 < data.len()
        && data[average_stop_base + 1].time - data[cursor].time < config.neighbor / 2
    {
        average_stop_base += 1;
        average_counts_base[data[average_stop_base].detector as usize] += 1;
    }

    loop {
        let mut step = 0;
        let mut counts: Vec<u32> = vec![0; detector_count];
        counts[data[cursor].detector as usize] = 1;
        let mut average_stop = average_stop_base;
        let mut average_counts = average_counts_base.clone();

        loop {
            let duration = data[cursor + step].time - data[cursor].time;
            let average_start_time = (data[cursor].time - config.neighbor / 2).max(start);
            let average_stop_time = (data[cursor + step].time + config.neighbor / 2).min(stop);
            let average_duration = (average_stop_time - average_start_time) - duration;
            let average_percent = duration.to_seconds() / average_duration.to_seconds();
            let threshold = 1.0 - config.fp_year / (3600.0 * 24.0 * 365.0 / duration.to_seconds());
            if (0..detector_count)
                .map(|detector| {
                    let detector_weight = (config.detector_weight)(detector as u8);
                    let count = counts[detector];
                    let average_count = average_counts[detector] - count;
                    let average = average_count as f64 * average_percent;
                    let prob = if count == 0 {
                        0.0
                    } else if average >= 10.0 || count >= 10 {
                        match Poisson::new(average) {
                            Ok(poisson) => poisson.cdf(count as u64),
                            Err(_) => 1.0,
                        }
                    } else {
                        let average_hash = (average * 1000.0).round() as usize;
                        match cache[average_hash][count as usize] {
                            None => {
                                let prob = match Poisson::new(average) {
                                    Ok(poisson) => poisson.cdf(count as u64),
                                    Err(_) => 1.0,
                                };
                                cache[average_hash][count as usize] = Some(prob);
                                prob
                            }
                            Some(prob) => prob,
                        }
                    };
                    (prob, detector_weight)
                })
                .sorted_by(|a, b| b.0.partial_cmp(&a.0).unwrap())
                .reduce(|(prob, weight), (prob2, weight2)| {
                    if weight < config.min_detector {
                        (prob * prob2, weight + weight2)
                    } else {
                        (1.0 - (1.0 - prob) * (1.0 - prob2), weight + weight2)
                    }
                })
                .unwrap()
                .0
                > threshold
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
                || data[cursor + step].time - data[cursor].time >= config.max_duration
                || data[cursor + step].time >= stop
            {
                break;
            }

            counts[data[cursor + step].detector as usize] += 1;
            while average_stop + 1 < data.len()
                && data[average_stop + 1].time - data[cursor + step].time < config.neighbor / 2
            {
                average_stop += 1;
                average_counts[data[average_stop].detector as usize] += 1;
            }
        }

        cursor += 1;

        if cursor >= data.len() || data[cursor].time >= stop {
            break;
        }

        while average_start_base + 1 < data.len()
            && data[cursor].time - data[average_start_base + 1].time > config.neighbor / 2
        {
            average_counts_base[data[average_start_base].detector as usize] -= 1;
            average_start_base += 1;
        }
        while average_stop_base + 1 < data.len()
            && data[average_stop_base + 1].time - data[cursor].time < config.neighbor / 2
        {
            average_stop_base += 1;
            average_counts_base[data[average_stop_base].detector as usize] += 1;
        }
    }

    result
}
