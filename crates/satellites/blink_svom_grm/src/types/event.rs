use blink_core::types::MissionElapsedTime;
use serde::Serialize;

use crate::types::satellite::Svom;

#[derive(Serialize, Debug, Clone)]
pub struct Event {
    pub time: MissionElapsedTime<Svom>,
    pub channel: i16,
    pub detector_id: u8,
    pub gain_type: u8,
    pub dead_time: f32,
    pub evt_type: u8,
    pub anti_coin: u8,
    pub flag: u8,
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

    fn keep(&self) -> bool {
        true
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl Eq for Event {}
