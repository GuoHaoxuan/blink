use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum HxmtScintillator {
    NaI,
    CsI,
}

impl fmt::Display for HxmtScintillator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HxmtScintillator::NaI => write!(f, "NaI"),
            HxmtScintillator::CsI => write!(f, "CsI"),
        }
    }
}

impl Serialize for HxmtScintillator {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub struct HxmtDetectorType {
    pub id: u8,
    pub acd: u8,
    pub scintillator: HxmtScintillator,
    pub am241: bool,
}
