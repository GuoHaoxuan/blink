use crate::{
    satellites::hxmt::types::Hxmt,
    types::{Attitude, Time},
};
use anyhow::{Context, Result, anyhow};

pub struct AttFile {
    // HDU 3: ATT_Quater
    time: Vec<f64>,
    q1: Vec<f64>,
    q2: Vec<f64>,
    q3: Vec<f64>,
}

impl AttFile {
    pub fn new(filename: &str) -> Result<Self> {
        let mut fptr = fitsio::FitsFile::open(filename)
            .with_context(|| format!("Failed to open file: {}", filename))?;

        // HDU 3: ATT_Quater
        let att = fptr.hdu("ATT_Quater")?;
        let time = att.read_col::<f64>(&mut fptr, "Time").with_context(|| {
            format!(
                "Failed to read column Time from HDU ATT_Quater in file: {}",
                filename
            )
        })?;
        let q1 = att.read_col::<f64>(&mut fptr, "Q1").with_context(|| {
            format!(
                "Failed to read column Q1 from HDU ATT_Quater in file: {}",
                filename
            )
        })?;
        let q2 = att.read_col::<f64>(&mut fptr, "Q2").with_context(|| {
            format!(
                "Failed to read column Q2 from HDU ATT_Quater in file: {}",
                filename
            )
        })?;
        let q3 = att.read_col::<f64>(&mut fptr, "Q3").with_context(|| {
            format!(
                "Failed to read column Q3 from HDU ATT_Quater in file: {}",
                filename
            )
        })?;

        Ok(Self { time, q1, q2, q3 })
    }

    pub fn interpolate(&self, time: Time<Hxmt>) -> Attitude {
        let time_f64 = time.time.into_inner();
        let mut i = 0;
        while i < self.time.len() - 1 && self.time[i + 1] < time_f64 {
            i += 1;
        }
        if i == self.time.len() - 1 {
            // linear extension
            let last_time = self.time[self.time.len() - 1];
            let last_q1 = self.q1[self.q1.len() - 1];
            let last_q2 = self.q2[self.q2.len() - 1];
            let last_q3 = self.q3[self.q3.len() - 1];
            let last_q1_2 = self.q1[self.q1.len() - 2];
            let last_q2_2 = self.q2[self.q2.len() - 2];
            let last_q3_2 = self.q3[self.q3.len() - 2];
            let t_diff = time_f64 - last_time;
            let ratio = t_diff / (last_time - self.time[self.time.len() - 2]);
            let q1_interp = last_q1 + (last_q1 - last_q1_2) * ratio;
            let q2_interp = last_q2 + (last_q2 - last_q2_2) * ratio;
            let q3_interp = last_q3 + (last_q3 - last_q3_2) * ratio;
            return Attitude {
                time: time.to_chrono(),
                q1: q1_interp,
                q2: q2_interp,
                q3: q3_interp,
            };
        }

        let t0 = self.time[i];
        let t1 = self.time[i + 1];
        let q10 = self.q1[i];
        let q11 = self.q1[i + 1];
        let q20 = self.q2[i];
        let q21 = self.q2[i + 1];
        let q30 = self.q3[i];
        let q31 = self.q3[i + 1];

        // Linear interpolation
        let q1_interp = q10 + (q11 - q10) * (time_f64 - t0) / (t1 - t0);
        let q2_interp = q20 + (q21 - q20) * (time_f64 - t0) / (t1 - t0);
        let q3_interp = q30 + (q31 - q30) * (time_f64 - t0) / (t1 - t0);

        Attitude {
            time: time.to_chrono(),
            q1: q1_interp,
            q2: q2_interp,
            q3: q3_interp,
        }
    }
}
