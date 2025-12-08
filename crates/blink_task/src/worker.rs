use crate::db::{fail_task, finish_task, get_task, write_signal};
use crate::types::Record;
use blink_algorithms::light_curve::light_curve_chrono;
use blink_core::error::Error;
use blink_core::traits::Chunk;
use blink_core::traits::Event;
use blink_core::types::{Position, TemporalState, Trajectory};
use blink_solar::{
    apparent_solar_time, day_of_year, mean_solar_time, solar_azimuth_angle, solar_zenith_angle,
    solar_zenith_angle_at_noon,
};
use chrono::prelude::*;
use chrono::{DateTime, TimeDelta};
use rusqlite::Connection;

pub fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let worker = format!("{}:{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();

    // consume tasks
    while let Some((time, satellite, detector)) = get_task(&conn, &worker) {
        match (satellite.as_str(), detector.as_str()) {
            ("HXMT", "HE") => {
                let chunk = blink_hxmt_he::types::Chunk::from_epoch(&time);
                match chunk {
                    Ok(chunk) => {
                        let signals = chunk
                            .search()
                            .iter()
                            .filter_map(|signal| {
                                let peak_time =
                                    signal.start + signal.delay + signal.bin_size_best / 2.0;
                                let position = TemporalState::<DateTime<Utc>, Position> {
                                    timestamp: peak_time.into(),
                                    state: signal.orbit.interpolate(peak_time)?.state,
                                };
                                let lightnings = blink_lightning::database::get_lightnings(
                                    peak_time.to_utc() - TimeDelta::minutes(1),
                                    peak_time.to_utc() + TimeDelta::minutes(1),
                                )
                                .into_iter()
                                .map(|lightning| {
                                    let is_associated = lightning.is_associated(
                                        &position,
                                        TimeDelta::milliseconds(5),
                                        uom::si::f64::Length::new::<uom::si::length::meter>(
                                            800_000.0,
                                        ),
                                    );
                                    (lightning, is_associated)
                                })
                                .collect::<Vec<_>>();

                                let start_full = signal.start.to_utc();
                                let start_best = (signal.start + signal.delay).to_utc();
                                let stop_full = signal.stop.to_utc();
                                let stop_best =
                                    (signal.start + signal.delay + signal.bin_size_best).to_utc();
                                let peak = peak_time.to_utc();
                                let duration_full =
                                    (signal.stop - signal.start).get::<uom::si::time::second>();
                                let duration_best =
                                    signal.bin_size_best.get::<uom::si::time::second>();

                                let events_filtered = signal
                                    .events
                                    .iter()
                                    .filter(|event| event.keep())
                                    .cloned()
                                    .collect::<Vec<_>>();
                                let events_unfiltered_full = signal
                                    .events
                                    .iter()
                                    .filter(|event| {
                                        event.time().to_utc() >= start_full
                                            && event.time().to_utc() <= stop_full
                                    })
                                    .cloned()
                                    .collect::<Vec<_>>();
                                let events_unfiltered_best = signal
                                    .events
                                    .iter()
                                    .filter(|event| {
                                        event.time().to_utc() >= start_best
                                            && event.time().to_utc() <= stop_best
                                    })
                                    .cloned()
                                    .collect::<Vec<_>>();
                                let events_filtered_full = events_unfiltered_full
                                    .iter()
                                    .filter(|event| event.keep())
                                    .cloned()
                                    .collect::<Vec<_>>();
                                let events_filtered_best = events_unfiltered_best
                                    .iter()
                                    .filter(|event| event.keep())
                                    .cloned()
                                    .collect::<Vec<_>>();

                                if events_filtered_full.len() >= 100_000 {
                                    eprintln!(
                                        "Too many events({}) in signal: {} - {}",
                                        events_filtered_full.len(),
                                        signal.start.to_utc(),
                                        signal.stop.to_utc()
                                    );
                                    return None;
                                }

                                let count_unfiltered_full = events_unfiltered_full.len() as u32;
                                let count_unfiltered_best = events_unfiltered_best.len() as u32;
                                let count_filtered_full = events_filtered_full.len() as u32;
                                let count_filtered_best = events_filtered_best.len() as u32;

                                let light_curve_1s_unfiltered = light_curve_chrono(
                                    &signal
                                        .events
                                        .iter()
                                        .map(|event| event.time().to_utc())
                                        .collect::<Vec<_>>(),
                                    start_full - TimeDelta::milliseconds(500),
                                    stop_full + TimeDelta::milliseconds(500),
                                    TimeDelta::milliseconds(10),
                                )
                                .into_iter()
                                .take(100)
                                .collect::<Vec<_>>();
                                let light_curve_1s_filtered = light_curve_chrono(
                                    &events_filtered
                                        .iter()
                                        .map(|event| event.time().to_utc())
                                        .collect::<Vec<_>>(),
                                    start_full - TimeDelta::milliseconds(500),
                                    stop_full + TimeDelta::milliseconds(500),
                                    TimeDelta::milliseconds(10),
                                )
                                .into_iter()
                                .take(100)
                                .collect::<Vec<_>>();
                                let light_curve_100ms_unfiltered = light_curve_chrono(
                                    &signal
                                        .events
                                        .iter()
                                        .map(|event| event.time().to_utc())
                                        .collect::<Vec<_>>(),
                                    start_full - TimeDelta::milliseconds(50),
                                    stop_full + TimeDelta::milliseconds(50),
                                    TimeDelta::milliseconds(1),
                                )
                                .into_iter()
                                .take(100)
                                .collect::<Vec<_>>();
                                let light_curve_100ms_filtered = light_curve_chrono(
                                    &events_filtered
                                        .iter()
                                        .map(|event| event.time().to_utc())
                                        .collect::<Vec<_>>(),
                                    start_full - TimeDelta::milliseconds(50),
                                    stop_full + TimeDelta::milliseconds(50),
                                    TimeDelta::milliseconds(1),
                                )
                                .into_iter()
                                .take(100)
                                .collect::<Vec<_>>();

                                let associated_lightning_count = lightnings
                                    .iter()
                                    .filter(|(_, associated)| *associated)
                                    .count()
                                    as u32;
                                let coincidence_probability =
                                    blink_lightning::algorithms::coincidence_prob(
                                        &position,
                                        TimeDelta::milliseconds(5),
                                        uom::si::f64::Length::new::<uom::si::length::meter>(
                                            800_000.0,
                                        ),
                                        TimeDelta::minutes(2),
                                    );

                                Some(Record {
                                    start_full,
                                    start_best,
                                    stop_full,
                                    stop_best,
                                    peak,
                                    duration_full,
                                    duration_best,
                                    false_positive: signal.sf,
                                    false_positive_per_year: signal.false_positive_per_year,
                                    count_unfiltered_full,
                                    count_unfiltered_best,
                                    count_filtered_full,
                                    count_filtered_best,
                                    background: signal.mean / duration_best,
                                    flux_unfiltered_full: count_unfiltered_full as f64
                                        / duration_full,
                                    flux_unfiltered_best: count_unfiltered_best as f64
                                        / duration_best,
                                    flux_filtered_full: count_filtered_full as f64 / duration_full,
                                    flux_filtered_best: count_filtered_best as f64 / duration_best,
                                    events: signal
                                        .events
                                        .iter()
                                        .filter(|event| {
                                            event.time().to_utc()
                                                >= start_full - TimeDelta::milliseconds(1)
                                                && event.time().to_utc()
                                                    <= stop_full + TimeDelta::milliseconds(1)
                                        })
                                        .cloned()
                                        .collect(),
                                    light_curve_1s_unfiltered,
                                    light_curve_1s_filtered,
                                    light_curve_100ms_unfiltered,
                                    light_curve_100ms_filtered,
                                    longitude: position.state.longitude,
                                    latitude: position.state.latitude,
                                    altitude: position
                                        .state
                                        .altitude
                                        .get::<uom::si::length::meter>(),
                                    q1: signal.attitude.state.q1,
                                    q2: signal.attitude.state.q2,
                                    q3: signal.attitude.state.q3,
                                    orbit: Trajectory {
                                        points: signal
                                            .orbit
                                            .points
                                            .iter()
                                            .map(|point| TemporalState {
                                                timestamp: point.timestamp.to_utc(),
                                                state: position.state.clone(),
                                            })
                                            .collect(),
                                    },
                                    lightnings,
                                    associated_lightning_count,
                                    coincidence_probability,
                                    mean_solar_time: mean_solar_time(
                                        peak,
                                        position.state.longitude,
                                    ),
                                    apparent_solar_time: apparent_solar_time(
                                        peak,
                                        position.state.longitude,
                                    ),
                                    day_of_year: day_of_year(peak),
                                    month: peak.month(),
                                    solar_zenith_angle: solar_zenith_angle(
                                        peak,
                                        position.state.latitude,
                                        position.state.longitude,
                                    ),
                                    solar_zenith_angle_at_noon: solar_zenith_angle_at_noon(
                                        peak,
                                        position.state.latitude,
                                    ),
                                    solar_azimuth_angle: solar_azimuth_angle(
                                        peak,
                                        position.state.latitude,
                                        position.state.longitude,
                                    ),
                                })
                            })
                            .collect::<Vec<_>>();
                        for signal in signals {
                            write_signal(&conn, &signal, &satellite, &detector);
                        }
                        finish_task(&conn, &time, &satellite, &detector);
                    }
                    Err(e) => {
                        fail_task(&conn, &time, &satellite, &detector, e);
                    }
                }
            }
            _ => fail_task(
                &conn,
                &time,
                &satellite,
                &detector,
                Error::UnknownDetector(format!("{}/{}", satellite, detector)),
            ),
        }
    }
}
