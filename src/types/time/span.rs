use std::{
    marker::PhantomData,
    ops::{Div, Sub},
};

use ordered_float::NotNan;

use crate::types::Satellite;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub(crate) struct Span<T: Satellite> {
    pub(super) time: NotNan<f64>,
    pub(super) _phantom: PhantomData<T>,
}

impl<T: Satellite> Span<T> {
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

impl<T: Satellite> Div<f64> for Span<T> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            time: NotNan::new((self.time).into_inner() / rhs).unwrap(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Div<Span<T>> for Span<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            time: NotNan::new((self.time).into_inner() / (rhs.time).into_inner()).unwrap(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Sub<Span<T>> for Span<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            time: self.time - rhs.time,
            _phantom: PhantomData,
        }
    }
}
