use serde::Serialize;

use crate::types::{GenericEvent, Group, Time};

use super::{Hxmt, detector::HxmtDetectorType};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Serialize)]
pub struct HxmtEvent {
    pub time: Time<Hxmt>,
    pub channel: u16,
    pub detector: HxmtDetectorType,
}

impl HxmtEvent {
    pub fn detector(&self) -> HxmtDetectorType {
        self.detector
    }
}

impl crate::types::Event for HxmtEvent {
    type Satellite = Hxmt;
    type ChannelType = u16;

    fn time(&self) -> Time<Hxmt> {
        self.time
    }

    fn channel(&self) -> Self::ChannelType {
        self.channel
    }

    fn to_general(&self) -> GenericEvent {
        GenericEvent {
            time: self.time.to_chrono(),
            channel: self.channel as u32,
            detector: serde_json::to_value(self.detector).unwrap(),
        }
    }
}

impl Group for HxmtEvent {
    fn group(&self) -> u8 {
        0
    }
}
