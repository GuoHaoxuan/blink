use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub(crate) struct HxmtDetectorType {
    pub id: u8,
    pub acd: [bool; 18],
    pub pulse_width: u8,
}
