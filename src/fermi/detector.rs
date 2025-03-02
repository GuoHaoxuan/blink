use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Detector {
    Nai(u8),
    Bgo(u8),
}

impl Serialize for Detector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Detector::Nai(n) => serializer.serialize_str(&("NaI ".to_string() + &n.to_string())),
            Detector::Bgo(n) => serializer.serialize_str(&("BGO ".to_string() + &n.to_string())),
        }
    }
}
