use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

fn main() {
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
            .template("[{elapsed_precise}] [{wide_bar.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for day_offset in 0..=days_since_launch {
        let day = hxmt_he_launch_day + chrono::Duration::days(day_offset);
        progress_bar.set_message(format!("{}", day));
        blink_task::process_day::<blink_hxmt_he::types::Chunk>(day, &multi_progress);
        progress_bar.inc(1);
    }
    progress_bar.finish();
}
