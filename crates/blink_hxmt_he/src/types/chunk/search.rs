use super::Chunk;
use crate::algorithms::continuous;
use crate::types::{Event, Hxmt};
use blink_algorithms::snapshot_stepping::{SearchConfig, search_new};
use blink_core::traits::Event as _;
use blink_core::types::{Attitude, MissionElapsedTime, Position, Signal, Trajectory};
use uom::si::f64::*;

pub fn search(chunk: &Chunk) -> Vec<Signal<Event>> {
    let events = chunk
        .event_file
        .into_iter()
        .filter(|event| event.keep())
        .collect::<Vec<_>>();

    let results = search_new(
        &events,
        1,
        chunk.span[0],
        chunk.span[1],
        SearchConfig {
            min_duration: Time::new::<uom::si::time::microsecond>(0.0),
            max_duration: Time::new::<uom::si::time::millisecond>(1.0),
            neighbor: Time::new::<uom::si::time::second>(1.0),
            hollow: Time::new::<uom::si::time::millisecond>(10.0),
            false_positive_per_year: 20.0,
            min_number: 8,
        },
    );

    for result in &results {
        println!(
            "Found candidate: start = {} s, stop = {} s, count = {}, sf = {:.3}, FPPY = {:.3}",
            result.start.to_utc(),
            result.stop.to_utc(),
            result.count,
            result.sf(),
            result.false_positive_per_year()
        );
    }

    let results = continuous(
        results,
        Time::new::<uom::si::time::second>(10.0),
        Time::new::<uom::si::time::second>(1.0),
        10,
    );

    println!("After continuous merging:");
    for result in &results {
        println!(
            "Found candidate: start = {} s, stop = {} s, count = {}, sf = {:.3}, FPPY = {:.3}",
            result.start.to_utc(),
            result.stop.to_utc(),
            result.count,
            result.sf(),
            result.false_positive_per_year()
        );
    }

    let results = results
        .into_iter()
        .filter(|candidate| !chunk.check_saturation(candidate.start))
        .collect::<Vec<_>>();

    println!("After saturation check:");
    for result in &results {
        println!(
            "Found candidate: start = {} s, stop = {} s, count = {}, sf = {:.3}, FPPY = {:.3}",
            result.start.to_utc(),
            result.stop.to_utc(),
            result.count,
            result.sf(),
            result.false_positive_per_year()
        );
    }

    // let signals = signals
    //     .into_iter()
    //     .filter(|signal| {
    //         signal.false_positive_per_year <= 1e-5
    //             || (signal.false_positive_per_year <= 1e0 && signal.associated_lightning_count > 0)
    //     })
    //     .collect::<Vec<_>>();

    results
        .into_iter()
        .filter_map(|candidate| {
            let extended_half_time = Time::new::<uom::si::time::millisecond>(500.0);
            let events = chunk
                .event_file
                .into_iter()
                .filter(|event| {
                    event.time() >= candidate.start - extended_half_time
                        && event.time() <= candidate.stop + extended_half_time
                })
                .collect::<Vec<_>>();
            let attitude = Trajectory::<MissionElapsedTime<Hxmt>, Attitude>::from(&chunk.att_file)
                .interpolate(candidate.start)?;
            let orbit = Trajectory::<MissionElapsedTime<Hxmt>, Position>::from(&chunk.orbit_file)
                .window(candidate.start, Time::new::<uom::si::time::second>(500.0));
            Some(Signal {
                start: candidate.start,
                stop: candidate.stop,
                bin_size_min: candidate.bin_size_min,
                bin_size_max: candidate.bin_size_max,
                bin_size_best: candidate.bin_size_best,
                delay: candidate.delay,
                count: candidate.count,
                mean: candidate.mean,
                sf: candidate.sf(),
                false_positive_per_year: candidate.false_positive_per_year(),
                events,
                attitude,
                orbit,
            })
        })
        .collect::<Vec<_>>()
}
