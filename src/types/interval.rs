use serde::Serialize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize)]
pub(crate) struct Interval<T> {
    pub(crate) start: T,
    pub(crate) stop: T,
}
