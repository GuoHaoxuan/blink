use std::str::FromStr;
use std::sync::OnceLock;

use hifitime::prelude::*;

use crate::types::Satellite;

use super::Fermi;

impl Satellite for Fermi {
    fn ref_time() -> &'static Epoch {
        static REF_TIME: OnceLock<Epoch> = OnceLock::new();
        REF_TIME.get_or_init(|| Epoch::from_str("2001-01-01T00:00:00.000000000 UTC").unwrap())
    }
}
