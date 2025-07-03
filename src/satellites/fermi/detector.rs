use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum FermiDetectorType {
    Nai(u8),
    Bgo(u8),
}

impl fmt::Display for FermiDetectorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FermiDetectorType::Nai(n) => write!(f, "n{}", n),
            FermiDetectorType::Bgo(n) => write!(f, "b{}", n),
        }
    }
}

impl Serialize for FermiDetectorType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}
