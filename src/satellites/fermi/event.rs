use serde::Serialize;

use crate::types::{GenericEvent, Time};

use super::{Fermi, detector::FermiDetectorType};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Serialize)]
pub struct FermiEvent {
    pub(super) time: Time<Fermi>,
    pub(super) channel: i16,
    pub(super) detector: FermiDetectorType,
}

impl FermiEvent {
    pub fn detector(&self) -> FermiDetectorType {
        self.detector
    }
}

impl crate::types::Event for FermiEvent {
    type Satellite = Fermi;
    type ChannelType = i16;
    // type DetectorType = FermiDetectorType;

    fn time(&self) -> Time<Fermi> {
        self.time
    }

    fn channel(&self) -> Self::ChannelType {
        self.channel
    }

    // fn detector(&self) -> Self::DetectorType {
    //     self.detector
    // }

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

    fn to_general(&self) -> GenericEvent {
        GenericEvent {
            time: self.time.to_chrono(),
            channel: self.channel as u32,
            info: serde_json::Value::String(self.detector.to_string()),
            keep: true,
        }
    }
}
