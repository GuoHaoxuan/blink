use crate::{constants::DAYS_PER_YEAR, types::candidate::Candidate};
use blink_core::{traits::Event, types::MissionElapsedTime};
use statrs::distribution::{DiscreteCDF, Poisson};
use uom::si::f64::*;

pub struct SearchConfig {
    pub min_duration: Time,
    pub max_duration: Time,
    pub neighbor: Time,
    pub hollow: Time,
    pub false_positive_per_year: f64,
    pub min_number: u32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            min_duration: Time::new::<uom::si::time::microsecond>(10.0),
            max_duration: Time::new::<uom::si::time::millisecond>(1.0),
            neighbor: Time::new::<uom::si::time::second>(1.0),
            hollow: Time::new::<uom::si::time::millisecond>(10.0),
            false_positive_per_year: 20.0,
            min_number: 8,
        }
    }
}

// fn coincidence_prob(probs: &[f64], n: usize) -> f64 {
//     let mut cache = vec![0.0; n + 1];
//     cache[0] = 1.0;

//     for m_i in 1..=probs.len() {
//         for n_i in (0..=n).rev() {
//             cache[n_i] = match (m_i, n_i) {
//                 (_, 0) => 1.0,
//                 // The following line can be removed, because cache[n_i] is 0.0 initially,
//                 // although it is meaningless mathematically
//                 // (m_i, n_i) if m_i == n_i => probs[m_i - 1] * cache[n_i - 1],
//                 (m_i, n_i) => probs[m_i - 1] * cache[n_i - 1] + (1.0 - probs[m_i - 1]) * cache[n_i],
//             }
//         }
//     }

//     cache[n]
// }

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

pub fn search_new<E: Event>(
    data: &[E],
    group_number: usize,
    start: MissionElapsedTime<E::Satellite>,
    stop: MissionElapsedTime<E::Satellite>,
    config: SearchConfig,
) -> Vec<Candidate<E::Satellite>> {
    let mut result: Vec<Candidate<E::Satellite>> = Vec::new();
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
    let mut mean_numbers_snapshot: Vec<u32> = vec![0; group_number];
    mean_numbers_snapshot[data[cursor].group() as usize] = 1;
    while mean_stop_snapshot < data.len()
        && data[mean_stop_snapshot].time() - data[cursor].time() < config.neighbor / 2.0
    {
        mean_stop_snapshot += 1;
        mean_numbers_snapshot[data[mean_stop_snapshot].group() as usize] += 1;
    }

    let mut hollow_start_snapshot = cursor;
    let mut hollow_stop_snapshot = cursor;
    let mut hollow_numbers_snapshot: Vec<u32> = vec![0; group_number];
    hollow_numbers_snapshot[data[cursor].group() as usize] = 1;
    while hollow_stop_snapshot < data.len()
        && data[hollow_stop_snapshot].time() - data[cursor].time() < config.hollow / 2.0
    {
        hollow_stop_snapshot += 1;
        hollow_numbers_snapshot[data[hollow_stop_snapshot].group() as usize] += 1;
    }

    loop {
        let mut step = 0;
        let mut numbers: Vec<u32> = vec![0; group_number];
        numbers[data[cursor].group() as usize] = 1;
        let mut mean_stop = mean_stop_snapshot;
        let mut mean_numbers = mean_numbers_snapshot.clone();
        let mut hollow_stop = hollow_stop_snapshot;
        let mut hollow_numbers = hollow_numbers_snapshot.clone();

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
                let pure_mean_percent =
                    (duration / pure_mean_duration).get::<uom::si::ratio::ratio>();
                let fps = (0..group_number)
                    .map(|group| {
                        let pure_mean_number = mean_numbers[group] - hollow_numbers[group];
                        let equivalent_background_number =
                            pure_mean_number as f64 * pure_mean_percent;
                        match (equivalent_background_number, numbers[group]) {
                            (0.0, 0) => 1.0,
                            (0.0, _) => 1.0,
                            _ => Poisson::new(equivalent_background_number)
                                .unwrap()
                                .sf(numbers[group] as u64),
                        }
                    })
                    .collect::<Vec<f64>>();
                let fp = fps[0];
                let threshold = config.false_positive_per_year
                    / (uom::si::f64::Time::new::<uom::si::time::second>(3600.0)
                        * 24.0
                        * DAYS_PER_YEAR
                        / duration)
                        .get::<uom::si::ratio::ratio>();
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
                    let current = Candidate::new(
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
            mean_numbers_snapshot[data[mean_start_snapshot].group() as usize] -= 1;
            mean_start_snapshot += 1;
        }
        while mean_stop_snapshot + 1 < data.len()
            && data[mean_stop_snapshot + 1].time() - data[cursor].time() < config.neighbor / 2.0
        {
            mean_stop_snapshot += 1;
            mean_numbers_snapshot[data[mean_stop_snapshot].group() as usize] += 1;
        }
        while hollow_start_snapshot + 1 < data.len()
            && data[cursor].time() - data[hollow_start_snapshot + 1].time() > config.hollow / 2.0
        {
            hollow_numbers_snapshot[data[hollow_start_snapshot].group() as usize] -= 1;
            hollow_start_snapshot += 1;
        }
        while hollow_stop_snapshot + 1 < data.len()
            && data[hollow_stop_snapshot + 1].time() - data[cursor].time() < config.hollow / 2.0
        {
            hollow_stop_snapshot += 1;
            hollow_numbers_snapshot[data[hollow_stop_snapshot].group() as usize] += 1;
        }
    }

    result
}
