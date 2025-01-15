use hifitime::prelude::*;

#[derive(Clone)]
pub struct Event {
    pub time: Epoch,
    // pi is unused now
    // pub pi: u32,
    pub detector: u8,
}

#[derive(Clone)]
pub struct CompactedEvent<T: Clone> {
    pub time: f64,
    pub pi: T,
    pub detector: u8,
}
