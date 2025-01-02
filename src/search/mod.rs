pub mod algorithm;
pub mod light_curve;
pub mod poisson;
pub mod record;
pub mod trigger;

use core::str::FromStr;
use fitsio::FitsFile;
use hifitime::Epoch;
use itertools::Itertools;

pub fn calculate(filename: &str) -> Vec<record::Record> {
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
                algorithm::search_light_curve(&prefix_sum, shift, bin_size, 100, fp_year, min_count)
            } else {
                algorithm::search_raw(
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
