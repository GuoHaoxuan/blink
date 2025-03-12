use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    ops::{Add, Sub},
};

use chrono::{DateTime, Duration, Utc};
use ordered_float::NotNan;
use serde::Serialize;

use crate::types::Satellite;

use super::span::Span;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) struct Time<T: Satellite> {
    pub(crate) time: NotNan<f64>,
    _phantom: PhantomData<T>,
}

impl<T: Satellite> Time<T> {
    pub(crate) fn new(time: f64) -> Self {
        Self {
            time: NotNan::new(time).unwrap(),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn to_hifitime(self) -> DateTime<Utc> {
        let seconds = self.time.into_inner();
        let whole_seconds = seconds.trunc() as i64;
        let nanoseconds = ((seconds.fract() * 1_000_000_000.0) as i64)
            .max(0)
            .min(999_999_999);

        *T::ref_time() + Duration::seconds(whole_seconds) + Duration::nanoseconds(nanoseconds)
    }
}

impl<S: Satellite> From<DateTime<Utc>> for Time<S> {
    fn from(value: DateTime<Utc>) -> Self {
        let duration = value - *S::ref_time();
        let seconds = duration.num_seconds() as f64;
        let nanoseconds = duration.subsec_nanos() as f64 / 1_000_000_000.0;
        Self::new(seconds + nanoseconds)
    }
}

impl<S: Satellite> From<NotNan<f64>> for Time<S> {
    fn from(value: NotNan<f64>) -> Self {
        Self::new(value.into_inner())
    }
}

impl<T: Satellite> Debug for Time<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.to_hifitime().fmt(f)
    }
}

impl<T: Satellite> Serialize for Time<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_hifitime().serialize(serializer)
    }
}

impl<T: Satellite> Sub for Time<T> {
    type Output = Span<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        Span {
            time: self.time - rhs.time,
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Sub<Span<T>> for Time<T> {
    type Output = Self;

    fn sub(self, rhs: Span<T>) -> Self::Output {
        Self::from(self.time - rhs.time)
    }
}

impl<T: Satellite> Add<Span<T>> for Time<T> {
    type Output = Self;

    fn add(self, rhs: Span<T>) -> Self::Output {
        Self::from(self.time + rhs.time)
    }
}
