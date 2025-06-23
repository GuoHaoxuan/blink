use core::num;

use statrs::distribution::{DiscreteCDF, Poisson};

use super::trigger::Trigger;
use crate::types::{Event, Group, Satellite, Span, Time};

pub struct SearchConfig<T: Satellite> {
    pub min_duration: Span<T>,
    pub max_duration: Span<T>,
    pub neighbor: Span<T>,
    pub hollow: Span<T>,
    pub fp_year: f64,
    pub min_number: u32,
}

impl<T: Satellite> Default for SearchConfig<T> {
    fn default() -> Self {
        Self {
            min_duration: Span::microseconds(10.0),
            max_duration: Span::milliseconds(1.0),
            neighbor: Span::seconds(1.0),
            hollow: Span::milliseconds(10.0),
            fp_year: 20.0,
            min_number: 8,
        }
    }
}

fn coincidence_prob(probs: &[f64], n: usize) -> f64 {
    let mut cache = vec![0.0; n + 1];
    cache[0] = 1.0;

    for m_i in 1..=probs.len() {
        for n_i in (0..=n).rev() {
            cache[n_i] = match (m_i, n_i) {
                (_, 0) => 1.0,
                // The following line can be removed, because cache[n_i] is 0.0 initially,
                // although it is meaningless mathematically
                // (m_i, n_i) if m_i == n_i => probs[m_i - 1] * cache[n_i - 1],
                (m_i, n_i) => probs[m_i - 1] * cache[n_i - 1] + (1.0 - probs[m_i - 1]) * cache[n_i],
            }
        }
    }

    cache[n]
}

const CACHE_MEAN_MAX: f64 = 10.0;
const CACHE_COUNT_MAX: u32 = 10;
const CACHE_MEAN_HASH_FACTOR: f64 = 1000.0;

fn poisson_cdf(cache: &mut [Vec<Option<f64>>], mean: f64, count: u32) -> f64 {
    let do_calc = |mean: f64, count: u32| -> f64 {
        match Poisson::new(mean) {
            Ok(poisson) => poisson.cdf(count as u64),
            Err(_) => 1.0,
        }
    };
    if count == 0 {
        0.0
    } else if mean >= CACHE_MEAN_MAX || count >= CACHE_COUNT_MAX {
        do_calc(mean, count)
    } else {
        let mean_hash = (mean * CACHE_MEAN_HASH_FACTOR).floor() as usize;
        match cache[mean_hash][count as usize] {
            None => {
                let prob = do_calc(mean, count);
                cache[mean_hash][count as usize] = Some(prob);
                prob
            }
            Some(prob) => prob,
        }
    }
}

