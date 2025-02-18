use hifitime::prelude::*;

#[derive(Clone)]
pub struct Event {
    pub time: Epoch,
    // pi is unused now
    // pub pi: u32,
    pub detector: u8,
    pub group: u8,
}

// DO NOT use repr(packed) here
// This is UB
#[derive(Clone)]
pub struct PackedEvent<T: Clone> {
    pub time: f64,
    pub pi: T,
    pub detector: u8,
}
