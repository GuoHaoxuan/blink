mod time_units;

use ordered_float::NotNan;
use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    ops::{Add, Div, Sub},
};

use serde::Serialize;

use super::Satellite;

pub(crate) use time_units::TimeUnits;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) struct Epoch<T: Satellite> {
    pub(crate) time: NotNan<f64>,
    _phantom: PhantomData<T>,
}

impl<T: Satellite> Debug for Epoch<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.to_hifitime().fmt(f)
    }
}

impl<T: Satellite> Serialize for Epoch<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_hifitime().serialize(serializer)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub(crate) struct Duration<T: Satellite> {
    time: NotNan<f64>,
    _phantom: PhantomData<T>,
}

impl<T: Satellite> Epoch<T> {
    pub(crate) fn new(time: f64) -> Self {
        Self {
            time: NotNan::new(time).unwrap(),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn to_hifitime(self) -> hifitime::Epoch {
        *T::ref_time() + hifitime::Duration::from_seconds(self.time.into_inner())
    }
}

impl<T: Satellite> Duration<T> {
    pub(crate) fn new(time: f64) -> Self {
        Self {
            time: NotNan::new(time).unwrap(),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn to_seconds(self) -> f64 {
        (self.time).into_inner()
    }
}

impl<T: Satellite> Sub for Epoch<T> {
    type Output = Duration<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration {
            time: self.time - rhs.time,
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Sub<Duration<T>> for Epoch<T> {
    type Output = Self;

    fn sub(self, rhs: Duration<T>) -> Self::Output {
        Self {
            time: self.time - rhs.time,
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Add<Duration<T>> for Epoch<T> {
    type Output = Self;

    fn add(self, rhs: Duration<T>) -> Self::Output {
        Self {
            time: self.time + rhs.time,
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Div<f64> for Duration<T> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            time: NotNan::new((self.time).into_inner() / rhs).unwrap(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Div<Duration<T>> for Duration<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            time: NotNan::new((self.time).into_inner() / (rhs.time).into_inner()).unwrap(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Sub<Duration<T>> for Duration<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            time: self.time - rhs.time,
            _phantom: PhantomData,
        }
    }
}
