use crate::{
    env::DAYS_1_YEAR,
    types::{Satellite, Span},
};
use statrs::distribution::{DiscreteCDF, Poisson};

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

pub fn false_positive_per_year<S: Satellite>(sf: f64, duration: Span<S>) -> f64 {
    sf * (Span::seconds(3600.0 * 24.0 * DAYS_1_YEAR) / duration)
}
