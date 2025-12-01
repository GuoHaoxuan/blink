use crate::algorithms::geo::distance;
use crate::algorithms::geo::time_of_arrival;
use crate::constants::LIGHTNING_ALTITUDE;
use crate::database::get_lightnings;
use crate::types::Lightning;
use blink_core::types::Position;
use blink_core::types::TemporalState;
use chrono::Duration;
use chrono::prelude::*;
use itertools::Itertools;
use uom::si::f64::*;

pub fn coincidence_prob(
    location: TemporalState<Position>,
    time_tolerance: Duration,
    distance_tolerance: Length,
    time_window: Duration,
) -> f64 {
    let time_start = location.timestamp - time_tolerance - Duration::seconds(1) - time_window / 2;
    let time_end = location.timestamp + time_tolerance + Duration::seconds(1) + time_window / 2;
    let mut rows = get_lightnings(time_start, time_end);
    rows.retain(|lightning| {
        let dist = distance(
            location.state.latitude,
            location.state.longitude,
            lightning.lat,
            lightning.lon,
        );
        dist <= distance_tolerance
    });
    let windows = rows
        .iter()
        .map(|lightning| {
            coincidence_window(
                lightning,
                location.state.latitude,
                location.state.longitude,
                location.state.altitude,
                time_tolerance,
            )
        })
        .sorted_by(|a, b| a[0].partial_cmp(&b[0]).unwrap())
        .coalesce(|a, b| {
            if mergeable(&a, &b) {
                Ok(merge(&a, &b))
            } else {
                Err((a, b))
            }
        })
        .map(|window| trim(&window, time_start, time_end))
        .filter(|window| window[0] < window[1])
        .collect::<Vec<_>>();
    let total_time = time_end - time_start;
    let total_window = windows
        .iter()
        .map(|window| window[1] - window[0])
        .sum::<Duration>();
    let total_window = total_window.num_nanoseconds().unwrap_or(0) as f64;
    let total_time = total_time.num_nanoseconds().unwrap_or(0) as f64;
    total_window / total_time
}

fn coincidence_window(
    lightning: &Lightning,
    lat: f64,
    lon: f64,
    alt: Length,
    time_tolerance: Duration,
) -> [DateTime<Utc>; 2] {
    let dist = distance(lat, lon, lightning.lat, lightning.lon);
    let time_of_arrival_value = time_of_arrival(dist, alt, *LIGHTNING_ALTITUDE);
    let fixed_time = lightning.time + time_of_arrival_value;
    let start_time = fixed_time - time_tolerance;
    let end_time = fixed_time + time_tolerance;

    [start_time, end_time]
}

fn mergeable(a: &[DateTime<Utc>; 2], b: &[DateTime<Utc>; 2]) -> bool {
    b[0] <= a[1]
}

fn merge(a: &[DateTime<Utc>; 2], b: &[DateTime<Utc>; 2]) -> [DateTime<Utc>; 2] {
    [a[0].min(b[0]), a[1].max(b[1])]
}

fn trim(window: &[DateTime<Utc>; 2], min: DateTime<Utc>, max: DateTime<Utc>) -> [DateTime<Utc>; 2] {
    let start = min.max(window[0]);
    let end = max.min(window[1]);
    [start, end]
}
