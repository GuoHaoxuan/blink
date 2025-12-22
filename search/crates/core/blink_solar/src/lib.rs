use std::f64::consts::PI;

use chrono::{TimeDelta, prelude::*};

pub fn mean_solar_time(time: DateTime<Utc>, longitude: f64) -> NaiveTime {
    time.time() + TimeDelta::nanoseconds((longitude * 3600.0 * 1_000_000_000.0 / 15.0) as i64)
}

pub fn day_of_year(time: DateTime<Utc>) -> u32 {
    let start_of_year = time.with_month(1).unwrap().with_day(1).unwrap();
    (time - start_of_year).num_days() as u32 + 1
}

pub fn theta(time: DateTime<Utc>) -> f64 {
    let day = day_of_year(time) as f64;
    let year = time.year() as f64;
    let n0 = 79.6764 + 0.2422 * (year - 1985.0) - (0.25 * (year - 1985.0)).floor();
    2.0 * PI * (day - n0) / 365.2422
}

pub fn solar_declination_angle(time: DateTime<Utc>) -> f64 {
    let theta = theta(time);
    0.3723 + 23.2567 * theta.sin() + 0.1149 * (2.0 * theta).sin()
        - 0.1712 * (3.0 * theta).sin()
        - 0.7580 * theta.cos()
        + 0.3656 * (2.0 * theta).cos()
        + 0.0201 * (3.0 * theta).cos()
}

pub fn equation_of_time(time: DateTime<Utc>) -> f64 {
    let theta = theta(time);
    0.0028 - 1.9857 * theta.sin() + 9.9059 * (2.0 * theta).sin()
        - 7.0924 * theta.cos()
        - 0.6882 * (2.0 * theta).cos()
}

pub fn apparent_solar_time(time: DateTime<Utc>, longitude: f64) -> NaiveTime {
    let mean_time = mean_solar_time(time, longitude);
    let equation_of_time = equation_of_time(time);
    mean_time + TimeDelta::nanoseconds((equation_of_time * 60.0 * 1_000_000_000.0) as i64)
}

pub fn hour_angle(time: DateTime<Utc>, longitude: f64) -> f64 {
    let mean_time = mean_solar_time(time, longitude);
    let hour_angle = (mean_time.hour() as f64 + mean_time.minute() as f64 / 60.0) * 15.0;
    hour_angle - 180.0
}

pub fn solar_zenith_angle(time: DateTime<Utc>, latitude: f64, longitude: f64) -> f64 {
    let declination = solar_declination_angle(time);
    let hour_angle = hour_angle(time, longitude);
    (latitude.to_radians().sin() * declination.to_radians().sin()
        + latitude.to_radians().cos()
            * declination.to_radians().cos()
            * hour_angle.to_radians().cos())
    .acos()
    .to_degrees()
}

pub fn solar_azimuth_angle(time: DateTime<Utc>, latitude: f64, longitude: f64) -> f64 {
    let declination = solar_declination_angle(time);
    let hour_angle = hour_angle(time, longitude);
    let zenith_angle = solar_zenith_angle(time, latitude, longitude);
    (-hour_angle.to_radians().sin() * declination.to_radians().cos()
        / zenith_angle.to_radians().sin())
    .asin()
    .to_degrees()
}

pub fn solar_zenith_angle_at_noon(time: DateTime<Utc>, latitude: f64) -> f64 {
    (latitude - solar_declination_angle(time)).abs()
}
