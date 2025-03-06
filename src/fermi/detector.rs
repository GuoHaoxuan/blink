use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Detector {
    Nai(u8),
    Bgo(u8),
}

impl Detector {
    pub(crate) fn to_string(&self) -> String {
        match self {
            Detector::Nai(n) => format!("n{}", n),
            Detector::Bgo(n) => format!("b{}", n),
        }
    }
}

impl Serialize for Detector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}
