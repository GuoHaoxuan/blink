use crate::types::Chunk;
use crate::types::Event;
use crate::types::Svom;
use blink_algorithms::snapshot_stepping::SearchConfig;
use blink_algorithms::snapshot_stepping::search_new;
use blink_core::traits::Event as _;
use blink_core::types::Attitude;
use blink_core::types::MissionElapsedTime;
use blink_core::types::Position;
use blink_core::types::Signal;
use blink_core::types::Trajectory;
use uom::si::f64::*;

pub(super) fn search(chunk: &Chunk) -> Vec<Signal<Event>> {
    let events = chunk.evt_file.into_iter().collect::<Vec<_>>();
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

    results
        .into_iter()
        .filter_map(|candidate| {
            let extended_half_time = Time::new::<uom::si::time::millisecond>(500.0);
            let events = chunk
                .evt_file
                .into_iter()
                .filter(|event| {
                    event.time() >= candidate.start - extended_half_time
                        && event.time() <= candidate.stop + extended_half_time
                })
                .collect::<Vec<_>>();
            let attitude = Trajectory::<MissionElapsedTime<Svom>, Attitude>::from(&chunk.att_file)
                .interpolate(candidate.start)?;
            let orbit = Trajectory::<MissionElapsedTime<Svom>, Position>::from(&chunk.orb_file)
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
