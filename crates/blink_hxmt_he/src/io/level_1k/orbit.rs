use blink_core::{
    error::Error,
    types::{MissionElapsedTime, Position, TemporalState, Trajectory},
};

use crate::types::Hxmt;

pub struct OrbitFile {
    // HDU 1: Orbit
    time: Vec<f64>,
    lon: Vec<f64>,
    lat: Vec<f64>,
    alt: Vec<f64>,
}

impl OrbitFile {
    pub fn new(filename: &str) -> Result<Self, Error> {
        let mut fptr = fitsio::FitsFile::open(filename)?;

        // HDU 1: Orbit
        let orbit = fptr.hdu("Orbit")?;
        let time = orbit.read_col::<f64>(&mut fptr, "Time")?;
        let lon = orbit.read_col::<f64>(&mut fptr, "Lon")?;
        let lat = orbit.read_col::<f64>(&mut fptr, "Lat")?;
        let alt = orbit.read_col::<f64>(&mut fptr, "Alt")?;

        Ok(Self {
            time,
            lon,
            lat,
            alt,
        })
    }
}

impl From<OrbitFile> for Trajectory<MissionElapsedTime<Hxmt>, Position> {
    fn from(orbit_file: OrbitFile) -> Self {
        let points = orbit_file
            .time
            .iter()
            .zip(orbit_file.lon.iter())
            .zip(orbit_file.lat.iter())
            .zip(orbit_file.alt.iter())
            .map(|(((t, lon), lat), alt)| TemporalState {
                timestamp: MissionElapsedTime::new(*t),
                state: Position {
                    longitude: *lon,
                    latitude: *lat,
                    altitude: uom::si::f64::Length::new::<uom::si::length::meter>(*alt),
                },
            })
            .collect();

        Trajectory { points }
    }
}
