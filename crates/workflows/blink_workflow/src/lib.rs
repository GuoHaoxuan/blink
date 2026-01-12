use blink_core::traits::Instrument;
use chrono::prelude::*;
use indicatif::{MultiProgress, ProgressBar};

pub fn process<I, R, Map>(
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    map: Map,
    total_workers: usize,
    idx_worker: usize,
) -> Vec<R>
where
    I: Instrument,
    Map: Fn(NaiveDate, &MultiProgress) -> R,
{
    let start_date = match start_date {
        Some(date) => date,
        None => I::launch_day(),
    };
    let end_date = match end_date {
        Some(date) => date,
        None => Utc::now().naive_utc().date(),
    };
    let total_days = (end_date - start_date).num_days() + 1;

    println!(
        "Processing {} data from {} to {}, total {} days.",
        I::name(),
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
