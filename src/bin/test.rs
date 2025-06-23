use blink::hxmt::Hxmt;
use blink::search::algorithms::{SearchConfig, search_new};

use blink::types::{Event, Span, Time};
use chrono::prelude::*;

fn test() {
    let evt_file =
        blink::hxmt::EventFile::new("HXMT_20170824T10_HE-Evt_FFFFFF_V1_1K.FITS").unwrap();
    const CHANNEL_THRESHOLD: u16 = 38;
    let events = evt_file
        .into_iter()
        .filter(|event| !event.detector.am241)
        .filter(|event| event.energy() >= CHANNEL_THRESHOLD)
        .collect::<Vec<_>>();
    let test_start = DateTime::parse_from_rfc3339("2017-08-24T10:00:00+00:00")
        .unwrap()
        .to_utc();
    let test_stop = DateTime::parse_from_rfc3339("2017-08-24T11:00:00+00:00")
        .unwrap()
        .to_utc();
    let results = search_new(
        &events,
        1,
        Time::<Hxmt>::from(test_start),
        Time::<Hxmt>::from(test_stop),
        SearchConfig {
            min_duration: Span::microseconds(0.0),
            max_duration: Span::microseconds(1000.0),
            neighbor: Span::seconds(1.0),
            hollow: Span::milliseconds(10.0),
            fp_year: 20.0,
            min_number: 8,
        },
    );
    // print!("{:#?}", results);
    results.iter().for_each(|trigger| {
        println!("{}", trigger.fp_year());
    });
    // println!(
    //     "{:?}",
    //     results
    //         .iter()
    //         .map(|trigger| { (trigger.stop - trigger.start).to_seconds() })
    //         .collect::<Vec<_>>()
    // );
    // println!("Number of triggers: {}", results.len());
}

fn main() {
    test();
}
