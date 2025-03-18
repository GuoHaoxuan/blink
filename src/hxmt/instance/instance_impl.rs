use crate::{fermi::Fermi, types::Time};

use super::event_file::EventFile;

pub(crate) struct Instance {
    event_file: EventFile,
    span: [Time<Fermi>; 2],
}
