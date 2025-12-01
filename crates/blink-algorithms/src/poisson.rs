use statrs::distribution::{DiscreteCDF, Poisson};
use uom::si::f64::*;

const DAYS_1_YEAR: f64 = 365.25;

pub fn sf(mean: f64, count: u32) -> f64 {
    match (mean, count) {
        (0.0, 0) => 0.0,
        (0.0, _) => 1.0,
        _ => Poisson::new(mean)
            .inspect_err(|e| {
                eprintln!("Error in Poisson distribution: {}", e);
                eprintln!("Mean: {}", mean);
            })
            .unwrap()
            .sf(count as u64),
    }
}

pub fn false_positive_per_year(sf: f64, duration: Time) -> f64 {
    sf * (Time::new::<uom::si::time::second>(3600.0 * 24.0 * DAYS_1_YEAR) / duration)
        .get::<uom::si::ratio::ratio>()
}
