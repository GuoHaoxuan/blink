// pub(super) struct GtiHdu {
//     start: Vec<f64>,
//     stop: Vec<f64>,
// }

// impl GtiHdu {
//     pub fn from_fptr(fptr: &mut fitsio::FitsFile) -> Result<Self, fitsio::errors::Error> {
//         let gti = fptr.hdu("GTI")?;

//         let start = gti.read_col::<f64>(fptr, "START")?;
//         let stop = gti.read_col::<f64>(fptr, "STOP")?;

//         Ok(Self { start, stop })
//     }
// }
