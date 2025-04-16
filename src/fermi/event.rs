use serde::Serialize;

use crate::types::{GenericEvent, Group, Time};

use super::{detector::FermiDetectorType, Fermi};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Serialize)]
pub(crate) struct FermiEvent {
    pub(super) time: Time<Fermi>,
    pub(super) energy: i16,
    pub(super) detector: FermiDetectorType,
}

impl FermiEvent {
    pub(crate) fn detector(&self) -> FermiDetectorType {
        self.detector
    }
}

impl crate::types::Event for FermiEvent {
    type Satellite = Fermi;
    type EnergyType = i16;
    // type DetectorType = FermiDetectorType;

    fn time(&self) -> Time<Fermi> {
        self.time
    }

    fn energy(&self) -> Self::EnergyType {
        self.energy
    }

    // fn detector(&self) -> Self::DetectorType {
    //     self.detector
    // }

    fn to_general(&self, ec_function: impl Fn(&Self) -> [f64; 2]) -> GenericEvent {
        GenericEvent {
            time: self.time.to_chrono(),
            energy: ec_function(self),
            detector: self.detector.to_string(),
        }
    }
}

impl Group for FermiEvent {
    fn group(&self) -> u8 {
        match self.detector {
            FermiDetectorType::Nai(0..=2) => 0,
            FermiDetectorType::Nai(3..=5) => 1,
            FermiDetectorType::Nai(6..=8) => 2,
            FermiDetectorType::Nai(9..=11) => 3,
            FermiDetectorType::Bgo(0) => 4,
            FermiDetectorType::Bgo(1) => 5,
            _ => panic!("Invalid detector"),
        }
    }
}
