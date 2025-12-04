#[derive(PartialEq, Eq)]
pub enum Scintillator {
    /// Sodium Iodide (NaI)
    Nai,
    /// Cesium Iodide (CsI)
    Csi,
}

pub struct Detector {
    pub id: u8,
    pub scintillator: Scintillator,
}
