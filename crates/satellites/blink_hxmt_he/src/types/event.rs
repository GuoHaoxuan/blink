use crate::types::Detector;
use crate::types::Hxmt;
use crate::types::Scintillator;
use blink_core::traits::Event as EventTrait;
use blink_core::types::MissionElapsedTime;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Event {
    time: MissionElapsedTime<Hxmt>,
    channel: u8,
    pub detector: Detector,
    pub is_am241: bool,
    pub acds: [bool; 18],
}

impl EventTrait for Event {
    type Satellite = Hxmt;
    type ChannelType = u16;

    fn time(&self) -> MissionElapsedTime<Self::Satellite> {
        self.time
    }

    fn channel(&self) -> Self::ChannelType {
        let channel_u16 = self.channel as u16;
        if channel_u16 < 20 {
            channel_u16 + 256
        } else {
            channel_u16
        }
    }

    fn group(&self) -> u8 {
        0
    }

    fn keep(&self) -> bool {
        const CHANNEL_THRESHOLD: u16 = 38;
        self.detector.scintillator == Scintillator::Csi
            && !self.is_am241
            && self.channel() >= CHANNEL_THRESHOLD
    }
}

impl Event {
    pub fn new(
        time: MissionElapsedTime<Hxmt>,
        channel: u8,
        detector: Detector,
        is_am241: bool,
        acds: [bool; 18],
    ) -> Self {
        Self {
            time,
            channel,
            detector,
            is_am241,
            acds,
        }
    }
}
