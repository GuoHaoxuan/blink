use crate::algorithms::geo::distance;
use crate::algorithms::geo::time_of_arrival;
use crate::constants::LIGHTNING_ALTITUDE;
use crate::types::Lightning;
use blink_core::types::Position;
use blink_core::types::TemporalState;
use chrono::Duration;
use chrono::prelude::*;
use uom::si::f64::*;

impl Lightning {
    pub fn is_associated(
        &self,
        position: &TemporalState<DateTime<Utc>, Position>,
        time_tolerance: Duration,
        distance_tolerance: Length,
    ) -> bool {
        let dist = distance(
            position.state.latitude,
            position.state.longitude,
            self.lat,
            self.lon,
        );
        let time_of_arrival_value =
            time_of_arrival(dist, position.state.altitude, *LIGHTNING_ALTITUDE);
        let fixed_time = position.timestamp - time_of_arrival_value;
        let time_delta = self.time - fixed_time;
        time_delta.abs() <= time_tolerance && dist <= distance_tolerance
    }
}
