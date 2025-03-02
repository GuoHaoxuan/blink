pub(crate) struct Position {
    sclk_utc: Vec<f64>,
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
}
