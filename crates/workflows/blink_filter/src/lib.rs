use blink_core::types::TemporalState;
use blink_hxmt_he::types::HxmtHe;
use blink_lightning::database::get_lightnings;
use blink_load::load_all;
// use blink_svom_grm::types::SvomGrm;
use chrono::{TimeDelta, prelude::*};
// use indicatif::MultiProgress;
use uom::si::f64::*;

pub fn run() {
    let all_signals = load_all::<HxmtHe>();

    let mut all_signals = all_signals
        .into_iter()
        .filter(|signal| signal.false_positive_per_year <= 1e0)
        .filter(|signal| {
            let peak_time = signal.peak_time();
            let lightnings = get_lightnings(
                peak_time - TimeDelta::seconds(1),
                peak_time + TimeDelta::seconds(1),
            )
            .into_iter()
            .filter(|lightning| {
                lightning.is_associated(
                    &TemporalState {
                        timestamp: peak_time,
                        state: signal.position.clone(),
                    },
                    TimeDelta::milliseconds(5),
                    Length::new::<uom::si::length::kilometer>(800.0),
                )
            })
            .collect::<Vec<_>>();

            !lightnings.is_empty() || signal.false_positive_per_year <= 1e-5
        })
        .collect::<Vec<_>>();
    all_signals.sort_by(|a, b| a.start.cmp(&b.start));

    println!("Total unified signals loaded: {}", all_signals.len());

    for signal in all_signals.iter() {
        println!(
            "{} - {} | FPY: {:.2e} | Instrument: {}",
            signal.start, signal.stop, signal.false_positive_per_year, signal.instrument
        );
    }

    let time = Utc::now();
    println!("{}", serde_json::to_string_pretty(&time).unwrap());
}
