use hifitime::prelude::*;

use super::interval::Interval;

trait Peroid: Iterator {
    fn gti() -> Vec<Interval<Epoch>>;
}
