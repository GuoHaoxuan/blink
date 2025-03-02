use ordered_float::NotNan;
use serde::Serialize;

use crate::types::{Epoch, Group, Interval};

use super::{detector::Detector, Fermi};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Serialize)]
pub(crate) struct Event<T: Copy> {
    pub(super) time: Epoch<Fermi>,
    pub(super) energy: T,
    pub(super) detector: Detector,
}

pub(crate) type EventPha = Event<i16>;
pub(crate) type EventInterval = Event<Interval<NotNan<f64>>>;

impl<T: Copy> Event<T> {
    pub(crate) fn energy(&self) -> T {
        self.energy
    }

    pub(crate) fn detector(&self) -> Detector {
        self.detector
    }
}

impl<T: Copy + Serialize> crate::types::Event for Event<T> {
    type Satellite = Fermi;

    fn time(&self) -> Epoch<Fermi> {
        self.time
    }
}

impl<T: Copy + Serialize> Group for Event<T> {
    fn group(&self) -> u8 {
        match self.detector {
            Detector::Nai(0..=2) => 0,
            Detector::Nai(3..=5) => 1,
            Detector::Nai(6..=8) => 2,
            Detector::Nai(9..=11) => 3,
            Detector::Bgo(0) => 4,
            Detector::Bgo(1) => 5,
            _ => panic!("Invalid detector"),
        }
    }
}
