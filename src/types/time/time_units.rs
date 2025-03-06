use crate::types::Satellite;

use super::Span;

pub(crate) trait TimeUnits<T: Satellite> {
    fn seconds(self) -> Span<T>;
    fn milliseconds(self) -> Span<T>;
}

impl<T: Satellite> TimeUnits<T> for f64 {
    fn seconds(self) -> Span<T> {
        Span::new(self)
    }
    fn milliseconds(self) -> Span<T> {
        Span::new(self / 1000.0)
    }
}
