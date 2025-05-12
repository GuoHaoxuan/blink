use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    ops::{Add, Sub},
    sync::LazyLock,
};

use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};
use ordered_float::NotNan;
use serde::Serialize;

use crate::types::Satellite;

use super::span::Span;

static LEAP_SECONDS: LazyLock<[DateTime<Utc>; 27]> = LazyLock::new(|| {
    [
        Utc.with_ymd_and_hms(1972, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1972, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1973, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1974, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1975, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1976, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1977, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1978, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1979, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1981, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1982, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1983, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1985, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1987, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1989, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1990, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1992, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1993, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1994, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1995, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1997, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(1998, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(2005, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(2008, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(2012, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(2015, 6, 30, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
        Utc.with_ymd_and_hms(2016, 12, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(1_000_000_000)
            .unwrap(),
    ]
});

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Time<T: Satellite> {
    pub time: NotNan<f64>,
    _phantom: PhantomData<T>,
}

impl<T: Satellite> Time<T> {
    pub fn to_chrono(self) -> DateTime<Utc> {
        let seconds = self.time.into_inner();
        let whole_seconds = seconds.trunc() as i64;
        let nanoseconds = ((seconds.fract() * 1_000_000_000.0) as i64).clamp(0, 999_999_999);

        let mut time =
            *T::ref_time() + Duration::seconds(whole_seconds) + Duration::nanoseconds(nanoseconds);
        for leap_second in LEAP_SECONDS.iter() {
            if *T::ref_time() < *leap_second && time > *leap_second {
                time -= Duration::seconds(1);
            }
        }
        time
    }

    pub fn seconds(seconds: f64) -> Self {
        Self {
            time: NotNan::new(seconds).unwrap(),
            _phantom: PhantomData,
        }
    }
}

impl<S: Satellite> From<DateTime<Utc>> for Time<S> {
    fn from(value: DateTime<Utc>) -> Self {
        let duration = value - *S::ref_time();
        let seconds = duration.num_seconds() as f64;
        let nanoseconds = duration.subsec_nanos() as f64 / 1_000_000_000.0;

        let mut time = seconds + nanoseconds;
        for leap_second in LEAP_SECONDS.iter() {
            if *S::ref_time() < *leap_second && value > *leap_second {
                time += 1.0;
            }
        }
        Self::seconds(time)
    }
}

impl<T: Satellite> Debug for Time<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.to_chrono().fmt(f)
    }
}

impl<T: Satellite> Serialize for Time<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_chrono().serialize(serializer)
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
        Self::seconds(self.time.into_inner() - rhs.time.into_inner())
    }
}

impl<T: Satellite> Add<Span<T>> for Time<T> {
    type Output = Self;

    fn add(self, rhs: Span<T>) -> Self::Output {
        Self::seconds(self.time.into_inner() + rhs.time.into_inner())
    }
}
