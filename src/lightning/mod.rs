use chrono::{prelude::*, Duration};
use itertools::Itertools;
use rusqlite::params;
use serde::Serialize;

use crate::env::LIGHTNING_CONNECTION;

const SPEED_OF_LIGHT: f64 = 299_792_458.0;
const R_EARTH: f64 = 6_371_000.0;
const LIGHTNING_ALTITUDE: f64 = 15_000.0;

fn hav(theta: f64) -> f64 {
    (theta / 2.0).sin().powi(2)
}

fn distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_phi = (lat2 - lat1).to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();

    let a = hav(delta_phi) + phi1.cos() * phi2.cos() * hav(delta_lambda);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    R_EARTH * c
}

fn time_of_arrival(distance: f64, h1: f64, h2: f64) -> Duration {
    let alpha = distance / R_EARTH;
    let d2 = (R_EARTH + h1).powi(2) + (R_EARTH + h2).powi(2)
        - 2.0 * (R_EARTH + h1) * (R_EARTH + h2) * alpha.cos();
    Duration::nanoseconds((d2.sqrt() / SPEED_OF_LIGHT * 1_000_000_000.0).round() as i64)
}

#[derive(Debug, Serialize)]
pub(crate) struct Lightning {
    pub(crate) time: DateTime<Utc>,
    pub(crate) lat: f64,
    pub(crate) lon: f64,
    pub(crate) resid: f64,
    pub(crate) nstn: u32,
    pub(crate) energy: Option<f64>,
    pub(crate) energy_uncertainty: Option<f64>,
    pub(crate) estn: Option<u32>,
}

fn get_lightnings(time_start: DateTime<Utc>, time_end: DateTime<Utc>) -> Vec<Lightning> {
    let time_start_str = time_start.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let time_end_str = time_end.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let connection = LIGHTNING_CONNECTION.lock().unwrap();
    let mut statement = connection
        .prepare(
            "
                SELECT
                    time,
                    lat,
                    lon,
                    resid,
                    nstn,
                    energy,
                    energy_uncertainty,
                    estn
                FROM
                    lightning
                WHERE
                    time BETWEEN ?1 AND ?2
                ORDER BY time ASC
                ",
        )
        .unwrap();
    statement
        .query_map(params![time_start_str, time_end_str], |row| {
            Ok(Lightning {
                time: NaiveDateTime::parse_from_str(
                    &row.get::<_, String>(0).unwrap(),
                    "%Y-%m-%d %H:%M:%S%.6f",
                )
                .unwrap()
                .and_utc(),
                lat: row.get::<_, f64>(1).unwrap(),
                lon: row.get::<_, f64>(2).unwrap(),
                resid: row.get::<_, f64>(3).unwrap(),
                nstn: row.get::<_, i64>(4).unwrap() as u32,
                energy: row.get::<_, Option<f64>>(5).unwrap(),
                energy_uncertainty: row.get::<_, Option<f64>>(6).unwrap(),
                estn: row.get::<_, Option<i64>>(7).unwrap().map(|x| x as u32),
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>()
}

pub(crate) fn associated_lightning(
    time: DateTime<Utc>,
    lat: f64,
    lon: f64,
    alt: f64,
    time_tolerance: Duration,
    distance_tolerance: f64,
) -> Vec<Lightning> {
    let time_start = time - time_tolerance - Duration::seconds(1);
    let time_end = time + time_tolerance + Duration::seconds(1);
    let mut rows = get_lightnings(time_start, time_end);

    rows.retain(|lightning| {
        let dist = distance(lat, lon, lightning.lat, lightning.lon);
        let time_of_arrival_value = time_of_arrival(dist, alt, LIGHTNING_ALTITUDE);
        let fixed_time = time - time_of_arrival_value;
        let time_delta = lightning.time - fixed_time;
        time_delta.abs() <= time_tolerance && dist <= distance_tolerance
    });

    rows.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

    rows
}

fn coincidence_window(
    lightning: &Lightning,
    lat: f64,
    lon: f64,
    alt: f64,
    time_tolerance: Duration,
) -> [DateTime<Utc>; 2] {
    let dist = distance(lat, lon, lightning.lat, lightning.lon);
    let time_of_arrival_value = time_of_arrival(dist, alt, LIGHTNING_ALTITUDE);
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

pub(crate) fn coincidence_prob(
    time: DateTime<Utc>,
    lat: f64,
    lon: f64,
    alt: f64,
    time_tolerance: Duration,
    distance_tolerance: f64,
    time_window: Duration,
) -> f64 {
    let time_start = time - time_tolerance - Duration::seconds(1) - time_window / 2;
    let time_end = time + time_tolerance + Duration::seconds(1) + time_window / 2;
    let mut rows = get_lightnings(time_start, time_end);
    rows.retain(|lightning| {
        let dist = distance(lat, lon, lightning.lat, lightning.lon);
        dist <= distance_tolerance
    });
    let windows = rows
        .iter()
        .map(|lightning| coincidence_window(lightning, lat, lon, alt, time_tolerance))
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
