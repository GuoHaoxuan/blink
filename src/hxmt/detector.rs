use std::fmt::Display;

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub(crate) struct HxmtDetectorType {
    pub id: u8,
    pub acd: [bool; 18],
}

impl Display for HxmtDetectorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "HxmtDetectorType {{ id: {}, acds: {:?} }}",
            self.id, self.acd
        )
    }
}
