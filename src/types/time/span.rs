use std::{
    marker::PhantomData,
    ops::{Div, Mul, MulAssign, Sub},
};

use ordered_float::NotNan;
use serde::Serialize;

use crate::types::Satellite;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub struct Span<T: Satellite> {
    pub(super) time: NotNan<f64>,
    pub(super) _phantom: PhantomData<T>,
}

impl<T: Satellite> Span<T> {
    pub fn seconds(seconds: f64) -> Self {
        Self {
            time: NotNan::new(seconds).unwrap(),
            _phantom: PhantomData,
        }
    }

    pub fn milliseconds(milliseconds: f64) -> Self {
        Self {
            time: NotNan::new(milliseconds / 1000.0).unwrap(),
            _phantom: PhantomData,
        }
    }

    pub fn microseconds(microseconds: f64) -> Self {
        Self {
            time: NotNan::new(microseconds / 1_000_000.0).unwrap(),
            _phantom: PhantomData,
        }
    }

    pub fn to_seconds(self) -> f64 {
        (self.time).into_inner()
    }

    pub fn to_nanoseconds(self) -> f64 {
        (self.time).into_inner() * 1e9
    }

    pub fn to_chrono(self) -> chrono::TimeDelta {
        chrono::Duration::nanoseconds((self.time).into_inner() as i64 * 1e9 as i64)
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

impl<T: Satellite> Div for Span<T> {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        (self.time).into_inner() / (rhs.time).into_inner()
    }
}

impl<T: Satellite> Sub for Span<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            time: self.time - rhs.time,
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> Mul<f64> for Span<T> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            time: NotNan::new((self.time).into_inner() * rhs).unwrap(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Satellite> MulAssign<f64> for Span<T> {
    fn mul_assign(&mut self, rhs: f64) {
        self.time = NotNan::new((self.time).into_inner() * rhs).unwrap();
    }
}
