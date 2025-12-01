use crate::traits::Interpolatable;
use uom::si::f64::*;

pub struct Position {
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: Length,
}

impl Interpolatable for Position {
    fn interpolate(&self, other: &Self, ratio: f64) -> Self {
        Position {
            longitude: self.longitude + (other.longitude - self.longitude) * ratio,
            latitude: self.latitude + (other.latitude - self.latitude) * ratio,
            altitude: self.altitude + (other.altitude - self.altitude) * ratio,
        }
    }
}
