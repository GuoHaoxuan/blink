use serde::Serialize;

use crate::types::{GenericEvent, Group, Time};

use super::{detector::HxmtDetectorType, Hxmt};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Serialize)]
pub(crate) struct HxmtEvent {
    pub(super) time: Time<Hxmt>,
    pub(super) energy: u16,
    pub(super) detector: HxmtDetectorType,
}

impl HxmtEvent {
    pub(crate) fn detector(&self) -> HxmtDetectorType {
        self.detector
    }
}

impl crate::types::Event for HxmtEvent {
    type Satellite = Hxmt;
    type EnergyType = u16;

    fn time(&self) -> Time<Hxmt> {
        self.time
    }

    fn energy(&self) -> Self::EnergyType {
        self.energy
    }

    fn to_general(&self, ebounds: &crate::types::Ebounds) -> GenericEvent {
        println!("{}", serde_json::to_string(&self.detector).unwrap());
        GenericEvent {
            time: self.time.to_chrono(),
            energy: [
                ebounds[self.energy as usize][0],
                ebounds[self.energy as usize][1],
            ],
            detector: serde_json::to_string(&self.detector).unwrap(),
        }
    }
}

impl Group for HxmtEvent {
    fn group(&self) -> u8 {
        0
    }
}
