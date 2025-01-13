pub mod algorithms;
pub mod algorithms2;
pub mod fermi;
pub mod light_curve;
pub mod poisson;
pub mod record;
pub mod trigger;

use algorithms::search_all;
use core::str::FromStr;
use fitsio::FitsFile;
use hifitime::prelude::*;
use itertools::Itertools;

pub fn calculate_hxmt(filename: &str) -> Vec<record::Record> {
    let mut fptr = FitsFile::open(filename).unwrap();
    let events = fptr.hdu("EVENTS").unwrap();
    let start: f64 = events.read_key(&mut fptr, "TSTART").unwrap();
    let stop: f64 = events.read_key(&mut fptr, "TSTOP").unwrap();
    let date_obs: String = events.read_key(&mut fptr, "DATE-OBS").unwrap();
    let date_obs = Epoch::from_str(&date_obs).unwrap();
    let channel: Vec<u8> = events.read_col(&mut fptr, "Channel").unwrap();
    let time: Vec<_> = events.read_col::<f64>(&mut fptr, "Time").unwrap();
    let time: Vec<_> = time
        .iter()
        .zip(channel)
        .filter(|&(_, c)| c >= 38)
        .map(|(&t, _)| t - start)
        .dedup_by(|a, b| (a - b).abs() < 10e-9)
        .collect();
    calculate(&time, start, stop, date_obs, 100, 20.0, 8)
}

pub fn calculate(
    time: &[f64],
    start: f64,
    stop: f64,
    date_obs: Epoch,
    num_neighbors: usize,
    fp_year: f64,
    min_count: u32,
) -> Vec<record::Record> {
    let results = search_all(time, 0.0, stop - start, num_neighbors, fp_year, min_count);
    results
        .into_iter()
        .coalesce(|prev, next| {
            if prev.mergeable(&next, 0) {
                Ok(prev.merge(&next))
            } else {
                Err((prev, next))
            }
        })
        .map(|trigger| record::Record::new(&trigger, date_obs))
        .collect()
}
