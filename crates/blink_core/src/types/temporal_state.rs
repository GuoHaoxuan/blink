use serde::Serialize;

use crate::traits::Temporal;

#[derive(Clone, Serialize, Debug)]
pub struct TemporalState<Time: Temporal, State: Clone> {
    pub timestamp: Time,
    pub state: State,
}
