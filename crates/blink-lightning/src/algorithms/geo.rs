use crate::constants::{R_EARTH, SPEED_OF_LIGHT};
use chrono::Duration;
use uom::si::f64::*;
use uom::typenum::P2;

pub fn hav(theta: f64) -> f64 {
    (theta / 2.0).sin().powi(2)
}

pub fn distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Length {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_phi = (lat2 - lat1).to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();

    let a = hav(delta_phi) + phi1.cos() * phi2.cos() * hav(delta_lambda);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    *R_EARTH * c
}

pub fn time_of_arrival(distance: Length, h1: Length, h2: Length) -> Duration {
    let alpha = (distance / *R_EARTH).get::<uom::si::ratio::ratio>();
    let d2 = (*R_EARTH + h1).powi(P2::new()) + (*R_EARTH + h2).powi(P2::new())
        - 2.0 * (*R_EARTH + h1) * (*R_EARTH + h2) * alpha.cos();
    Duration::nanoseconds(
        (d2.sqrt() / *SPEED_OF_LIGHT)
            .get::<uom::si::time::nanosecond>()
            .round() as i64,
    )
}
