use itertools::Itertools;
use serde::Serialize;
use statrs::distribution::{DiscreteCDF, Poisson};
use std::fmt::Debug;

use crate::types::{Satellite, Span, Time};

#[derive(Clone, Debug, Serialize)]
pub struct Trigger<S: Satellite> {
    pub start: Time<S>,
    pub stop: Time<S>,
    pub bin_size_min: Span<S>,
    pub bin_size_max: Span<S>,
    pub bin_size_best: Span<S>,
    pub delay: Span<S>,
    pub count: u32,
    pub mean: f64,
}

impl<S: Satellite> Trigger<S> {
    pub fn new(start: Time<S>, stop: Time<S>, count: u32, mean: f64) -> Trigger<S> {
        let bin_size = stop - start;
        Trigger {
            start,
            stop,
            bin_size_min: bin_size,
            bin_size_max: bin_size,
            bin_size_best: bin_size,
            delay: Span::seconds(0.0),
            count,
            mean,
        }
    }

    pub fn sf(&self) -> f64 {
        match (self.mean, self.count) {
            (0.0, 0) => 0.0,
            (0.0, _) => 1.0,
            _ => Poisson::new(self.mean)
                .inspect_err(|e| {
                    eprintln!("Error in Poisson distribution: {}", e);
                    eprintln!("Mean: {}", self.mean);
                })
                .unwrap()
                .sf(self.count as u64),
        }
    }

    pub fn fp_year(&self) -> f64 {
        self.sf() * (Span::seconds(3600.0) * 24.0 * 365.0 / (self.stop - self.start))
    }

    pub fn mergeable(&self, other: &Self, vision: f64) -> bool {
        self.stop + self.bin_size_max.max(other.bin_size_max) * vision >= other.start
    }
    pub fn merge(&self, other: &Self) -> Self {
        let mut res = self.clone();
        res = Trigger {
            stop: res.stop.max(other.stop),
            bin_size_min: res.bin_size_min.min(other.bin_size_min),
            bin_size_max: res.bin_size_max.max(other.bin_size_max),
            ..res
        };
        if other.sf() < res.sf() {
            res = Trigger {
                count: other.count,
                mean: other.mean,
                bin_size_best: other.bin_size_best,
                delay: other.start - res.start,
                ..res
            };
        }
        res
    }
}

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
                fp_year / (Span::seconds(3600.0) * 24.0 * 365.0 / bin_size),
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

pub fn prefix_sum(light_curve: &[u32]) -> Vec<u32> {
    light_curve
        .iter()
        .scan(0, |state, &x| {
            *state += x;
            Some(*state)
        })
        .collect()
}
