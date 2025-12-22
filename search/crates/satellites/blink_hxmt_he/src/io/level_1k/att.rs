use crate::types::Hxmt;
use blink_core::error::Error;
use blink_core::types::{Attitude, MissionElapsedTime, TemporalState, Trajectory};

pub struct AttFile {
    // HDU 3: ATT_Quater
    time: Vec<f64>,
    q1: Vec<f64>,
    q2: Vec<f64>,
    q3: Vec<f64>,
}

impl AttFile {
    pub fn new(filename: &str) -> Result<Self, Error> {
        let mut fptr = fitsio::FitsFile::open(filename)?;

        // HDU 3: ATT_Quater
        let att = fptr.hdu("ATT_Quater")?;
        let time = att.read_col::<f64>(&mut fptr, "Time")?;
        let q1 = att.read_col::<f64>(&mut fptr, "Q1")?;
        let q2 = att.read_col::<f64>(&mut fptr, "Q2")?;
        let q3 = att.read_col::<f64>(&mut fptr, "Q3")?;

        Ok(Self { time, q1, q2, q3 })
    }
}

impl From<&AttFile> for Trajectory<MissionElapsedTime<Hxmt>, Attitude> {
    fn from(att_file: &AttFile) -> Self {
        let points = att_file
            .time
            .iter()
            .zip(att_file.q1.iter())
            .zip(att_file.q2.iter())
            .zip(att_file.q3.iter())
            .map(|(((t, q1), q2), q3)| TemporalState {
                timestamp: MissionElapsedTime::new(*t),
                state: Attitude {
                    q1: *q1,
                    q2: *q2,
                    q3: *q3,
                },
            })
            .collect();

        Trajectory { points }
    }
}
