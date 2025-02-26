use statrs::distribution::{DiscreteCDF, Poisson};

use crate::types::{Duration, Epoch, Event, Group, Interval, Satellite, TimeUnits};

pub struct SearchConfig<T: Satellite> {
    pub max_duration: Duration<T>,
    pub neighbor: Duration<T>,
    pub fp_year: f64,
    pub min_detector: u32,
}

impl<T: Satellite> Default for SearchConfig<T> {
    fn default() -> Self {
        Self {
            max_duration: 1.0.milliseconds(),
            neighbor: 1.0.seconds(),
            fp_year: 20.0,
            min_detector: 3,
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
                // The following line can be removed, because cache[n_i] is 0.0 initially
                // although it is meaningless mathematically
                // (m_i, n_i) if m_i == n_i => probs[m_i - 1] * cache[n_i - 1],
                (m_i, n_i) => probs[m_i - 1] * cache[n_i - 1] + (1.0 - probs[m_i - 1]) * cache[n_i],
            }
        }
    }

    cache[n]
}

pub fn search<E: Event + Group>(
    data: &[E],
    group_count: usize,
    start: Epoch<E::Satellite>,
    stop: Epoch<E::Satellite>,
    config: SearchConfig<E::Satellite>,
) -> Vec<Interval<Epoch<E::Satellite>>> {
    let mut result: Vec<Interval<Epoch<E::Satellite>>> = Vec::new();
    let mut cache = vec![vec![None; 10]; 10000];

    let mut cursor = match data.binary_search_by(|event| event.time().cmp(&start)) {
        Ok(index) => index,
        Err(index) => index,
    };

    if cursor == data.len() {
        return result;
    }

    let mut average_start_base = cursor;
    let mut average_stop_base = cursor;
    let mut average_counts_base: Vec<u32> = vec![0; group_count];
    average_counts_base[data[cursor].group().unwrap() as usize] = 1;
    while average_stop_base + 1 < data.len()
        && data[average_stop_base + 1].time() - data[cursor].time() < config.neighbor / 2.0
    {
        average_stop_base += 1;
        average_counts_base[data[average_stop_base].group().unwrap() as usize] += 1;
    }

    loop {
        let mut step = 0;
        let mut counts: Vec<u32> = vec![0; group_count];
        counts[data[cursor].group().unwrap() as usize] = 1;
        let mut average_stop = average_stop_base;
        let mut average_counts = average_counts_base.clone();

        loop {
            let duration = data[cursor + step].time() - data[cursor].time();
            let average_start_time = (data[cursor].time() - config.neighbor / 2.0).max(start);
            let average_stop_time = (data[cursor + step].time() + config.neighbor / 2.0).min(stop);
            let average_duration = (average_stop_time - average_start_time) - duration;
            let average_percent = duration.to_seconds() / average_duration.to_seconds();
            let threshold = 1.0 - config.fp_year / (3600.0 * 24.0 * 365.0 / duration.to_seconds());
            let probs = (0..group_count)
                .map(|group| {
                    let count = counts[group];
                    let average_count = average_counts[group] - count;
                    let average = average_count as f64 * average_percent;

                    if count == 0 {
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
                    }
                })
                .collect::<Vec<f64>>();
            let prob = coincidence_prob(&probs, 3);
            if prob > threshold {
                let new_interval = Interval {
                    start: data[cursor].time(),
                    stop: data[cursor + step].time(),
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
                || data[cursor + step].time() - data[cursor].time() >= config.max_duration
                || data[cursor + step].time() >= stop
            {
                break;
            }

            counts[data[cursor + step].group().unwrap() as usize] += 1;
            while average_stop + 1 < data.len()
                && data[average_stop + 1].time() - data[cursor + step].time()
                    < config.neighbor / 2.0
            {
                average_stop += 1;
                average_counts[data[average_stop].group().unwrap() as usize] += 1;
            }
        }

        cursor += 1;

        if cursor >= data.len() || data[cursor].time() >= stop {
            break;
        }

        while average_start_base + 1 < data.len()
            && data[cursor].time() - data[average_start_base + 1].time() > config.neighbor / 2.0
        {
            average_counts_base[data[average_start_base].group().unwrap() as usize] -= 1;
            average_start_base += 1;
        }
        while average_stop_base + 1 < data.len()
            && data[average_stop_base + 1].time() - data[cursor].time() < config.neighbor / 2.0
        {
            average_stop_base += 1;
            average_counts_base[data[average_stop_base].group().unwrap() as usize] += 1;
        }
    }

    result
}
