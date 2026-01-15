use blink_core::types::{TemporalState, UnifiedSignal};
use blink_hxmt_he::types::HxmtHe;
use blink_lightning::{algorithms::coincidence_prob, database::get_lightnings};
use blink_load::load_all;
// use blink_svom_grm::types::SvomGrm;
use chrono::TimeDelta;
use indicatif::{ProgressBar, ProgressIterator};
use serde::Serialize;
use uom::si::f64::*;

#[derive(Serialize)]
struct LightningInfo {
    associated: bool,
    coincidence_probability: f64,
}

#[derive(Serialize)]
struct Tgf {
    signal: UnifiedSignal,
    lightning: LightningInfo,
}

pub fn run() {
    let signals = load_all::<HxmtHe>();
    let progress = ProgressBar::new(signals.len() as u64);
    progress.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let tgfs = signals
        .into_iter()
        .progress_with(progress)
        .map(|signal| {
            let peak_time = signal.peak_time();
            let position = TemporalState {
                timestamp: peak_time,
                state: signal.position.clone(),
            };
            let lightnings = get_lightnings(
                peak_time - TimeDelta::seconds(1),
                peak_time + TimeDelta::seconds(1),
            )
            .into_iter()
            .filter(|lightning| {
                lightning.is_associated(
                    &position,
                    TimeDelta::milliseconds(5),
                    Length::new::<uom::si::length::kilometer>(800.0),
                )
            })
            .collect::<Vec<_>>();

            Tgf {
                signal,
                lightning: LightningInfo {
                    associated: !lightnings.is_empty(),
                    coincidence_probability: coincidence_prob(
                        &position,
                        TimeDelta::milliseconds(5),
                        Length::new::<uom::si::length::kilometer>(800.0),
                        TimeDelta::minutes(2),
                    ),
                },
            }
        })
        .collect::<Vec<_>>();
    let json = serde_json::to_string_pretty(&tgfs).expect("failed to serialize to json");
    std::fs::write("tgfs.json", json).expect("failed to write tgfs.json");
}
