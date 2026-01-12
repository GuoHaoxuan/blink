use blink_core::{traits::Instrument, types::UnifiedSignal};
use blink_workflow::process;
use chrono::prelude::*;
use indicatif::MultiProgress;

pub fn load_day<I: Instrument>(
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

pub fn load_all<I: Instrument>() -> Vec<UnifiedSignal> {
    process::<I, _, _>(None, None, load_day::<I>, 1, 0)
        .into_iter()
        .flatten()
        .collect()
}
