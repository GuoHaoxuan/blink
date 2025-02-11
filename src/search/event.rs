use hifitime::prelude::*;

#[derive(Clone)]
pub struct Event {
    pub time: Epoch,
    // pi is unused now
    // pub pi: u32,
    pub detector: u8,
}

#[derive(Clone)]
#[repr(packed)]
pub struct PackedEvent<T: Clone> {
    pub time: f64,
    pub pi: T,
    pub detector: u8,
}
