use super::super::super::types::Hxmt;
use crate::types::{Location, LocationList, Time};
use anyhow::{Context, Result};

pub struct OrbitFile {
    // HDU 1: Orbit
    time: Vec<f64>,
    lon: Vec<f64>,
    lat: Vec<f64>,
    alt: Vec<f64>,
}

impl OrbitFile {
    pub fn new(filename: &str) -> Result<Self> {
        let mut fptr = fitsio::FitsFile::open(filename)
            .with_context(|| format!("Failed to open file: {}", filename))?;

        // HDU 1: Orbit
        let orbit = fptr.hdu("Orbit")?;
        let time = orbit.read_col::<f64>(&mut fptr, "Time").with_context(|| {
            format!(
                "Failed to read column Time from HDU Orbit in file: {}",
                filename
            )
        })?;
        let lon = orbit.read_col::<f64>(&mut fptr, "Lon").with_context(|| {
            format!(
                "Failed to read column Lon from HDU Orbit in file: {}",
                filename
            )
        })?;
        let lat = orbit.read_col::<f64>(&mut fptr, "Lat").with_context(|| {
            format!(
                "Failed to read column Lat from HDU Orbit in file: {}",
                filename
            )
        })?;
        let alt = orbit.read_col::<f64>(&mut fptr, "Alt").with_context(|| {
            format!(
                "Failed to read column Alt from HDU Orbit in file: {}",
                filename
            )
        })?;

        Ok(Self {
            time,
            lon,
            lat,
            alt,
        })
    }

    pub fn window(&self, time: Time<Hxmt>, window: f64) -> LocationList {
        let time_f64 = time.time.into_inner();
        let start_time = time_f64 - window / 2.0;
        let end_time = time_f64 + window / 2.0;

        LocationList {
            data: self
                .time
                .iter()
                .enumerate()
                .filter(|(_, t)| **t >= start_time && **t <= end_time)
                .map(|(i, _)| {
                    let longitude = self.lon[i];
                    let latitude = self.lat[i];
                    let altitude = self.alt[i];
                    Location {
                        time: time.to_chrono(),
                        longitude,
                        latitude,
                        altitude,
                    }
                })
                .collect(),
        }
    }
}
