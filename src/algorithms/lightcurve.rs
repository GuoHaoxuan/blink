use chrono::{TimeDelta, prelude::*};
use itertools::Itertools;

use super::trigger::Trigger;
use crate::{
    env::DAYS_1_YEAR,
    types::{Satellite, Span, Time},
};

pub fn search_light_curve<S: Satellite>(
    light_curve_prefix_sum: &[u32],
    start: Time<S>,
    bin_size: Span<S>,
    fp_year: f64,
    min_count: u32,
) -> Vec<Trigger<S>> {
    let mut cache = vec![0; 100_000];
    let num_neighbors = (Span::seconds(1.0) / bin_size).round() as usize;
    let num_neighbors_hollow = (Span::seconds(1e-2) / bin_size).round() as usize;

    std::iter::once(0)
        .chain(light_curve_prefix_sum.iter().cloned())
        .tuple_windows()
        .map(|(prev, next)| next - prev)
        .enumerate()
        .filter_map(|(i, count)| {
            if count < min_count {
                return None;
            }
            let index_start = (i as isize - num_neighbors as isize / 2).max(0) as usize;
            let index_stop = (i + num_neighbors / 2).min(light_curve_prefix_sum.len() - 1);
            let mean_length = (index_stop - index_start + 1) as f64;
            let mean_count =
                (light_curve_prefix_sum[index_stop] - light_curve_prefix_sum[index_start]) as f64;

            let hollow_start = (i as isize - num_neighbors_hollow as isize / 2).max(0) as usize;
            let hollow_stop = (i + num_neighbors_hollow / 2).min(light_curve_prefix_sum.len() - 1);
            let hollow_length = (hollow_stop - hollow_start + 1) as f64;
            let hollow_count =
                (light_curve_prefix_sum[hollow_stop] - light_curve_prefix_sum[hollow_start]) as f64;

            let mean = (mean_count - hollow_count) / (mean_length - hollow_length);

            let threshold = poisson_isf_cached(
                fp_year / (Span::seconds(3600.0) * 24.0 * DAYS_1_YEAR / bin_size),
                mean,
                &mut cache,
            );
            if count != 0 && count >= threshold {
                Some(Trigger::new(
                    start + bin_size * i as f64,
                    start + bin_size * (i + 1) as f64,
                    count,
                    mean,
                ))
            } else {
                None
            }
        })
        .collect()
}

pub fn poisson_isf(p: f64, lambda: f64) -> u32 {
    let mut k = 0;
    let mut cumulative_prob = (-lambda).exp();
    let mut part = 0.0;

    while cumulative_prob < 1.0 - p {
        k += 1;
        part += (lambda / k as f64).ln();
        cumulative_prob += (-lambda + part).exp();
    }

    k
}

pub fn poisson_isf_cached(p: f64, lambda: f64, cache: &mut [u32]) -> u32 {
    let lambda_100x = (lambda * 100.0).round() as usize;
    if lambda_100x == 0 {
        return 0;
    }
    if lambda_100x >= cache.len() {
        return poisson_isf(p, lambda);
    }
    if cache[lambda_100x] == 0 {
        cache[lambda_100x] = poisson_isf(p, lambda);
    }
    cache[lambda_100x]
}

pub fn light_curve<S: Satellite>(
    time: &[Time<S>],
    start: Time<S>,
    stop: Time<S>,
    bin_size: Span<S>,
) -> Vec<u32> {
    let length = ((stop - start) / bin_size).ceil() as usize;
    let mut light_curve = vec![0; length];
    time.iter().for_each(|&time| {
        if time >= start && time < stop {
            let index = ((time - start) / bin_size).floor() as usize;
            light_curve[index] += 1;
        }
    });
    light_curve
}

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

pub fn prefix_sum(light_curve: &[u32]) -> Vec<u32> {
    light_curve
        .iter()
        .scan(0, |state, &x| {
            *state += x;
            Some(*state)
        })
        .collect()
}
