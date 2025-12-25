use chrono::prelude::*;
use indicatif::ProgressBar;

fn main() {
    let bar = ProgressBar::new(1000);
    for _ in 0..1000 {
        bar.inc(1);
        // ...
    }
    bar.finish();

    let hxmt_he_launch_day = NaiveDate::from_ymd_opt(2017, 6, 22).unwrap();
    let today = Utc::now().naive_utc().date();
    let days_since_launch = (today - hxmt_he_launch_day).num_days();
    println!(
        "Processing HXMT-HE data from launch day ({}) to today ({}), total {} days.",
        hxmt_he_launch_day,
        today,
        days_since_launch + 1
    );
    let progress_bar = ProgressBar::new(days_since_launch as u64 + 1);
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    for day_offset in 0..=days_since_launch {
        let day = hxmt_he_launch_day + chrono::Duration::days(day_offset);
        progress_bar.set_message(format!("{}", day));
        blink_task::process_day::<blink_hxmt_he::types::Chunk>(day);
        progress_bar.inc(1);
    }
    progress_bar.finish();
}
