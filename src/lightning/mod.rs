use chrono::{prelude::*, Duration};
use rusqlite::params;
use serde::Serialize;

use crate::env::LIGHTNING_CONNECTION;

const SPEED_OF_LIGHT: f64 = 299_792_458.0;
const R_EARTH: f64 = 6_371_000.0;

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

impl Lightning {
    pub(crate) fn associated_lightning(
        time: DateTime<Utc>,
        lat: f64,
        lon: f64,
        time_tolerance: Duration,
        distance_tolerance: f64,
    ) -> Vec<Self> {
        let time_start = time - time_tolerance - Duration::milliseconds(50);
        let time_start_str = time_start.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let time_end = time + time_tolerance + Duration::milliseconds(50);
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
                ",
            )
            .unwrap();
        let mut rows = statement
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
            .collect::<Vec<_>>();

        rows.retain(|lightning| {
            let dist = distance(lat, lon, lightning.lat, lightning.lon);
            let time_of_arrival_value = time_of_arrival(dist, 550_000.0, 15_000.0);
            let time_delta = (lightning.time + time_of_arrival_value) - time;
            time_delta.abs() <= time_tolerance && dist <= distance_tolerance
        });

        rows.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        rows
    }
}
