use std::fs;

use blink_core::traits::{Chunk, Satellite};
use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

pub fn process<S, R, Map>(
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    map: Map,
    total_workers: usize,
    idx_worker: usize,
) -> Vec<R>
where
    S: Satellite,
    Map: Fn(NaiveDate, &MultiProgress) -> R,
{
    let start_date = match start_date {
        Some(date) => date,
        None => S::launch_day(),
    };
    let end_date = match end_date {
        Some(date) => date,
        None => Utc::now().naive_utc().date(),
    };
    let total_days = (end_date - start_date).num_days() + 1;

    println!(
        "Processing {} data from {} to {}, total {} days.",
        S::name(),
        start_date,
        end_date,
        total_days
    );

    let multi_progress = MultiProgress::new();
    let progress_bar = multi_progress.add(ProgressBar::new(total_days as u64));
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut results = Vec::new();

    for day_offset in 0..total_days {
        let day = start_date + chrono::Duration::days(day_offset);
        progress_bar.set_message(format!("{}", day));
        if (day_offset as usize) % total_workers == idx_worker {
            let result = map(day, &multi_progress);
            results.push(result);
        }
        progress_bar.inc(1);
    }

    progress_bar.finish();

    results
}

pub fn process_all<S: Satellite>(total_workers: usize, idx_worker: usize) {
    process::<S, _, _>(None, None, process_day::<S>, total_workers, idx_worker);
}

fn process_day<S: Satellite>(day: NaiveDate, multi_progress: &MultiProgress) {
    let spin_bar = multi_progress.add(ProgressBar::new(24));
    spin_bar.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );

    spin_bar.set_message("ensure folder exist");
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

    spin_bar.set_message("check last modified");
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
                return;
            }
        }
    }

    spin_bar.finish_and_clear();

    let progress_bar = multi_progress.add(ProgressBar::new(24));
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.yellow/red}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut all_signals = Vec::new();
    let mut errors: Vec<(u32, blink_core::error::Error)> = Vec::new();
    for hour in 0..24 {
        let naive = day.and_hms_opt(hour, 0, 0).expect("invalid time");
        match S::Chunk::from_epoch(&Utc.from_utc_datetime(&naive)) {
            Ok(chunk) => {
                let mut sigs = chunk.search().into_iter().map(|e| e.to_unified()).collect();
                all_signals.append(&mut sigs);
            }
            Err(e) => {
                errors.push((hour, e));
            }
        }
        progress_bar.inc(1);
    }
    progress_bar.finish_and_clear(); // 使用 finish_and_clear() 以便完成后清除内层进度条

    let spin_bar_writting = multi_progress.add(ProgressBar::new_spinner());
    spin_bar_writting.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    spin_bar_writting.set_message("writing output files");

    let suffix = format!(".{}.tmp", nanoid::nanoid!(3));
    let temp_file = format!("{}{}", &output_file, &suffix);

    let json = serde_json::to_string_pretty(&all_signals).expect("failed to serialize signals");
    std::fs::write(&temp_file, json).expect("failed to write output file");
    std::fs::rename(&temp_file, &output_file).expect("failed to rename output file");

    spin_bar_writting.set_message("writing error file");
    let error_file = format!(
        "{}{:04}{:02}{:02}_errors.txt",
        output_dir,
        year,
        month,
        day.day(),
    );
    let error_file_temp = format!("{}{}", &error_file, &suffix);
    if errors.is_empty() {
        let _ = fs::remove_file(&error_file);
    } else {
        let mut error_contents = String::new();
        for (hour, error) in &errors {
            error_contents.push_str(&format!("Error {}T{:02}: {}\n", day, hour, error));
        }
        fs::write(&error_file_temp, error_contents).expect("failed to write error file");
        fs::rename(&error_file_temp, &error_file).expect("failed to rename error file");
    }

    spin_bar_writting.finish_and_clear();
}
