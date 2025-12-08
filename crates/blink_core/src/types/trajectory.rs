use serde::Serialize;

use crate::{
    traits::{Interpolatable, Temporal},
    types::TemporalState,
};

#[derive(Serialize, Debug)]
pub struct Trajectory<Time: Temporal, State: Interpolatable + Clone> {
    pub points: Vec<TemporalState<Time, State>>,
}

impl<Time: Temporal, State: Interpolatable + Clone> Trajectory<Time, State> {
    pub fn interpolate(&self, time: Time) -> Option<TemporalState<Time, State>> {
        let mut i = 0;
        while i < self.points.len() - 1 && self.points[i + 1].timestamp < time {
            i += 1;
        }
        if i == self.points.len() - 1 {
            return None;
        }

        let t0 = self.points[i].timestamp;
        let t1 = self.points[i + 1].timestamp;

        let lerp_factor = time.lerp_factor(t0, t1);

        Some(TemporalState {
            timestamp: time,
            state: self.points[i]
                .state
                .interpolate(&self.points[i + 1].state, lerp_factor),
        })
    }

    pub fn window(&self, time: Time, half_width: Time::Duration) -> Self {
        let start_time = time - half_width;
        let end_time = time + half_width;

        Trajectory {
            points: self
                .points
                .iter()
                .filter(|point| point.timestamp >= start_time && point.timestamp <= end_time)
                .cloned()
                .collect(),
        }
    }
}
