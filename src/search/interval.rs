use hifitime::prelude::*;

#[derive(Clone, Debug)]
pub struct Interval {
    pub start: Epoch,
    pub stop: Epoch,
}
