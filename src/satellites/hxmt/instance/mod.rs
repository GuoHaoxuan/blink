mod define;
mod from_epoch;
mod into_iter;
mod saturation;

pub use define::Instance;

use super::algorithms::continuous;
use crate::{
    algorithms::{
        algorithms::{SearchConfig, search_new},
        lightcurve::light_curve,
    },
    lightning::{associated_lightning, coincidence_prob},
    satellites::hxmt::instance::from_epoch::instance_from_epoch,
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
            .filter(|event| event.keep())
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
                false_positive_per_year: 20.0,
                min_number: 8,
            },
        );

        for result in &results {
            println!(
                "Found candidate: start = {} s, stop = {} s, count = {}, sf = {:.3}, FPPY = {:.3}",
                result.start.to_chrono(),
                result.stop.to_chrono(),
                result.count,
                result.sf(),
                result.false_positive_per_year()
            );
        }

        let results = continuous(results, Span::seconds(10.0), Span::seconds(1.0), 10);
        println!("After continuous merging:");
        for result in &results {
            println!(
                "Found candidate: start = {} s, stop = {} s, count = {}, sf = {:.3}, FPPY = {:.3}",
                result.start.to_chrono(),
                result.stop.to_chrono(),
                result.count,
                result.sf(),
                result.false_positive_per_year()
            );
        }

        let results = results
            .into_iter()
            .filter(|trigger| !self.check_saturation(trigger.start))
            .collect::<Vec<_>>();

        println!("After saturation check:");
        for result in &results {
            println!(
                "Found candidate: start = {} s, stop = {} s, count = {}, sf = {:.3}, FPPY = {:.3}",
                result.start.to_chrono(),
                result.stop.to_chrono(),
                result.count,
                result.sf(),
                result.false_positive_per_year()
            );
        }

        let signals = results
            .into_iter()
            .filter_map(|trigger| {
                let extended_half_time = Span::milliseconds(500.0);
                let events = self
                    .into_iter()
                    .filter(|event| {
                        event.time() >= trigger.start - extended_half_time
                            && event.time() <= trigger.stop + extended_half_time
                    })
                    .map(|event| event.to_general())
                    .collect::<Vec<_>>();
                let attitude = self.att_file.interpolate(trigger.start);
                let orbit = self.orbit_file.window(trigger.start, 1000.0);

                Signal::new(trigger, events, attitude, orbit)
            })
            .collect::<Vec<_>>();

        Ok(signals)
    }
}
