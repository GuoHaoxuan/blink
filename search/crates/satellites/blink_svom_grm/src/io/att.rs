/*
Filename: svom_att_250101_00_v00.fits
No.    Name      Ver    Type      Cards   Dimensions   Format
  0  PrimaryHDU    1 PrimaryHDU      39   ()
  1  Quaternion    1 BinTableHDU     72   2069R x 12C   [1D, 1E, 1E, 1E, 1E, 1E, 1E, 1E, 1B, 1J, 1B, 1B]
*/

use blink_core::types::{Attitude, MissionElapsedTime, TemporalState, Trajectory};

use crate::types::Svom;

pub struct AttFile {
    quaternion: QuaternionHdu,
}

impl AttFile {
    pub fn from_fits_file(path: &str) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(path)?;

        let quaternion = QuaternionHdu::from_fptr(&mut fptr)?;

        Ok(Self { quaternion })
    }
}

struct QuaternionHdu {
    time: Vec<f64>,
    q0: Vec<f32>,
    q1: Vec<f32>,
    q2: Vec<f32>,
    q3: Vec<f32>,
    wx: Vec<f32>,
    wy: Vec<f32>,
    wz: Vec<f32>,
    slew_stat: Vec<u8>,
    target_id: Vec<i32>,
    quality: Vec<u8>,
    att_ref: Vec<u8>,
}

impl QuaternionHdu {
    fn from_fptr(fptr: &mut fitsio::FitsFile) -> Result<Self, fitsio::errors::Error> {
        let quaternion = fptr.hdu("Quaternion")?;

        let time = quaternion.read_col::<f64>(fptr, "TIME")?;
        let q0 = quaternion.read_col::<f32>(fptr, "Q0")?;
        let q1 = quaternion.read_col::<f32>(fptr, "Q1")?;
        let q2 = quaternion.read_col::<f32>(fptr, "Q2")?;
        let q3 = quaternion.read_col::<f32>(fptr, "Q3")?;
        let wx = quaternion.read_col::<f32>(fptr, "wx")?;
        let wy = quaternion.read_col::<f32>(fptr, "wy")?;
        let wz = quaternion.read_col::<f32>(fptr, "wz")?;
        let slew_stat = quaternion.read_col::<u8>(fptr, "slew_stat")?;
        let target_id = quaternion.read_col::<i32>(fptr, "TargetID")?;
        let quality = quaternion.read_col::<u8>(fptr, "Quality")?;
        let att_ref = quaternion.read_col::<u8>(fptr, "AttRef")?;

        Ok(Self {
            time,
            q0,
            q1,
            q2,
            q3,
            wx,
            wy,
            wz,
            slew_stat,
            target_id,
            quality,
            att_ref,
        })
    }
}

impl From<&AttFile> for Trajectory<MissionElapsedTime<Svom>, Attitude> {
    fn from(att_file: &AttFile) -> Self {
        let points = att_file
            .quaternion
            .time
            .iter()
            .zip(att_file.quaternion.q0.iter())
            .zip(att_file.quaternion.q1.iter())
            .zip(att_file.quaternion.q2.iter())
            .map(|(((t, q0), q1), q2)| TemporalState {
                timestamp: MissionElapsedTime::new(*t),
                state: Attitude {
                    q1: *q0 as f64,
                    q2: *q1 as f64,
                    q3: *q2 as f64,
                },
            })
            .collect();

        Trajectory { points }
    }
}
