use serde::Serialize;

use crate::types::{GenericEvent, Time};

use super::{Hxmt, HxmtDetectorType, HxmtScintillator};

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

    fn keep(&self) -> bool {
        const CHANNEL_THRESHOLD: u16 = 38;
        self.detector.scintillator == HxmtScintillator::NaI
            && !self.detector.am241
            && self.channel >= CHANNEL_THRESHOLD
    }

    fn to_general(&self) -> GenericEvent {
        GenericEvent {
            time: self.time.to_chrono(),
            channel: self.channel as u32,
            info: serde_json::to_value(self.detector).unwrap(),
            keep: self.keep(),
        }
    }
}
