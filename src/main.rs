mod search;

use search::{fermi::process, record::Record};
use std::str::FromStr;

use hifitime::prelude::*;
use polars::prelude::*;

fn print_results(results: &[Record]) {
    let df: DataFrame = df!(
        "start" => results.iter().map(|x| x.start.to_string()).collect::<Vec<_>>(),
        "stop" => results.iter().map(|x| x.stop.to_string()).collect::<Vec<_>>(),
        "bin_size_min" => results.iter().map(|x| (x.bin_size_min.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "bin_size_max" => results.iter().map(|x| (x.bin_size_max.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "bin_size_best" => results.iter().map(|x| (x.bin_size_best.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "delay" => results.iter().map(|x| (x.delay.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "count" => results.iter().map(|x| x.count).collect::<Vec<_>>(),
        "average" => results.iter().map(|x| x.average).collect::<Vec<_>>(),
    )
    .unwrap();
    if df.height() > 0 {
        println!("{}", df);
    }
}

fn main() {
    let start = Epoch::from_str("2023-01-01T00:00:00").unwrap();
    let end = Epoch::from_str("2024-01-01T00:00:00").unwrap();
    let step = 1.hours();
    let time_series = TimeSeries::inclusive(start, end, step);
    for epoch in time_series {
        let results = process(&epoch);
        print_results(&results);
    }
}
