use crate::{traits::Interpolatable, types::TemporalState};
use chrono::prelude::*;

pub struct Trajectory<T: Interpolatable> {
    pub points: Vec<TemporalState<T>>,
}

impl<T: Interpolatable> Trajectory<T> {
    pub fn interpolate(&self, time: DateTime<Utc>) -> Option<TemporalState<T>> {
        let mut i = 0;
        while i < self.points.len() - 1 && self.points[i + 1].timestamp < time {
            i += 1;
        }
        if i == self.points.len() - 1 {
            return None;
        }

        let t0 = self.points[i].timestamp;
        let t1 = self.points[i + 1].timestamp;

        let ratio = (time - t0).num_nanoseconds()? as f64 / (t1 - t0).num_nanoseconds()? as f64;

        Some(TemporalState {
            timestamp: time,
            state: self.points[i]
                .state
                .interpolate(&self.points[i + 1].state, ratio),
        })
    }
}
