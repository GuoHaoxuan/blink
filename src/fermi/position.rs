use serde::Serialize;

use crate::types::Epoch;

use super::Fermi;

pub(crate) struct Position {
    pub(crate) sclk_utc: Vec<f64>,
    qsj_1: Vec<f64>,
    qsj_2: Vec<f64>,
    qsj_3: Vec<f64>,
    qsj_4: Vec<f64>,
    wsj_1: Vec<f64>,
    wsj_2: Vec<f64>,
    wsj_3: Vec<f64>,
    pos_x: Vec<f32>,
    pos_y: Vec<f32>,
    pos_z: Vec<f32>,
    vel_x: Vec<f32>,
    vel_y: Vec<f32>,
    vel_z: Vec<f32>,
    sc_lat: Vec<f32>,
    sc_lon: Vec<f32>,
    sada_py: Vec<f32>,
    sada_ny: Vec<f32>,
    flags: Vec<i16>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PositionRow {
    pub(crate) sclk_utc: f64,
    pub(crate) qsj: [f64; 4],
    pub(crate) wsj: [f64; 3],
    pub(crate) pos: [f32; 3],
    pub(crate) vel: [f32; 3],
    pub(crate) sc_lat: f32,
    pub(crate) sc_lon: f32,
    pub(crate) sada: [f32; 2],
    pub(crate) flags: i16,
}

impl Position {
    pub(crate) fn new(filename: &str) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(filename)?;
        let hdu = fptr.hdu("GLAST POS HIST")?;
        Ok(Self {
            sclk_utc: hdu.read_col::<f64>(&mut fptr, "SCLK_UTC")?,
            qsj_1: hdu.read_col::<f64>(&mut fptr, "QSJ_1")?,
            qsj_2: hdu.read_col::<f64>(&mut fptr, "QSJ_2")?,
            qsj_3: hdu.read_col::<f64>(&mut fptr, "QSJ_3")?,
            qsj_4: hdu.read_col::<f64>(&mut fptr, "QSJ_4")?,
            wsj_1: hdu.read_col::<f64>(&mut fptr, "WSJ_1")?,
            wsj_2: hdu.read_col::<f64>(&mut fptr, "WSJ_2")?,
            wsj_3: hdu.read_col::<f64>(&mut fptr, "WSJ_3")?,
            pos_x: hdu.read_col::<f32>(&mut fptr, "POS_X")?,
            pos_y: hdu.read_col::<f32>(&mut fptr, "POS_Y")?,
            pos_z: hdu.read_col::<f32>(&mut fptr, "POS_Z")?,
            vel_x: hdu.read_col::<f32>(&mut fptr, "VEL_X")?,
            vel_y: hdu.read_col::<f32>(&mut fptr, "VEL_Y")?,
            vel_z: hdu.read_col::<f32>(&mut fptr, "VEL_Z")?,
            sc_lat: hdu.read_col::<f32>(&mut fptr, "SC_LAT")?,
            sc_lon: hdu.read_col::<f32>(&mut fptr, "SC_LON")?,
            sada_py: hdu.read_col::<f32>(&mut fptr, "SADA_PY")?,
            sada_ny: hdu.read_col::<f32>(&mut fptr, "SADA_NY")?,
            flags: hdu.read_col::<i16>(&mut fptr, "FLAGS")?,
        })
    }

    pub(crate) fn get_row(&self, epoch: Epoch<Fermi>) -> Option<PositionRow> {
        let sclk_utc = epoch.time.into_inner();
        let pos = self
            .sclk_utc
            .binary_search_by(|probe| probe.partial_cmp(&sclk_utc).unwrap());

        let idx = match pos {
            Ok(idx) => idx,
            Err(idx) => {
                if idx == 0 || idx == self.sclk_utc.len() {
                    return None;
                }
                idx - 1
            }
        };

        let t0 = self.sclk_utc[idx];
        let t1 = self.sclk_utc[idx + 1];
        let alpha = (sclk_utc - t0) / (t1 - t0);
        let alpha_f32 = alpha as f32;

        Some(PositionRow {
            sclk_utc,
            qsj: [
                self.qsj_1[idx] * (1.0 - alpha) + self.qsj_1[idx + 1] * alpha,
                self.qsj_2[idx] * (1.0 - alpha) + self.qsj_2[idx + 1] * alpha,
                self.qsj_3[idx] * (1.0 - alpha) + self.qsj_3[idx + 1] * alpha,
                self.qsj_4[idx] * (1.0 - alpha) + self.qsj_4[idx + 1] * alpha,
            ],
            wsj: [
                self.wsj_1[idx] * (1.0 - alpha) + self.wsj_1[idx + 1] * alpha,
                self.wsj_2[idx] * (1.0 - alpha) + self.wsj_2[idx + 1] * alpha,
                self.wsj_3[idx] * (1.0 - alpha) + self.wsj_3[idx + 1] * alpha,
            ],
            pos: [
                self.pos_x[idx] * (1.0 - alpha_f32) + self.pos_x[idx + 1] * alpha_f32,
                self.pos_y[idx] * (1.0 - alpha_f32) + self.pos_y[idx + 1] * alpha_f32,
                self.pos_z[idx] * (1.0 - alpha_f32) + self.pos_z[idx + 1] * alpha_f32,
            ],
            vel: [
                self.vel_x[idx] * (1.0 - alpha_f32) + self.vel_x[idx + 1] * alpha_f32,
                self.vel_y[idx] * (1.0 - alpha_f32) + self.vel_y[idx + 1] * alpha_f32,
                self.vel_z[idx] * (1.0 - alpha_f32) + self.vel_z[idx + 1] * alpha_f32,
            ],
            sc_lat: self.sc_lat[idx] * (1.0 - alpha_f32) + self.sc_lat[idx + 1] * alpha_f32,
            sc_lon: self.sc_lon[idx] * (1.0 - alpha_f32) + self.sc_lon[idx + 1] * alpha_f32,
            sada: [
                self.sada_py[idx] * (1.0 - alpha_f32) + self.sada_py[idx + 1] * alpha_f32,
                self.sada_ny[idx] * (1.0 - alpha_f32) + self.sada_ny[idx + 1] * alpha_f32,
            ],
            flags: self.flags[idx],
        })
    }
}
