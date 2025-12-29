use std::fs;

use blink_core::traits::{Chunk, Satellite};
use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

pub fn process_all<S: Satellite>(total_workers: usize, idx_worker: usize) {
    let launch_day = S::launch_day();
    let today = Utc::now().naive_utc().date();
    let days_since_launch = (today - launch_day).num_days();
    println!(
        "Processing {} data from launch day ({}) to today ({}), total {} days.",
        S::name(),
        launch_day,
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
        let day = launch_day + chrono::Duration::days(day_offset);
        progress_bar.set_message(format!("{}", day));
        if (day_offset as usize) % total_workers == idx_worker {
            process_day::<S>(day, &multi_progress);
        }
        progress_bar.inc(1);
    }
    progress_bar.finish();
}

fn process_day<S: Satellite>(day: NaiveDate, multi_progress: &MultiProgress) {
    let mut all_signals = Vec::new();
    let mut errors: Vec<(u32, blink_core::error::Error)> = Vec::new();

    let year = day.year();
    let month = day.month();
    let output_dir = format!(
        "data/{}/{:04}/{:02}/",
        S::name().replace("/", "_"),
        year,
        month
    );
    std::fs::create_dir_all(&output_dir).expect("failed to create output directory");
    let output_file = format!(
        "{}{:04}{:02}{:02}_signals.json",
        output_dir,
        year,
        month,
        day.day(),
    );

    let last_modified = (0..24)
        .filter_map(|hour| {
            let naive = day.and_hms_opt(hour, 0, 0).expect("invalid time");
            let epoch = Utc.from_utc_datetime(&naive);
            S::Chunk::last_modified(&epoch).ok()
        })
        .max();
    if let Some(last_modified) = last_modified {
        let last_processed = fs::metadata(&output_file).and_then(|metadata| metadata.modified());
        if let Ok(last_processed) = last_processed {
            let last_processed: DateTime<Utc> = last_processed.into();
            if last_processed >= last_modified {
                // println!("Data for {} on {} is up to date, skipping.", S::name(), day);
                return;
            }
        }
    }

    let progress_bar = multi_progress.add(ProgressBar::new(24));
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.yellow/red}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    for hour in 0..24 {
        let naive = day.and_hms_opt(hour, 0, 0).expect("invalid time");
        match S::Chunk::from_epoch(&Utc.from_utc_datetime(&naive)) {
            Ok(chunk) => {
                let mut sigs = chunk.search();
                all_signals.append(&mut sigs);
            }
            Err(e) => {
                errors.push((hour, e));
            }
        }
        progress_bar.inc(1);
    }
    progress_bar.finish_and_clear(); // 使用 finish_and_clear() 以便完成后清除内层进度条

    let suffix = format!(".{}.tmp", nanoid::nanoid!(3));
    let temp_file = format!("{}{}", &output_file, &suffix);

    let json = serde_json::to_string_pretty(&all_signals).expect("failed to serialize signals");
    std::fs::write(&temp_file, json).expect("failed to write output file");
    std::fs::rename(&temp_file, &output_file).expect("failed to rename output file");

    for (hour, error) in errors {
        eprintln!("Error {}T{:02}: {}", day, hour, error);
    }
}
