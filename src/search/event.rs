use hifitime::prelude::*;

#[derive(Clone)]
pub struct Event {
    pub time: Epoch,
    pub pi: u32,
    pub detector: usize,
}
