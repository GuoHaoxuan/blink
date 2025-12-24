use chrono::prelude::*;

fn main() {
    let hxmt_he_launch_day = NaiveDate::from_ymd_opt(2017, 6, 22).unwrap();
    let today = Utc::now().naive_utc().date();
    let days_since_launch = (today - hxmt_he_launch_day).num_days();
    for day_offset in 0..=days_since_launch {
        let day = hxmt_he_launch_day + chrono::Duration::days(day_offset);
        blink_task::process_day::<blink_hxmt_he::types::Chunk>(day);
    }
}
