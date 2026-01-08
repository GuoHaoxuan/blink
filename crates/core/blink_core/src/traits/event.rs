use crate::traits::Instrument;
use crate::types::MissionElapsedTime;
use serde::Serialize;
use std::fmt::Debug;

pub trait Event: Serialize + Debug + Clone {
    type Satellite: Instrument;
    type ChannelType;
    // type DetectorType;

    fn time(&self) -> MissionElapsedTime<Self::Satellite>;
    fn channel(&self) -> Self::ChannelType;
    // fn detector(&self) -> Self::DetectorType;
    fn group(&self) -> u8;
    fn keep(&self) -> bool;
}
