use itertools::Itertools;
use statrs::distribution::{DiscreteCDF, Poisson};
use std::fmt::Debug;

use crate::types::{Satellite, Span, Time};

#[derive(Clone, Debug)]
pub struct Trigger {
    pub start: f64,
    pub stop: f64,
    pub bin_size_min: f64,
    pub bin_size_max: f64,
    pub bin_size_best: f64,
    pub delay: f64,
    pub count: u32,
    pub average: f64,
    pub fp_year: f64,
}

impl Trigger {
    pub fn new(start: f64, stop: f64, count: u32, average: f64, fp_year: f64) -> Trigger {
        let bin_size = stop - start;
        Trigger {
            start,
            stop,
            bin_size_min: bin_size,
            bin_size_max: bin_size,
            bin_size_best: bin_size,
            delay: 0.0,
            count,
            average,
            fp_year,
        }
    }

    pub fn sf(&self) -> f64 {
        Poisson::new(self.average).unwrap().sf(self.count as u64)
    }

    pub fn mergeable(&self, other: &Trigger, vision: u32) -> bool {
        self.stop + self.bin_size_max.max(other.bin_size_max) * (vision as f64) > other.start
    }

    pub fn merge(&self, other: &Trigger) -> Trigger {
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
                average: other.average,
                bin_size_best: other.bin_size_best,
                fp_year: other.fp_year,
                delay: other.start - res.start,
                ..res
            };
        }
        res
    }

    pub fn threshold(&self) -> u32 {
        poisson_isf(
            self.fp_year / (3600.0 * 24.0 * 365.0 / (self.stop - self.start)),
            self.average,
        )
    }
}

pub fn search_light_curve(
    light_curve_prefix_sum: &[u32],
    start: f64,
    bin_size: f64,
    num_neighbors: usize,
    fp_year: f64,
    min_count: u32,
) -> Vec<Trigger> {
    let mut cache = vec![0; 100_000];

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
            let average = (light_curve_prefix_sum[index_stop] - light_curve_prefix_sum[index_start])
                as f64
                / (index_stop - index_start + 1) as f64;
            let threshold = poisson_isf_cached(
                fp_year / (3600.0 * 24.0 * 365.0 / bin_size),
                average,
                &mut cache,
            );
            if count != 0 && count >= threshold {
                Some(Trigger::new(
                    start + i as f64 * bin_size,
                    start + (i + 1) as f64 * bin_size,
                    count,
                    average,
                    fp_year,
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

#[derive(Debug)]
pub struct Record {
    pub start: Epoch,
    pub stop: Epoch,
    pub bin_size_min: Duration,
    pub bin_size_max: Duration,
    pub bin_size_best: Duration,
    pub delay: Duration,
    pub count: u32,
    pub average: f64,
    pub fp_year: f64,
}

impl Record {
    pub fn new(trigger: &Trigger, start: Epoch) -> Record {
        Record {
            start: start + (trigger.start * 1e6).round().microseconds(),
            stop: start + (trigger.stop * 1e6).round().microseconds(),
            bin_size_min: (trigger.bin_size_min * 1e6).round().microseconds(),
            bin_size_max: (trigger.bin_size_max * 1e6).round().microseconds(),
            bin_size_best: (trigger.bin_size_best * 1e6).round().microseconds(),
            delay: (trigger.delay * 1e6).round().microseconds(),
            count: trigger.count,
            average: trigger.average,
            fp_year: trigger.fp_year,
        }
    }
}

pub fn calculate(filename: &str) -> Vec<Record> {
    let mut fptr = FitsFile::open(filename).unwrap();
    let events = fptr.hdu("EVENTS").unwrap();
    let start: f64 = events.read_key(&mut fptr, "TSTART").unwrap();
    let stop: f64 = events.read_key(&mut fptr, "TSTOP").unwrap();
    let date_obs: String = events.read_key(&mut fptr, "DATE-OBS").unwrap();
    let channel: Vec<u8> = events.read_col(&mut fptr, "Channel").unwrap();
    let time: Vec<_> = events
        .read_col::<f64>(&mut fptr, "Time")
        .unwrap()
        .iter()
        .zip(channel)
        .filter(|&(_, c)| c >= 38)
        .map(|(&t, _)| t - start)
        .collect();

    let mut results = Vec::new();
    let fp_year = 20.0;
    let min_count = 8;
    let mut bin_size = 10e-6;

    while bin_size < 1e-3 {
        results.extend((0..4).flat_map(|shift| {
            let shift = shift as f64 / 4.0 * bin_size;
            let bins = ((stop - start) / bin_size).ceil();
            let time_estimated_light_curve = bins / 500_000.0;
            let time_length = time.len() as f64;
            let time_estimated_direct = time_length / 50_000.0;

            if time_estimated_light_curve < time_estimated_direct {
                let lc = light_curve::light_curve(&time, shift, stop - start, bin_size);
                let prefix_sum = light_curve::prefix_sum(&lc);
                algorithms::search_light_curve(
                    &prefix_sum,
                    shift,
                    bin_size,
                    100,
                    fp_year,
                    min_count,
                )
            } else {
                algorithms::search_raw(
                    &time,
                    shift,
                    stop - start,
                    bin_size,
                    100,
                    fp_year,
                    min_count,
                )
            }
        }));
        bin_size *= 2.0;
    }
    results.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    results
        .into_iter()
        .coalesce(|prev, next| {
            if prev.mergeable(&next, 0) {
                Ok(prev.merge(&next))
            } else {
                Err((prev, next))
            }
        })
        .map(|trigger| record::Record::new(&trigger, Epoch::from_str(&date_obs).unwrap()))
        .collect()
}
