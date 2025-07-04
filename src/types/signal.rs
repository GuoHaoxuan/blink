use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::lightning::LightningAssociation;
use crate::solar::{
    apparent_solar_time, day_of_year, mean_solar_time, solar_azimuth_angle, solar_zenith_angle,
    solar_zenith_angle_at_noon,
};

use super::GenericEvent;

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub time: DateTime<Utc>,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
}

pub struct LocationList {
    pub data: Vec<Location>,
}

impl LocationList {
    pub fn interpolate(&self, time: DateTime<Utc>) -> Option<Location> {
        let mut i = 0;
        while i < self.data.len() - 1 && self.data[i + 1].time < time {
            i += 1;
        }
        if i == self.data.len() - 1 {
            return None;
        }

        let t0 = self.data[i].time;
        let t1 = self.data[i + 1].time;
        let lon0 = self.data[i].longitude;
        let lon1 = self.data[i + 1].longitude;
        let lat0 = self.data[i].latitude;
        let lat1 = self.data[i + 1].latitude;
        let alt0 = self.data[i].altitude;
        let alt1 = self.data[i + 1].altitude;

        let ratio = (time - t0).num_nanoseconds()? as f64 / (t1 - t0).num_nanoseconds()? as f64;

        let lon = lon0 + (lon1 - lon0) * ratio;
        let lat = lat0 + (lat1 - lat0) * ratio;
        let alt = alt0 + (alt1 - alt0) * ratio;

        Some(Location {
            time,
            longitude: lon,
            latitude: lat,
            altitude: alt,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Attitude {
    pub time: DateTime<Utc>,
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
}

#[derive(Debug, Serialize)]
pub struct Signal {
    pub start: DateTime<Utc>,
    pub start_best: DateTime<Utc>,
    pub stop: DateTime<Utc>,
    pub stop_best: DateTime<Utc>,
    pub peak: DateTime<Utc>,
    pub duration: f64,
    pub duration_best: f64,
    pub fp_year: f64,
    pub count: u32,
    pub count_best: u32,
    pub count_filtered: u32,
    pub count_filtered_best: u32,
    pub background: f64,
    pub flux: f64,
    pub flux_best: f64,
    pub flux_filtered: f64,
    pub flux_filtered_best: f64,
    pub veto_proportion: f64,
    pub veto_proportion_best: f64,
    pub veto_proportion_filtered: f64,
    pub veto_proportion_filtered_best: f64,
    pub simultaneous_proportion: f64,
    pub simultaneous_proportion_best: f64,
    pub simultaneous_proportion_filtered: f64,
    pub simultaneous_proportion_filtered_best: f64,
    pub events: Vec<GenericEvent>,
    pub light_curve_1s: Vec<u32>,
    pub light_curve_1s_filtered: Vec<u32>,
    pub light_curve_100ms: Vec<u32>,
    pub light_curve_100ms_filtered: Vec<u32>,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
    pub orbit: Vec<Location>,
    pub lightnings: Vec<LightningAssociation>,
    pub associated_lightning_count: u32,
    pub coincidence_probability: f64,
    pub mean_solar_time: NaiveTime,
    pub apparent_solar_time: NaiveTime,
    pub day_of_year: u32,
    pub month: u32,
    pub solar_zenith_angle: f64,
    pub solar_zenith_angle_at_noon: f64,
    pub solar_azimuth_angle: f64,
}

impl Signal {
    pub fn new(
        start: DateTime<Utc>,
        start_best: DateTime<Utc>,
        stop: DateTime<Utc>,
        stop_best: DateTime<Utc>,
        fp_year: f64,
        count: u32,
        count_best: u32,
        count_filtered: u32,
        count_filtered_best: u32,
        background: f64,
        veto_proportion: f64,
        veto_proportion_best: f64,
        veto_proportion_filtered: f64,
        veto_proportion_filtered_best: f64,
        simultaneous_proportion: f64,
        simultaneous_proportion_best: f64,
        simultaneous_proportion_filtered: f64,
        simultaneous_proportion_filtered_best: f64,
        events: Vec<GenericEvent>,
        light_curve_1s: Vec<u32>,
        light_curve_1s_filtered: Vec<u32>,
        light_curve_100ms: Vec<u32>,
        light_curve_100ms_filtered: Vec<u32>,
        longitude: f64,
        latitude: f64,
        altitude: f64,
        q1: f64,
        q2: f64,
        q3: f64,
        orbit: Vec<Location>,
        lightnings: Vec<LightningAssociation>,
        coincidence_probability: f64,
    ) -> Self {
        let peak = start_best + (stop_best - start_best) / 2;
        let duration = (stop - start).num_nanoseconds().unwrap() as f64 / 1e9;
        let duration_best = (stop_best - start_best).num_nanoseconds().unwrap() as f64 / 1e9;
        let associated_lightning_count =
            lightnings.iter().filter(|l| l.is_associated).count() as u32;
        Self {
            start,
            start_best,
            stop,
            stop_best,
            peak,
            duration,
            duration_best,
            fp_year,
            count,
            count_best,
            count_filtered,
            count_filtered_best,
            background,
            flux: count as f64 / duration,
            flux_best: count_best as f64 / duration_best,
            flux_filtered: count_filtered as f64 / duration,
            flux_filtered_best: count_filtered_best as f64 / duration_best,
            veto_proportion,
            veto_proportion_best,
            veto_proportion_filtered,
            veto_proportion_filtered_best,
            simultaneous_proportion,
            simultaneous_proportion_best,
            simultaneous_proportion_filtered,
            simultaneous_proportion_filtered_best,
            events,
            light_curve_1s,
            light_curve_1s_filtered,
            light_curve_100ms,
            light_curve_100ms_filtered,
            longitude,
            latitude,
            altitude,
            q1,
            q2,
            q3,
            orbit,
            lightnings,
            associated_lightning_count,
            coincidence_probability,
            mean_solar_time: mean_solar_time(peak, longitude),
            apparent_solar_time: apparent_solar_time(peak, longitude),
            day_of_year: day_of_year(peak),
            month: peak.month(),
            solar_zenith_angle: solar_zenith_angle(peak, latitude, longitude),
            solar_zenith_angle_at_noon: solar_zenith_angle_at_noon(peak, latitude),
            solar_azimuth_angle: solar_azimuth_angle(peak, latitude, longitude),
        }
    }
}
