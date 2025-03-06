use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Detector {
    Nai(u8),
    Bgo(u8),
}

impl fmt::Display for Detector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Detector::Nai(n) => write!(f, "n{}", n),
            Detector::Bgo(n) => write!(f, "b{}", n),
        }
    }
}

impl Serialize for Detector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}
