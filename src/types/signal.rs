use crate::algorithms::lightcurve::light_curve_chrono;
use crate::algorithms::trigger::Trigger;
use crate::lightning::{LightningAssociation, associated_lightning, coincidence_prob};
use crate::solar::{
    apparent_solar_time, day_of_year, mean_solar_time, solar_azimuth_angle, solar_zenith_angle,
    solar_zenith_angle_at_noon,
};
use crate::types::Satellite;
use chrono::{TimeDelta, prelude::*};
use serde::{Deserialize, Serialize};

use super::GenericEvent;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Location {
    pub time: DateTime<Utc>,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationList {
    pub data: Vec<Location>,
}

impl LocationList {
    pub fn interpolate(&self, time: DateTime<Utc>) -> Option<Location> {
        println!(
            "[DEBUG] Interpolating location for time: {}",
            time.to_rfc3339()
        );
        println!("[DEBUG] Location data length: {}", self.data.len());
        println!(
            "[DEBUG] Time range: {} - {}",
            self.data.first()?.time.to_rfc3339(),
            self.data.last()?.time.to_rfc3339()
        );
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
    pub start_full: DateTime<Utc>,
    pub start_best: DateTime<Utc>,
    pub stop_full: DateTime<Utc>,
    pub stop_best: DateTime<Utc>,
    pub peak: DateTime<Utc>,
    pub duration_full: f64,
    pub duration_best: f64,
    pub false_positive: f64,
    pub false_positive_per_year: f64,
    pub count_unfiltered_full: u32,
    pub count_unfiltered_best: u32,
    pub count_filtered_full: u32,
    pub count_filtered_best: u32,
    pub background: f64,
    pub flux_unfiltered_full: f64,
    pub flux_unfiltered_best: f64,
    pub flux_filtered_full: f64,
    pub flux_filtered_best: f64,
    pub events: Vec<GenericEvent>,
    pub light_curve_1s_unfiltered: Vec<u32>,
    pub light_curve_1s_filtered: Vec<u32>,
    pub light_curve_100ms_unfiltered: Vec<u32>,
    pub light_curve_100ms_filtered: Vec<u32>,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
    pub orbit: LocationList,
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
    pub fn new<S: Satellite>(
        trigger: Trigger<S>,
        events: Vec<GenericEvent>,
        attitude: Attitude,
        orbit: LocationList,
    ) -> Option<Self> {
        let events_filtered: Vec<GenericEvent> =
            events.iter().filter(|event| event.keep).cloned().collect();

        let start_full = trigger.start.to_chrono();
        let start_best = (trigger.start + trigger.delay).to_chrono();
        let stop_full = trigger.stop.to_chrono();
        let stop_best = (trigger.start + trigger.delay + trigger.bin_size_best).to_chrono();
        let peak = (trigger.start + trigger.delay + trigger.bin_size_best / 2.0).to_chrono();
        let duration_full = (trigger.stop - trigger.start).to_seconds();
        let duration_best = trigger.bin_size_best.to_seconds();

        let events_unfiltered_full: Vec<GenericEvent> = events
            .iter()
            .filter(|event| event.time >= start_full && event.time <= stop_full)
            .cloned()
            .collect();
        let events_unfiltered_best: Vec<GenericEvent> = events_unfiltered_full
            .iter()
            .filter(|event| event.time >= start_best && event.time <= stop_best)
            .cloned()
            .collect();
        let events_filtered_full: Vec<GenericEvent> = events_unfiltered_full
            .iter()
            .filter(|event| event.keep)
            .cloned()
            .collect();
        let events_filtered_best: Vec<GenericEvent> = events_unfiltered_best
            .iter()
            .filter(|event| event.keep)
            .cloned()
            .collect();

        if events_filtered_full.len() >= 100_000 {
            eprintln!(
                "Too many events({}) in signal: {} - {}",
                events_filtered_full.len(),
                trigger.start.to_chrono(),
                trigger.stop.to_chrono()
            );
            return None;
        }

        let count_unfiltered_full = events_unfiltered_full.len() as u32;
        let count_unfiltered_best = events_unfiltered_best.len() as u32;
        let count_filtered_full = events_filtered_full.len() as u32;
        let count_filtered_best = events_filtered_best.len() as u32;

        let light_curve_1s_unfiltered = light_curve_chrono(
            &events.iter().map(|event| event.time).collect::<Vec<_>>(),
            start_full - TimeDelta::milliseconds(500),
            stop_full + TimeDelta::milliseconds(500),
            TimeDelta::milliseconds(10),
        );
        let light_curve_1s_filtered = light_curve_chrono(
            &events_filtered
                .iter()
                .map(|event| event.time)
                .collect::<Vec<_>>(),
            start_full - TimeDelta::milliseconds(500),
            stop_full + TimeDelta::milliseconds(500),
            TimeDelta::milliseconds(10),
        );
        let light_curve_100ms_unfiltered = light_curve_chrono(
            &events.iter().map(|event| event.time).collect::<Vec<_>>(),
            start_full - TimeDelta::milliseconds(50),
            stop_full + TimeDelta::milliseconds(50),
            TimeDelta::milliseconds(1),
        );
        let light_curve_100ms_filtered = light_curve_chrono(
            &events_filtered
                .iter()
                .map(|event| event.time)
                .collect::<Vec<_>>(),
            start_full - TimeDelta::milliseconds(50),
            stop_full + TimeDelta::milliseconds(50),
            TimeDelta::milliseconds(1),
        );

        println!("[DEBUG] Signal: {} - {}", start_full, stop_full);

        let location = orbit.interpolate(peak)?;
        println!(
            "[DEBUG] Location: {} - {}, {}, {}",
            location.time, location.longitude, location.latitude, location.altitude
        );
        let lightnings = associated_lightning(
            location,
            TimeDelta::milliseconds(5),
            800_000.0,
            TimeDelta::minutes(2),
        );
        let associated_lightning_count = lightnings
            .iter()
            .filter(|lightning| lightning.is_associated)
            .count() as u32;
        let coincidence_probability = coincidence_prob(
            location,
            TimeDelta::milliseconds(5),
            800_000.0,
            TimeDelta::minutes(2),
        );

        Some(Signal {
            start_full,
            start_best,
            stop_full,
            stop_best,
            peak,
            duration_full,
            duration_best,
            false_positive: trigger.sf(),
            false_positive_per_year: trigger.false_positive_per_year(),
            count_unfiltered_full,
            count_unfiltered_best,
            count_filtered_full,
            count_filtered_best,
            background: trigger.mean / duration_best,
            flux_unfiltered_full: count_unfiltered_full as f64 / duration_full,
            flux_unfiltered_best: count_unfiltered_best as f64 / duration_best,
            flux_filtered_full: count_filtered_full as f64 / duration_full,
            flux_filtered_best: count_filtered_best as f64 / duration_best,
            events,
            light_curve_1s_unfiltered,
            light_curve_1s_filtered,
            light_curve_100ms_unfiltered,
            light_curve_100ms_filtered,
            longitude: location.longitude,
            latitude: location.latitude,
            altitude: location.altitude,
            q1: attitude.q1,
            q2: attitude.q2,
            q3: attitude.q3,
            orbit,
            lightnings,
            associated_lightning_count,
            coincidence_probability,
            mean_solar_time: mean_solar_time(peak, location.longitude),
            apparent_solar_time: apparent_solar_time(peak, location.longitude),
            day_of_year: day_of_year(peak),
            month: peak.month(),
            solar_zenith_angle: solar_zenith_angle(peak, location.latitude, location.longitude),
            solar_zenith_angle_at_noon: solar_zenith_angle_at_noon(peak, location.latitude),
            solar_azimuth_angle: solar_azimuth_angle(peak, location.latitude, location.longitude),
        })
    }
}
