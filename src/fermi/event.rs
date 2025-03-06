use serde::Serialize;

use crate::types::{Epoch, GeneralEvent, Group};

use super::{detector::Detector, Fermi};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Serialize)]
pub(crate) struct Event {
    pub(super) time: Epoch<Fermi>,
    pub(super) energy: i16,
    pub(super) detector: Detector,
}

impl Event {
    pub(crate) fn detector(&self) -> Detector {
        self.detector
    }
}

impl crate::types::Event for Event {
    type Satellite = Fermi;

    fn time(&self) -> Epoch<Fermi> {
        self.time
    }

    fn to_general(&self, ebounds: &crate::types::Ebounds) -> GeneralEvent {
        GeneralEvent {
            time: self.time.to_hifitime(),
            energy: [
                ebounds[self.energy as usize][0],
                ebounds[self.energy as usize][1],
            ],
            detector: self.detector.to_string(),
        }
    }
}

impl Group for Event {
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
