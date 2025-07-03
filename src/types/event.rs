use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Group, Satellite, Time};

pub trait Event: Serialize + Eq + Ord + Copy + Group {
    type Satellite: Satellite;
    type ChannelType;
    // type DetectorType;

    fn time(&self) -> Time<Self::Satellite>;
    fn channel(&self) -> Self::ChannelType;
    // fn detector(&self) -> Self::DetectorType;
    fn to_general(&self) -> GenericEvent;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent {
    pub time: DateTime<Utc>,
    pub channel: u32,
    pub detector: serde_json::Value,
}