pub fn search<E: Event + Group>(
    data: &[E],
    group_count: usize,
    start: Time<E::Satellite>,
    stop: Time<E::Satellite>,
    config: SearchConfig<E::Satellite>,
) -> Vec<([Time<E::Satellite>; 2], f64)> {
    let mut result: Vec<([Time<E::Satellite>; 2], f64)> = Vec::new();
    let mut cache = vec![
        vec![None; CACHE_COUNT_MAX as usize];
        (CACHE_MEAN_MAX * CACHE_MEAN_HASH_FACTOR).ceil() as usize
    ];

    let mut cursor = data
        .binary_search_by(|event| event.time().cmp(&start))
        .unwrap_or_else(|index| index);

    if cursor == data.len() {
        return result;
    }

    let mut mean_start_base = cursor;
    let mut mean_stop_base = cursor;
    let mut mean_counts_base: Vec<u32> = vec![0; group_count];
    mean_counts_base[data[cursor].group() as usize] = 1;
    while mean_stop_base + 1 < data.len()
        && data[mean_stop_base + 1].time() - data[cursor].time() < config.neighbor / 2.0
    {
        mean_stop_base += 1;
        mean_counts_base[data[mean_stop_base].group() as usize] += 1;
    }

    loop {
        let mut step = 0;
        let mut counts: Vec<u32> = vec![0; group_count];
        counts[data[cursor].group() as usize] = 1;
        let mut mean_stop = mean_stop_base;
        let mut mean_counts = mean_counts_base.clone();

        loop {
            let duration = data[cursor + step].time() - data[cursor].time();
            let mean_start_time = (data[cursor].time() - config.neighbor / 2.0).max(start);
            let mean_stop_time = (data[cursor + step].time() + config.neighbor / 2.0).min(stop);
            let mean_duration = (mean_stop_time - mean_start_time) - duration;
            let mean_percent = duration.to_seconds() / mean_duration.to_seconds();
            let threshold = 1.0 - config.fp_year / (3600.0 * 24.0 * 365.0 / duration.to_seconds());
            let probs = (0..group_count)
                .map(|group| {
                    let count = counts[group];
                    let mean_count = mean_counts[group] - count;
                    let mean = mean_count as f64 * mean_percent;

                    poisson_cdf(&mut cache, mean, count)
                })
                .collect::<Vec<f64>>();
            let prob = coincidence_prob(&probs, 3);
            if prob > threshold {
                let fp_year_real = (1.0 - prob) * 3600.0 * 24.0 * 365.0 / duration.to_seconds();
                let new_interval = [data[cursor].time(), data[cursor + step].time()];
                if let Some(last) = result.last_mut() {
                    if last.0[1] >= new_interval[0] {
                        last.0[1] = new_interval[1];
                        last.1 = last.1.min(fp_year_real);
                    } else {
                        result.push((new_interval, fp_year_real));
                    }
                } else {
                    result.push((new_interval, fp_year_real));
                }
            }

            step += 1;

            if cursor + step >= data.len()
                || data[cursor + step].time() - data[cursor].time() >= config.max_duration
                || data[cursor + step].time() >= stop
            {
                break;
            }

            counts[data[cursor + step].group() as usize] += 1;
            while mean_stop + 1 < data.len()
                && data[mean_stop + 1].time() - data[cursor + step].time() < config.neighbor / 2.0
            {
                mean_stop += 1;
                mean_counts[data[mean_stop].group() as usize] += 1;
            }
        }

        cursor += 1;

        if cursor >= data.len() || data[cursor].time() >= stop {
            break;
        }

        while mean_start_base + 1 < data.len()
            && data[cursor].time() - data[mean_start_base + 1].time() > config.neighbor / 2.0
        {
            mean_counts_base[data[mean_start_base].group() as usize] -= 1;
            mean_start_base += 1;
        }
        while mean_stop_base + 1 < data.len()
            && data[mean_stop_base + 1].time() - data[cursor].time() < config.neighbor / 2.0
        {
            mean_stop_base += 1;
            mean_counts_base[data[mean_stop_base].group() as usize] += 1;
        }
    }

    result
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

pub fn search_new<E: Event + Group>(
    data: &[E],
    group_number: usize,
    start: Time<E::Satellite>,
    stop: Time<E::Satellite>,
    config: SearchConfig<E::Satellite>,
) -> Vec<Trigger<E::Satellite>> {
    let mut result: Vec<Trigger<E::Satellite>> = Vec::new();
    // let mut cache = vec![
    //     vec![None; CACHE_COUNT_MAX as usize];
    //     (CACHE_MEAN_MAX * CACHE_MEAN_HASH_FACTOR).ceil() as usize
    // ];
    // let mut cache = vec![0; 100_000];

    let mut cursor = data
        .binary_search_by(|event| event.time().cmp(&start))
        .unwrap_or_else(|index| index);
    if cursor == data.len() {
        return result;
    }

    let mut mean_start_snapshot = cursor;
    let mut mean_stop_snapshot = cursor;
    let mut mean_number_snapshot: Vec<u32> = vec![0; group_number];
    mean_number_snapshot[data[cursor].group() as usize] = 1;
    while mean_stop_snapshot < data.len()
        && data[mean_stop_snapshot].time() - data[cursor].time() < config.neighbor / 2.0
    {
        mean_stop_snapshot += 1;
        mean_number_snapshot[data[mean_stop_snapshot].group() as usize] += 1;
    }

    let mut hollow_start_snapshot = cursor;
    let mut hollow_stop_snapshot = cursor;
    let mut hollow_number_snapshot: Vec<u32> = vec![0; group_number];
    hollow_number_snapshot[data[cursor].group() as usize] = 1;
    while hollow_stop_snapshot < data.len()
        && data[hollow_stop_snapshot].time() - data[cursor].time() < config.hollow / 2.0
    {
        hollow_stop_snapshot += 1;
        hollow_number_snapshot[data[hollow_stop_snapshot].group() as usize] += 1;
    }

    loop {
        let mut step = 0;
        let mut numbers: Vec<u32> = vec![0; group_number];
        numbers[data[cursor].group() as usize] = 1;
        let mut mean_stop = mean_stop_snapshot;
        let mut mean_numbers = mean_number_snapshot.clone();
        let mut hollow_stop = hollow_stop_snapshot;
        let mut hollow_numbers = hollow_number_snapshot.clone();

        loop {
            let total_number = numbers.iter().sum(); // [TODO] Use real total number calculation
            let duration = data[cursor + step].time() - data[cursor].time();
            if total_number >= config.min_number && duration >= config.min_duration {
                let mean_start_time = (data[cursor].time() - config.neighbor / 2.0).max(start);
                let mean_stop_time = (data[cursor + step].time() + config.neighbor / 2.0).min(stop);
                let hollow_start_time = (data[cursor].time() - config.hollow / 2.0).max(start);
                let hollow_stop_time = (data[cursor + step].time() + config.hollow / 2.0).min(stop);
                let pure_mean_duration =
                    (mean_stop_time - mean_start_time) - (hollow_stop_time - hollow_start_time);
                let pure_mean_percent = duration.to_seconds() / pure_mean_duration.to_seconds();
                let fps = (0..group_number)
                    .map(|group| {
                        let pure_mean_number = mean_numbers[group] - hollow_numbers[group];
                        let equivalent_background_number =
                            pure_mean_number as f64 * pure_mean_percent;
                        Poisson::new(equivalent_background_number)
                            .unwrap()
                            .sf(numbers[group] as u64)
                    })
                    .collect::<Vec<f64>>();
                let fp = fps[0];
                let threshold = config.fp_year / (Span::seconds(3600.0) * 24.0 * 365.0 / duration);
                if fp < threshold {
                    let total_equivalent_background_number = (0..group_number)
                        .map(|group| mean_numbers[group] - hollow_numbers[group])
                        .sum::<u32>()
                        as f64
                        * pure_mean_percent;
                    // println!(
                    //     "Found trigger: total_number: {}, equivalent_background_number: {}, fp: {}, threshold: {}, duration: {}",
                    //     total_number,
                    //     total_equivalent_background_number,
                    //     fp,
                    //     threshold,
                    //     duration.to_seconds() * 1e6
                    // );
                    let current = Trigger::new(
                        data[cursor].time(),
                        data[cursor + step].time(),
                        total_number,
                        total_equivalent_background_number,
                    );
                    if let Some(last) = result.last_mut() {
                        if last.mergeable(&current, 0.0) {
                            *last = last.merge(&current);
                        } else {
                            result.push(current);
                        }
                    } else {
                        result.push(current);
                    }
                }
            }

            step += 1;
            if cursor + step >= data.len()
                || data[cursor + step].time() - data[cursor].time() >= config.max_duration
                || data[cursor + step].time() >= stop
            {
                break;
            }
            numbers[data[cursor + step].group() as usize] += 1;
            while mean_stop + 1 < data.len()
                && data[mean_stop + 1].time() - data[cursor + step].time() < config.neighbor / 2.0
            {
                mean_stop += 1;
                mean_numbers[data[mean_stop].group() as usize] += 1;
            }
            while hollow_stop + 1 < data.len()
                && data[hollow_stop + 1].time() - data[cursor + step].time() < config.hollow / 2.0
            {
                hollow_stop += 1;
                hollow_numbers[data[hollow_stop].group() as usize] += 1;
            }
        }

        cursor += 1;
        if cursor >= data.len() || data[cursor].time() >= stop {
            break;
        }
        while mean_start_snapshot + 1 < data.len()
            && data[cursor].time() - data[mean_start_snapshot + 1].time() > config.neighbor / 2.0
        {
            mean_numbers[data[mean_start_snapshot].group() as usize] -= 1;
            mean_start_snapshot += 1;
        }
        while mean_stop_snapshot + 1 < data.len()
            && data[mean_stop_snapshot + 1].time() - data[cursor].time() < config.neighbor / 2.0
        {
            mean_stop_snapshot += 1;
            mean_numbers[data[mean_stop_snapshot].group() as usize] += 1;
        }
        while hollow_start_snapshot + 1 < data.len()
            && data[cursor].time() - data[hollow_start_snapshot + 1].time() > config.hollow / 2.0
        {
            hollow_numbers[data[hollow_start_snapshot].group() as usize] -= 1;
            hollow_start_snapshot += 1;
        }
        while hollow_stop_snapshot + 1 < data.len()
            && data[hollow_stop_snapshot + 1].time() - data[cursor].time() < config.hollow / 2.0
        {
            hollow_stop_snapshot += 1;
            hollow_numbers[data[hollow_stop_snapshot].group() as usize] += 1;
        }
    }

    result
}
