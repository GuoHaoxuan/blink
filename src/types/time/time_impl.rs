use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    ops::{Add, Sub},
};

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

    pub(crate) fn to_hifitime(self) -> hifitime::Epoch {
        *T::ref_time() + hifitime::Duration::from_seconds(self.time.into_inner())
    }
}

impl<S: Satellite> From<hifitime::Epoch> for Time<S> {
    fn from(value: hifitime::Epoch) -> Self {
        Self::new((value - *S::ref_time()).to_seconds())
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
