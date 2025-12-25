use serde::Serialize;

#[derive(PartialEq, Eq, Serialize, Debug, Clone)]
pub enum Scintillator {
    /// Sodium Iodide (NaI)
    Nai,
    /// Cesium Iodide (CsI)
    Csi,
}

#[derive(Serialize, Debug, Clone)]
pub struct Detector {
    pub id: u8,
    pub scintillator: Scintillator,
}
