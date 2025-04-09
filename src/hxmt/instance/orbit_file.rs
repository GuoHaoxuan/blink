pub(crate) struct OrbitFile {
    // HDU 1: Orbit
    time: Vec<f64>,
    lon: Vec<i32>,
    lat: Vec<i32>,
    alt: Vec<i32>,
}

impl OrbitFile {
    pub(super) fn new(filename: &str) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(filename)?;

        // HDU 1: Orbit
        let orbit = fptr.hdu("Orbit")?;
        let time = orbit.read_col::<f64>(&mut fptr, "Time")?;
        let lon = orbit.read_col::<i32>(&mut fptr, "Lon")?;
        let lat = orbit.read_col::<i32>(&mut fptr, "Lat")?;
        let alt = orbit.read_col::<i32>(&mut fptr, "Alt")?;

        Ok(Self {
            time,
            lon,
            lat,
            alt,
        })
    }

    pub fn interpolate(&self, time: f64) -> anyhow::Result<(f64, f64, f64)> {
        let mut i = 0;
        while i < self.time.len() - 1 && self.time[i + 1] < time {
            i += 1;
        }
        if i == self.time.len() - 1 {
            return Err(anyhow::anyhow!("Time out of bounds"));
        }

        let t0 = self.time[i];
        let t1 = self.time[i + 1];
        let lon0 = self.lon[i] as f64;
        let lon1 = self.lon[i + 1] as f64;
        let lat0 = self.lat[i] as f64;
        let lat1 = self.lat[i + 1] as f64;
        let alt0 = self.alt[i] as f64;
        let alt1 = self.alt[i + 1] as f64;

        let lon = lon0 + (lon1 - lon0) * (time - t0) / (t1 - t0);
        let lat = lat0 + (lat1 - lat0) * (time - t0) / (t1 - t0);
        let alt = alt0 + (alt1 - alt0) * (time - t0) / (t1 - t0);

        Ok((lon, lat, alt))
    }
}
