use crate::types::Satellite;

use super::Duration;

pub(crate) trait TimeUnits<T: Satellite> {
    fn seconds(self) -> Duration<T>;
    fn milliseconds(self) -> Duration<T>;
}

impl<T: Satellite> TimeUnits<T> for f64 {
    fn seconds(self) -> Duration<T> {
        Duration::new(self)
    }
    fn milliseconds(self) -> Duration<T> {
        Duration::new(self / 1000.0)
    }
}
