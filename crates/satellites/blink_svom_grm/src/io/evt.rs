/*
Filename: svom_grm_evt_250101_00_v00.fits
No.    Name      Ver    Type      Cards   Dimensions   Format
  0  PrimaryHDU    1 PrimaryHDU      39   ()
  1  EBOUNDS       1 BinTableHDU     66   259R x 3C   [I, E, E]
  2  GTI           1 BinTableHDU     52   1R x 2C   [D, D]
  3  EVENTS01      1 BinTableHDU     68   1094846R x 7C   [D, I, B, E, B, B, B]
  4  EVENTS02      1 BinTableHDU     68   1028001R x 7C   [D, I, B, E, B, B, B]
  5  EVENTS03      1 BinTableHDU     68   1270337R x 7C   [D, I, B, E, B, B, B]
*/

mod ebounds_hdu;
mod events_hdu;
mod gti_hdu;

use ebounds_hdu::EboundsHdu;
use events_hdu::EventsHdu;
use gti_hdu::GtiHdu;

pub struct EvtFile {
    ebounds: EboundsHdu,
    gti: GtiHdu,
    events01: EventsHdu,
    events02: EventsHdu,
    events03: EventsHdu,
}

impl EvtFile {
    pub fn from_fits_file(path: &str) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(path)?;

        let ebounds = EboundsHdu::from_fptr(&mut fptr)?;
        let gti = GtiHdu::from_fptr(&mut fptr)?;
        let events01 = EventsHdu::from_fptr(&mut fptr, 1)?;
        let events02 = EventsHdu::from_fptr(&mut fptr, 2)?;
        let events03 = EventsHdu::from_fptr(&mut fptr, 3)?;

        Ok(Self {
            ebounds,
            gti,
            events01,
            events02,
            events03,
        })
    }
}
