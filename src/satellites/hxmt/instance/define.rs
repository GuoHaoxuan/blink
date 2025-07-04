use crate::types::Time;

use super::super::{
    data::{
        data_1b::{EngFile, SciFile},
        data_1k::{AttFile, EventFile, OrbitFile},
    },
    types::Hxmt,
};

pub struct Instance {
    pub event_file: EventFile,
    pub eng_files: [EngFile; 3],
    pub sci_files: [SciFile; 3],
    pub orbit_file: OrbitFile,
    pub att_file: AttFile,
    pub span: [Time<Hxmt>; 2],
}
