use blink_core::traits::Chunk;
use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

pub fn process_all(total_workers: usize, idx_worker: usize) {
    let hxmt_he_launch_day = NaiveDate::from_ymd_opt(2017, 6, 22).unwrap();
    let today = Utc::now().naive_utc().date();
    let days_since_launch = (today - hxmt_he_launch_day).num_days();
    println!(
        "Processing HXMT-HE data from launch day ({}) to today ({}), total {} days.",
        hxmt_he_launch_day,
        today,
        days_since_launch + 1
    );

    let multi_progress = MultiProgress::new();
    let progress_bar = multi_progress.add(ProgressBar::new(days_since_launch as u64 + 1));
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for day_offset in 0..=days_since_launch {
        let day = hxmt_he_launch_day + chrono::Duration::days(day_offset);
        progress_bar.set_message(format!("{}", day));
        if (day_offset as usize) % total_workers == idx_worker {
            process_day::<blink_hxmt_he::types::Chunk>(day, &multi_progress);
        }
        progress_bar.inc(1);
    }
    progress_bar.finish();
}

pub fn process_day<C: Chunk>(day: NaiveDate, multi_progress: &MultiProgress) {
    let mut all_signals = Vec::new();
    let mut errors: Vec<blink_core::error::Error> = Vec::new();

    let progress_bar = multi_progress.add(ProgressBar::new(24));
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.yellow/red}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

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
        progress_bar.inc(1);
    }
    progress_bar.finish_and_clear(); // 使用 finish_and_clear() 以便完成后清除内层进度条

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
