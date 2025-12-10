use blink_core::types::MissionElapsedTime;
use serde::Serialize;

use crate::types::satellite::Svom;

#[derive(Serialize, Debug, Clone)]
pub struct Event {
    time: MissionElapsedTime<Svom>,
    channel: i16,
    pub detector_id: u8,
    pub gain_type: u8,
    pub dead_time: f32,
    pub evt_type: u8,
    pub anti_coin: u8,
    flag: u8,
}

impl blink_core::traits::Event for Event {
    type Satellite = Svom;
    type ChannelType = i16;

    fn time(&self) -> MissionElapsedTime<Self::Satellite> {
        self.time
    }

    fn channel(&self) -> Self::ChannelType {
        self.channel
    }

    fn group(&self) -> u8 {
        0
    }
}
