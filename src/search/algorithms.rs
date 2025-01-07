use super::light_curve;
use super::poisson::poisson_isf_cached;
use super::trigger::Trigger;

use itertools::Itertools;

fn search_by_light_curve(
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
                ))
            } else {
                None
            }
        })
        .collect()
}

fn search_by_raw(
    time: &[f64],
    start: f64,
    stop: f64,
    bin_size: f64,
    num_neighbors: usize,
    fp_year: f64,
    min_count: u32,
) -> Vec<Trigger> {
    let mut result = Vec::new();
    let mut cache = vec![0; 100_000];

    let mut bin_start = 0;
    let mut bin_stop = 0;
    let mut average_start = 0;
    let mut average_stop = 0;
    let mut cursor = start;

    loop {
        while bin_stop < time.len() && time[bin_stop] <= cursor + bin_size {
            bin_stop += 1;
        }
        while average_stop < time.len()
            && time[average_stop] <= cursor + bin_size * num_neighbors as f64 / 2.0
        {
            average_stop += 1;
        }
        while time[bin_start] <= cursor {
            bin_start += 1;
        }
        while time[average_start] <= cursor - bin_size * num_neighbors as f64 / 2.0 {
            average_start += 1;
        }
        let count = (bin_stop - bin_start) as u32;
        if count >= min_count {
            let index = ((cursor - start) / bin_size).floor() as usize;
            let len = ((stop - start) / bin_size).ceil() as usize;
            let index_start = (index as isize - num_neighbors as isize / 2).max(0) as usize;
            let index_stop = (index + num_neighbors / 2).min(len - 1);
            let average =
                (average_stop - average_start) as f64 / (index_stop - index_start + 1) as f64;
            let threshold = poisson_isf_cached(
                fp_year / (3600.0 * 24.0 * 365.0 / bin_size),
                average,
                &mut cache,
            );
            if count >= threshold {
                result.push(Trigger::new(cursor, cursor + bin_size, count, average));
            }
        }
        if bin_stop == time.len() {
            break;
        }
        cursor += ((time[bin_stop] - cursor) / bin_size).floor() * bin_size;
    }

    result
}

fn estimate_light_curve_time(duration: f64, bin_size: f64) -> f64 {
    let bins = (duration / bin_size).ceil();
    bins / 500_000.0
}

fn estimate_raw_time(time: &[f64]) -> f64 {
    time.len() as f64 / 50_000.0
}

fn search_auto(
    time: &[f64],
    start: f64,
    stop: f64,
    bin_size: f64,
    num_neighbors: usize,
    fp_year: f64,
    min_count: u32,
) -> Vec<Trigger> {
    if estimate_light_curve_time(stop - start, bin_size) < estimate_raw_time(time) {
        let lc = light_curve::light_curve(time, start, stop, bin_size);
        let prefix_sum = light_curve::prefix_sum(&lc);
        search_by_light_curve(
            &prefix_sum,
            start,
            bin_size,
            num_neighbors,
            fp_year,
            min_count,
        )
    } else {
        search_by_raw(
            time,
            start,
            stop,
            bin_size,
            num_neighbors,
            fp_year,
            min_count,
        )
    }
}

pub fn search_all(
    time: &[f64],
    start: f64,
    stop: f64,
    num_neighbors: usize,
    fp_year: f64,
    min_count: u32,
) -> Vec<Trigger> {
    let mut results = Vec::new();
    let mut bin_size = 10e-6;

    while bin_size < 1e-3 {
        results.extend((0..4).flat_map(|shift| {
            let shift = shift as f64 / 4.0 * bin_size;
            search_auto(
                time,
                start + shift,
                stop,
                bin_size,
                num_neighbors,
                fp_year,
                min_count,
            )
        }));
        bin_size *= 2.0;
    }
    results.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    results
}
