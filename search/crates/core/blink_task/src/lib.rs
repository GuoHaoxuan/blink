pub mod db;
pub mod types;
pub mod worker;

use blink_core::traits::Chunk;
use chrono::prelude::*;

pub use worker::consume;

pub fn process_day<C: Chunk>(day: NaiveDate) {
    let mut all_signals = Vec::new();
    let mut errors: Vec<blink_core::error::Error> = Vec::new();

    for hour in 0..24 {
        let naive = day.and_hms_opt(hour, 0, 0).expect("invalid time");
        match C::from_epoch(&Utc.from_utc_datetime(&naive)) {
            Ok(chunk) => {
                let mut sigs = chunk.search();
                all_signals.append(&mut sigs);
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    // ensusre folder "data/HXMT-HE/year/month/" exists
    let year = day.year();
    let month = day.month();
    let output_dir = format!("data/HXMT-HE/{:04}/{:02}/", year, month);
    std::fs::create_dir_all(&output_dir).expect("failed to create output directory");
    let output_file = format!(
        "{}{:04}{:02}{:02}_signals.json.tmp",
        output_dir,
        year,
        month,
        day.day()
    );
    let json = serde_json::to_string_pretty(&all_signals).expect("failed to serialize signals");
    std::fs::write(&output_file, json).expect("failed to write output file");
    let final_output_file = output_file.trim_end_matches(".tmp");
    std::fs::rename(&output_file, final_output_file).expect("failed to rename output file");
}
