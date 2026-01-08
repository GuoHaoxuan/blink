use serde::{Deserialize, Serialize};

use crate::traits::Interpolatable;

#[derive(Clone, Serialize, Deserialize)]
pub struct Attitude {
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
}

impl Interpolatable for Attitude {
    fn interpolate(&self, other: &Self, ratio: f64) -> Self {
        Attitude {
            q1: self.q1 + (other.q1 - self.q1) * ratio,
            q2: self.q2 + (other.q2 - self.q2) * ratio,
            q3: self.q3 + (other.q3 - self.q3) * ratio,
        }
    }
}
