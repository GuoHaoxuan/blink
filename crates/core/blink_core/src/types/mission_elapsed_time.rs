use crate::traits::Instrument;
use chrono::{Duration, prelude::*};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};
use std::{marker::PhantomData, sync::LazyLock};
use uom::si::f64::*;
use uom::si::time::second;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub struct MissionElapsedTime<I: Instrument> {
    time: Time,
    _phantom: PhantomData<I>,
}

impl<I: Instrument> Eq for MissionElapsedTime<I> {}

impl<I: Instrument> PartialOrd for MissionElapsedTime<I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: Instrument> Ord for MissionElapsedTime<I> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .partial_cmp(&other.time)
            .unwrap_or(Ordering::Equal)
    }
}

impl<I: Instrument> MissionElapsedTime<I> {
    pub fn new(met: f64) -> Self {
        Self {
            time: Time::new::<second>(met),
            _phantom: PhantomData,
        }
    }

    pub fn time(&self) -> Time {
        self.time
    }

    pub fn to_utc(&self) -> DateTime<Utc> {
        (*self).into()
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

impl<I: Instrument> From<MissionElapsedTime<I>> for DateTime<Utc> {
    fn from(val: MissionElapsedTime<I>) -> Self {
        let seconds = val.time.get::<second>();
        let whole_seconds = seconds.trunc() as i64;
        let nanoseconds = ((seconds.fract() * 1_000_000_000.0) as i64).clamp(0, 999_999_999);

        let mut time =
            *I::ref_time() + Duration::seconds(whole_seconds) + Duration::nanoseconds(nanoseconds);
        for leap_second in LEAP_SECONDS.iter() {
            if *I::ref_time() < *leap_second && time > *leap_second {
                time -= Duration::seconds(1);
            }
        }
        time
    }
}

impl<I: Instrument> From<DateTime<Utc>> for MissionElapsedTime<I> {
    fn from(value: DateTime<Utc>) -> Self {
        let duration = value - *I::ref_time();
        let seconds = duration.num_seconds() as f64;
        let nanoseconds = duration.subsec_nanos() as f64 / 1_000_000_000.0;

        let mut time = seconds + nanoseconds;
        for leap_second in LEAP_SECONDS.iter() {
            if *I::ref_time() < *leap_second && value > *leap_second {
                time += 1.0;
            }
        }
        MissionElapsedTime {
            time: Time::new::<second>(time),
            _phantom: PhantomData,
        }
    }
}

impl<I: Instrument> Sub for MissionElapsedTime<I> {
    type Output = Time;

    fn sub(self, rhs: Self) -> Self::Output {
        self.time - rhs.time
    }
}

impl<I: Instrument> Sub<Time> for MissionElapsedTime<I> {
    type Output = Self;

    fn sub(self, rhs: Time) -> Self::Output {
        MissionElapsedTime {
            time: self.time - rhs,
            _phantom: PhantomData,
        }
    }
}

impl<I: Instrument> Add<Time> for MissionElapsedTime<I> {
    type Output = Self;

    fn add(self, rhs: Time) -> Self::Output {
        MissionElapsedTime {
            time: self.time + rhs,
            _phantom: PhantomData,
        }
    }
}
