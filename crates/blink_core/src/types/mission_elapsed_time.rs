use crate::traits::Satellite;
use chrono::{Duration, prelude::*};
use std::cmp::Ordering;
use std::ops::{Add, Sub};
use std::{marker::PhantomData, sync::LazyLock};
use uom::si::f64::*;
use uom::si::time::second;

#[derive(Clone, Copy, PartialEq)]
pub struct MissionElapsedTime<S: Satellite> {
    time: Time,
    _phantom: PhantomData<S>,
}

impl<S: Satellite> Eq for MissionElapsedTime<S> {}

impl<S: Satellite> PartialOrd for MissionElapsedTime<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: Satellite> Ord for MissionElapsedTime<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .partial_cmp(&other.time)
            .unwrap_or(Ordering::Equal)
    }
}

impl<S: Satellite> MissionElapsedTime<S> {
    pub fn new(met: f64) -> Self {
        Self {
            time: Time::new::<second>(met),
            _phantom: PhantomData,
        }
    }

    pub fn time(&self) -> Time {
        self.time
    }
}

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

impl<S: Satellite> From<MissionElapsedTime<S>> for DateTime<Utc> {
    fn from(val: MissionElapsedTime<S>) -> Self {
        let seconds = val.time.get::<second>();
        let whole_seconds = seconds.trunc() as i64;
        let nanoseconds = ((seconds.fract() * 1_000_000_000.0) as i64).clamp(0, 999_999_999);

        let mut time =
            *S::ref_time() + Duration::seconds(whole_seconds) + Duration::nanoseconds(nanoseconds);
        for leap_second in LEAP_SECONDS.iter() {
            if *S::ref_time() < *leap_second && time > *leap_second {
                time -= Duration::seconds(1);
            }
        }
        time
    }
}

impl<S: Satellite> From<DateTime<Utc>> for MissionElapsedTime<S> {
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
        MissionElapsedTime {
            time: Time::new::<second>(time),
            _phantom: PhantomData,
        }
    }
}

impl<S: Satellite> Sub for MissionElapsedTime<S> {
    type Output = Time;

    fn sub(self, rhs: Self) -> Self::Output {
        self.time - rhs.time
    }
}

impl<S: Satellite> Sub<Time> for MissionElapsedTime<S> {
    type Output = Self;

    fn sub(self, rhs: Time) -> Self::Output {
        MissionElapsedTime {
            time: self.time - rhs,
            _phantom: PhantomData,
        }
    }
}

impl<S: Satellite> Add<Time> for MissionElapsedTime<S> {
    type Output = Self;

    fn add(self, rhs: Time) -> Self::Output {
        MissionElapsedTime {
            time: self.time + rhs,
            _phantom: PhantomData,
        }
    }
}
