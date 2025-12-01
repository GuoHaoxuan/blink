use chrono::prelude::*;

pub struct TemporalState<T> {
    pub timestamp: DateTime<Utc>,
    pub state: T,
}
