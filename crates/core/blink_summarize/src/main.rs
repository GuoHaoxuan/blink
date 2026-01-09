use blink_core::{traits::Instrument, types::UnifiedSignal};
use blink_hxmt_he::types::HxmtHe;
use blink_svom_grm::types::SvomGrm;
use blink_task::process;
use chrono::prelude::*;
use indicatif::MultiProgress;

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
    let mut all_signals = load_all_signals!(HxmtHe, SvomGrm);
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
