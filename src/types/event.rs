use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Satellite, Time};

pub trait Event: Serialize + Eq + Ord + Copy {
    type Satellite: Satellite;
    type ChannelType;
    // type DetectorType;

    fn time(&self) -> Time<Self::Satellite>;
    fn channel(&self) -> Self::ChannelType;
    // fn detector(&self) -> Self::DetectorType;
    fn group(&self) -> u8 {
        0
    }
    fn keep(&self) -> bool {
        true
    }
    fn to_general(&self) -> GenericEvent;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent {
    pub time: DateTime<Utc>,
    pub channel: u32,
    pub info: serde_json::Value,
    pub keep: bool,
}
