use blink_core::{
    traits::Instrument,
    types::{TemporalState, UnifiedSignal},
};
use blink_hxmt_he::types::HxmtHe;
use blink_lightning::database::get_lightnings;
use blink_svom_grm::types::SvomGrm;
use blink_task::process;
use chrono::{TimeDelta, prelude::*};
use indicatif::MultiProgress;
use uom::si::f64::*;

macro_rules! load_all_signals {
    ($($instrument:ident),*) => {
        {
            let mut all_signals: Vec<UnifiedSignal> = Vec::new();
            $(
                {
                    let signals: Vec<_> =
                        process::<$instrument, _, _>(None, None, process_day::<$instrument>, 1, 0)
                            .into_iter()
                            .flatten()
                            .collect();
                    println!("Total signals loaded ({}): {}", $instrument::name(), signals.len());
                    for signal in signals {
                        all_signals.push(signal);
                    }
                }
            )*
            all_signals
        }
    };
}

fn main() {
    let all_signals = load_all_signals!(HxmtHe);

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

    let time = Utc::now();
    println!("{}", serde_json::to_string_pretty(&time).unwrap());
}

fn process_day<I: Instrument>(
    day: NaiveDate,
    _multi_progress: &MultiProgress,
) -> Vec<UnifiedSignal> {
    let year = day.year();
    let month = day.month();
    let output_dir = format!(
        "data/{}/{:04}/{:02}/",
        I::name().replace("/", "_"),
        year,
        month
    );
    let output_file = format!(
        "{}{:04}{:02}{:02}_signals.json",
        output_dir,
        year,
        month,
        day.day(),
    );

    if !std::path::Path::new(&output_file).exists() {
        return Vec::new();
    }

    // read file as json
    let file_content = std::fs::read_to_string(&output_file).expect("failed to read output file");
    let signals: Vec<UnifiedSignal> =
        serde_json::from_str(&file_content).expect("failed to parse json");
    signals
}
