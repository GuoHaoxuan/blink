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
