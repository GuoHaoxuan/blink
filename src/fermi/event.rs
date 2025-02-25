use hifitime::prelude::*;

use super::detector::Detector;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub(crate) struct Event {
    pub(super) time: Epoch,
    pub(super) pha: i16,
    pub(super) detector: Detector,
}

impl Event {
    pub(crate) fn time(&self) -> Epoch {
        self.time
    }

    pub(crate) fn pha(&self) -> i16 {
        self.pha
    }

    pub(crate) fn detector(&self) -> Detector {
        self.detector
    }

    // TODO: move out of Event
    pub(crate) fn group(&self) -> Result<u8, &'static str> {
        match self.detector {
            Detector::Nai(0..=2) => Ok(0),
            Detector::Nai(3..=5) => Ok(1),
            Detector::Nai(6..=8) => Ok(2),
            Detector::Nai(9..=11) => Ok(3),
            Detector::Bgo(0) => Ok(4),
            Detector::Bgo(1) => Ok(5),
            _ => Err("Invalid detector"),
        }
    }
}
