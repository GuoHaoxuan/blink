use crate::types::Location;
use anyhow::{Context, Result, anyhow};

pub struct OrbitFile {
    // HDU 1: Orbit
    time: Vec<f64>,
    lon: Vec<f64>,
    lat: Vec<f64>,
    alt: Vec<f64>,
}

impl OrbitFile {
    pub(super) fn new(filename: &str) -> Result<Self> {
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

    pub fn interpolate(&self, time: f64) -> Result<(f64, f64, f64)> {
        let mut i = 0;
        while i < self.time.len() - 1 && self.time[i + 1] < time {
            i += 1;
        }
        if i == self.time.len() - 1 {
            return Err(anyhow!("Time {} is out of bounds for the orbit data", time));
        }

        let t0 = self.time[i];
        let t1 = self.time[i + 1];
        let lon0 = self.lon[i];
        let lon1 = self.lon[i + 1];
        let lat0 = self.lat[i];
        let lat1 = self.lat[i + 1];
        let alt0 = self.alt[i];
        let alt1 = self.alt[i + 1];

        let lon = lon0 + (lon1 - lon0) * (time - t0) / (t1 - t0);
        let lat = lat0 + (lat1 - lat0) * (time - t0) / (t1 - t0);
        let alt = alt0 + (alt1 - alt0) * (time - t0) / (t1 - t0);

        Ok((lon, lat, alt))
    }

    pub fn window(&self, time: f64, window: f64) -> Vec<Location> {
        let start_time = time - window / 2.0;
        let end_time = time + window / 2.0;

        self.time
            .iter()
            .enumerate()
            .filter(|(_, t)| **t >= start_time && **t <= end_time)
            .map(|(i, _)| {
                let longitude = self.lon[i];
                let latitude = self.lat[i];
                let altitude = self.alt[i];
                Location {
                    longitude,
                    latitude,
                    altitude,
                }
            })
            .collect()
    }
}
