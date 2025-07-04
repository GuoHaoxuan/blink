mod define;
mod from_epoch;
mod into_iter;
mod saturation;

pub use define::Instance;

use super::algorithms::continuous;
use crate::{
    lightning::{associated_lightning, coincidence_prob},
    satellites::hxmt::instance::from_epoch::instance_from_epoch,
    search::{
        algorithms::{SearchConfig, search_new},
        lightcurve::light_curve,
    },
    types::{Event, Instance as InstanceTrait, Signal, Span},
};
use anyhow::Result;
use chrono::{TimeDelta, prelude::*};
use itertools::Itertools;

impl InstanceTrait for Instance {
    fn from_epoch(epoch: &DateTime<Utc>) -> Result<Self> {
        instance_from_epoch(epoch)
    }

    fn search(&self) -> Result<Vec<Signal>> {
        let events = self
            .into_iter()
            .filter(|event| event.keep_for_tgf())
            .collect::<Vec<_>>();

        let results = search_new(
            &events,
            1,
            self.span[0],
            self.span[1],
            SearchConfig {
                min_duration: Span::microseconds(0.0),
                max_duration: Span::milliseconds(1.0),
                neighbor: Span::seconds(1.0),
                hollow: Span::milliseconds(10.0),
                fp_year: 20.0,
                min_number: 8,
            },
        );

        let results = continuous(results, Span::seconds(10.0), Span::seconds(1.0), 10);
        let results = results
            .into_iter()
            .filter(|trigger| !self.check_saturation(trigger.start))
            .collect::<Vec<_>>();

        let signals = results.into_iter().filter_map(|trigger| {
            let extended_half_time = Span::milliseconds(500.0);
            let unfiltered_events_extended = self
                .into_iter()
                .filter(|event| {
                    event.time() >= trigger.start - extended_half_time
                        && event.time() <= trigger.stop + extended_half_time
                })
                .collect::<Vec<_>>();
            let filtered_events_extended = unfiltered_events_extended
                .iter()
                .filter(|event| event.keep_for_tgf())
                .collect::<Vec<_>>();
            let (q1, q2, q3) = self
                .att_file
                .interpolate(trigger.start.time.into_inner())
                .unwrap();
            let orbit = self.orbit_file.window(trigger.start, 1000.0);

            return None;
        });

        let signals = results
            .into_iter()
            .filter_map(|trigger| {
                let extend = Span::milliseconds(1.0);
                let original_events_extended = self
                    .into_iter()
                    .filter(|event| {
                        event.time() >= trigger.start - extend
                            && event.time() <= trigger.stop + extend
                    })
                    .collect::<Vec<_>>();
                let filtered_events_extended = original_events_extended
                    .iter()
                    .filter(|event| event.channel() >= CHANNEL_THRESHOLD)
                    .collect::<Vec<_>>();
                if filtered_events_extended.len() >= 100000 {
                    eprintln!(
                        "Too many events({}) in signal: {} - {}",
                        filtered_events_extended.len(),
                        trigger.start.to_chrono(),
                        trigger.stop.to_chrono()
                    );
                    return None;
                }
                let (longitude, latitude, altitude) = self
                    .orbit_file
                    .interpolate(trigger.start.time.into_inner())
                    .unwrap_or((0.0, 0.0, 0.0));
                let (q1, q2, q3) = self
                    .att_file
                    .interpolate(trigger.start.time.into_inner())
                    .unwrap_or((0.0, 0.0, 0.0));
                let time_tolerance = TimeDelta::milliseconds(5);
                let distance_tolerance = 800_000.0;
                let lightning_window = TimeDelta::minutes(2);
                let lightnings = associated_lightning(
                    (trigger.start + trigger.delay + trigger.bin_size_best / 2.0).to_chrono(),
                    latitude,
                    longitude,
                    altitude,
                    time_tolerance,
                    distance_tolerance,
                    lightning_window,
                );
                let original_events = original_events_extended
                    .iter()
                    .filter(|event| event.time() >= trigger.start && event.time() <= trigger.stop)
                    .collect::<Vec<_>>();
                let original_events_best = original_events
                    .iter()
                    .filter(|event| {
                        event.time() >= trigger.start + trigger.delay
                            && event.time() <= trigger.start + trigger.delay + trigger.bin_size_best
                    })
                    .collect::<Vec<_>>();
                let filtered_events = filtered_events_extended
                    .iter()
                    .filter(|event| event.time() >= trigger.start && event.time() <= trigger.stop)
                    .collect::<Vec<_>>();
                let filtered_events_best = filtered_events
                    .iter()
                    .filter(|event| {
                        event.time() >= trigger.start + trigger.delay
                            && event.time() <= trigger.start + trigger.delay + trigger.bin_size_best
                    })
                    .collect::<Vec<_>>();
                let count = original_events.len() as u32;
                let count_best = original_events_best.len() as u32;
                let count_filtered = filtered_events.len() as u32;
                let count_filtered_best = filtered_events_best.len() as u32;
                if true {
                    Some(Signal::new(
                        trigger.start.to_chrono(),
                        (trigger.start + trigger.delay).to_chrono(),
                        trigger.stop.to_chrono(),
                        (trigger.start + trigger.delay + trigger.bin_size_best).to_chrono(),
                        trigger.fp_year(),
                        count,
                        count_best,
                        count_filtered,
                        count_filtered_best,
                        trigger.mean / trigger.bin_size_best.to_seconds(),
                        original_events
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / original_events.len() as f64,
                        original_events_best
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / original_events_best.len() as f64,
                        filtered_events
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / filtered_events.len() as f64,
                        filtered_events_best
                            .iter()
                            .filter(|event| event.detector().acd != 0)
                            .count() as f64
                            / filtered_events_best.len() as f64,
                        1.0 - original_events
                            .iter()
                            .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
                            .filter(|(count, _)| *count == 1)
                            .count() as f64
                            / original_events.len() as f64,
                        1.0 - original_events_best
                            .iter()
                            .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
                            .filter(|(count, _)| *count == 1)
                            .count() as f64
                            / original_events_best.len() as f64,
                        1.0 - filtered_events
                            .iter()
                            .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
                            .filter(|(count, _)| *count == 1)
                            .count() as f64
                            / filtered_events.len() as f64,
                        1.0 - filtered_events_best
                            .iter()
                            .dedup_by_with_count(|a, b| b.time() - a.time() < Span::seconds(0.3e-6))
                            .filter(|(count, _)| *count == 1)
                            .count() as f64
                            / filtered_events_best.len() as f64,
                        original_events_extended
                            .iter()
                            .map(|event| event.to_general())
                            .collect(),
                        light_curve(
                            &self
                                .into_iter()
                                .map(|event| event.time())
                                .collect::<Vec<_>>(),
                            trigger.start - Span::milliseconds(500.0),
                            trigger.start + Span::milliseconds(500.0),
                            Span::milliseconds(10.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        light_curve(
                            &events_time,
                            trigger.start - Span::milliseconds(500.0),
                            trigger.start + Span::milliseconds(500.0),
                            Span::milliseconds(10.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        light_curve(
                            &self
                                .into_iter()
                                .map(|event| event.time())
                                .collect::<Vec<_>>(),
                            trigger.start - Span::milliseconds(50.0),
                            trigger.start + Span::milliseconds(50.0),
                            Span::milliseconds(1.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        light_curve(
                            &events_time,
                            trigger.start - Span::milliseconds(50.0),
                            trigger.start + Span::milliseconds(50.0),
                            Span::milliseconds(1.0),
                        )
                        .into_iter()
                        .take(100)
                        .collect::<Vec<_>>(),
                        longitude,
                        latitude,
                        altitude,
                        q1,
                        q2,
                        q3,
                        self.orbit_file.window(trigger.start, 1000.0),
                        lightnings,
                        coincidence_prob(
                            (trigger.start + trigger.delay + trigger.bin_size_best / 2.0)
                                .to_chrono(),
                            latitude,
                            longitude,
                            altitude,
                            time_tolerance,
                            distance_tolerance,
                            lightning_window,
                        ),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(signals)
    }
}
