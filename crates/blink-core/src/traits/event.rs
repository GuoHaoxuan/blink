use crate::traits::Satellite;
use crate::types::MissionElapsedTime;

pub trait Event {
    type Satellite: Satellite;
    type ChannelType;
    // type DetectorType;

    fn time(&self) -> MissionElapsedTime<Self::Satellite>;
    fn channel(&self) -> Self::ChannelType;
    // fn detector(&self) -> Self::DetectorType;
    fn group(&self) -> u8 {
        0
    }
    fn keep(&self) -> bool {
        true
    }
}
