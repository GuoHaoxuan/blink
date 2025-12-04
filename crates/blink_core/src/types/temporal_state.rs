use crate::traits::Temporal;

#[derive(Clone)]
pub struct TemporalState<Time: Temporal, State: Clone> {
    pub timestamp: Time,
    pub state: State,
}
