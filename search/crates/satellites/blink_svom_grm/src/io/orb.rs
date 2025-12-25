/*
Filename: svom_orb_250101_00_v00.fits
No.    Name      Ver    Type      Cards   Dimensions   Format
  0  PrimaryHDU    1 PrimaryHDU      39   ()
  1  ORB           1 BinTableHDU     92   2067R x 16C   [1D, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1E]
*/

use blink_core::types::{MissionElapsedTime, Position, TemporalState, Trajectory};

use crate::types::Svom;

pub struct OrbFile {
    orb: OrbHdu,
}

impl OrbFile {
    pub fn from_fits_file(path: &str) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(path)?;

        let orb = OrbHdu::from_fptr(&mut fptr)?;

        Ok(Self { orb })
    }
}

struct OrbHdu {
    time: Vec<f64>,
    // x_j2000: Vec<f32>,
    // y_j2000: Vec<f32>,
    // z_j2000: Vec<f32>,
    // vx_j2000: Vec<f32>,
    // vy_j2000: Vec<f32>,
    // vz_j2000: Vec<f32>,
    // x_wgs84: Vec<f32>,
    // y_wgs84: Vec<f32>,
    // z_wgs84: Vec<f32>,
    // vx_wgs84: Vec<f32>,
    // vy_wgs84: Vec<f32>,
    // vz_wgs84: Vec<f32>,
    lon: Vec<f32>,
    lat: Vec<f32>,
    alt: Vec<f32>,
}

impl OrbHdu {
    fn from_fptr(fptr: &mut fitsio::FitsFile) -> Result<Self, fitsio::errors::Error> {
        let orb = fptr.hdu("ORB")?;

        let time = orb.read_col::<f64>(fptr, "TIME")?;
        // let x_j2000 = orb.read_col::<f32>(fptr, "X_J2000")?;
        // let y_j2000 = orb.read_col::<f32>(fptr, "Y_J2000")?;
        // let z_j2000 = orb.read_col::<f32>(fptr, "Z_J2000")?;
        // let vx_j2000 = orb.read_col::<f32>(fptr, "VX_J2000")?;
        // let vy_j2000 = orb.read_col::<f32>(fptr, "VY_J2000")?;
        // let vz_j2000 = orb.read_col::<f32>(fptr, "VZ_J2000")?;
        // let x_wgs84 = orb.read_col::<f32>(fptr, "X_WGS84")?;
        // let y_wgs84 = orb.read_col::<f32>(fptr, "Y_WGS84")?;
        // let z_wgs84 = orb.read_col::<f32>(fptr, "Z_WGS84")?;
        // let vx_wgs84 = orb.read_col::<f32>(fptr, "VX_WGS84")?;
        // let vy_wgs84 = orb.read_col::<f32>(fptr, "VY_WGS84")?;
        // let vz_wgs84 = orb.read_col::<f32>(fptr, "VZ_WGS84")?;
        let lon = orb.read_col::<f32>(fptr, "LON")?;
        let lat = orb.read_col::<f32>(fptr, "LAT")?;
        let alt = orb.read_col::<f32>(fptr, "ALT")?;

        Ok(Self {
            time,
            // x_j2000,
            // y_j2000,
            // z_j2000,
            // vx_j2000,
            // vy_j2000,
            // vz_j2000,
            // x_wgs84,
            // y_wgs84,
            // z_wgs84,
            // vx_wgs84,
            // vy_wgs84,
            // vz_wgs84,
            lon,
            lat,
            alt,
        })
    }
}

impl From<&OrbFile> for Trajectory<MissionElapsedTime<Svom>, Position> {
    fn from(orb_file: &OrbFile) -> Self {
        let points = orb_file
            .orb
            .time
            .iter()
            .zip(orb_file.orb.lon.iter())
            .zip(orb_file.orb.lat.iter())
            .zip(orb_file.orb.alt.iter())
            .map(|(((t, lon), lat), alt)| TemporalState {
                timestamp: MissionElapsedTime::new(*t),
                state: Position {
                    longitude: *lon as f64,
                    latitude: *lat as f64,
                    altitude: uom::si::f64::Length::new::<uom::si::length::meter>(*alt as f64),
                },
            })
            .collect();

        Trajectory { points }
    }
}
