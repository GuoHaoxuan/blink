use serde::Serialize;

use crate::types::{GenericEvent, Group, Time};

use super::{Hxmt, detector::HxmtDetectorType, ec::HxmtCsiEc};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Serialize)]
pub struct HxmtEvent {
    pub time: Time<Hxmt>,
    pub energy: u16,
    pub detector: HxmtDetectorType,
}

impl HxmtEvent {
    pub fn detector(&self) -> HxmtDetectorType {
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

    fn to_general(&self) -> GenericEvent {
        let ec = HxmtCsiEc::from_datetime(&self.time.to_chrono()).unwrap();
        GenericEvent {
            time: self.time.to_chrono(),
            energy_channel: self.energy as u32,
            energy_deposition: ec.channel_to_energy(self.energy).unwrap(),
            energy_incident: 0.0, // [TODO] Placeholder, as we don't have incident energy in this context
            detector: serde_json::to_value(self.detector).unwrap(),
        }
    }
}

impl Group for HxmtEvent {
    fn group(&self) -> u8 {
        0
    }
}
