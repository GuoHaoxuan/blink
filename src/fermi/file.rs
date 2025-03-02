use std::iter::zip;

use crate::types::{Epoch, Interval};

use super::detector::Detector;
use super::event::EventPha;
use super::Fermi;

pub(super) struct File {
    // HDU 1: EBOUNDS
    ebounds_channel: Vec<i16>,
    pub(crate) ebounds_e_min: Vec<f32>,
    pub(crate) ebounds_e_max: Vec<f32>,

    // HDU 2: EVENTS
    events_time: Vec<f64>,
    events_pha: Vec<i16>,

    // HDU 3: GTI
    gti_start: Vec<f64>,
    gti_stop: Vec<f64>,

    // detector
    detector: Detector,
}

impl File {
    pub(super) fn new(filename: &str, detector: Detector) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(filename)?;

        // HDU 1: EBOUNDS
        let ebounds = fptr.hdu("EBOUNDS")?;
        let ebounds_channel = ebounds.read_col::<i16>(&mut fptr, "CHANNEL")?;
        let ebounds_e_min = ebounds.read_col::<f32>(&mut fptr, "E_MIN")?;
        let ebounds_e_max = ebounds.read_col::<f32>(&mut fptr, "E_MAX")?;

        // HDU 2: EVENTS
        let events = fptr.hdu("EVENTS")?;
        let events_time = events.read_col::<f64>(&mut fptr, "TIME")?;
        let events_pha = events.read_col::<i16>(&mut fptr, "PHA")?;

        // HDU 3: GTI
        let gti = fptr.hdu("GTI")?;
        let gti_start = gti.read_col::<f64>(&mut fptr, "START")?;
        let gti_stop = gti.read_col::<f64>(&mut fptr, "STOP")?;

        Ok(Self {
            ebounds_channel,
            ebounds_e_min,
            ebounds_e_max,
            events_time,
            events_pha,
            gti_start,
            gti_stop,
            detector,
        })
    }

    pub(super) fn gti(&self) -> Vec<Interval<Epoch<Fermi>>> {
        zip(&self.gti_start, &self.gti_stop)
            .map(|(start, stop)| Interval {
                start: Epoch::new(*start),
                stop: Epoch::new(*stop),
            })
            .collect()
    }
}

impl<'a> IntoIterator for &'a File {
    type Item = EventPha;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            file: self,
            index: 0,
        }
    }
}

pub(super) struct Iter<'a> {
    file: &'a File,
    index: usize,
}

impl Iterator for Iter<'_> {
    type Item = EventPha;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.file.events_time.len() {
            let event = EventPha {
                time: Epoch::new(self.file.events_time[self.index]),
                energy: self.file.events_pha[self.index],
                detector: self.file.detector,
            };
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }
}
