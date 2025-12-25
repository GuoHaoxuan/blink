use blink_core::traits::Chunk;
use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

pub fn process_day<C: Chunk>(day: NaiveDate, multi_progress: &MultiProgress) {
    let mut all_signals = Vec::new();
    let mut errors: Vec<blink_core::error::Error> = Vec::new();

    let progress_bar = multi_progress.add(ProgressBar::new(24 + 1));
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("  {spinner:.blue} [{bar:30.yellow/red}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for hour in 0..24 {
        let naive = day.and_hms_opt(hour, 0, 0).expect("invalid time");
        progress_bar.set_message(format!("Hour {:02}:00", hour));
        match C::from_epoch(&Utc.from_utc_datetime(&naive)) {
            Ok(chunk) => {
                let mut sigs = chunk.search();
                all_signals.append(&mut sigs);
            }
            Err(e) => {
                errors.push(e);
            }
        }
        progress_bar.inc(1);
    }

    progress_bar.set_message("Writing output");
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
    progress_bar.inc(1);
    progress_bar.finish_and_clear(); // 使用 finish_and_clear() 以便完成后清除内层进度条
}
