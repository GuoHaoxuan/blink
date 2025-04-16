use crate::hxmt::detector::HxmtDetectorType;
use crate::hxmt::event::HxmtEvent;
use crate::types::Time;

pub(crate) struct EventFile {
    // HDU 1: Events
    time: Vec<f64>,
    det_id: Vec<u8>,
    channel: Vec<u8>,
    pulse_width: Vec<u8>,
    acd: Vec<[bool; 18]>,
    event_type: Vec<u8>,
    // flag: Vec<u8>,
}

impl EventFile {
    pub(super) fn new(filename: &str) -> Result<Self, fitsio::errors::Error> {
        let mut fptr = fitsio::FitsFile::open(filename)?;

        // HDU 1: Events
        let events = fptr.hdu("Events")?;
        let time = events.read_col::<f64>(&mut fptr, "Time")?;
        let det_id = events.read_col::<u8>(&mut fptr, "Det_ID")?;
        let channel = events.read_col::<u8>(&mut fptr, "Channel")?;
        let pulse_width = events.read_col::<u8>(&mut fptr, "Pulse_Width")?;

        let acd_raw = events.read_col::<u32>(&mut fptr, "ACD")?;
        let mut acd = Vec::with_capacity(acd_raw.len());
        for &value in &acd_raw {
            let mut array = [false; 18];
            array.iter_mut().enumerate().for_each(|(i, bit)| {
                *bit = ((value >> i) & 1) == 1;
            });
            acd.push(array);
        }

        let event_type = events.read_col::<u8>(&mut fptr, "Event_Type")?;
        // let flag = events.read_col::<u8>(&mut fptr, "FLAG")?;

        Ok(Self {
            time,
            det_id,
            channel,
            pulse_width,
            acd,
            event_type,
            // flag,
        })
    }
}

impl<'a> IntoIterator for &'a EventFile {
    type Item = HxmtEvent;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            event_file: self,
            index: 0,
        }
    }
}

pub(crate) struct Iter<'a> {
    event_file: &'a EventFile,
    index: usize,
}

impl Iterator for Iter<'_> {
    type Item = HxmtEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.event_file.time.len() {
            let event = HxmtEvent {
                time: Time::seconds(self.event_file.time[self.index]),
                energy: self.event_file.channel[self.index] as u16,
                detector: HxmtDetectorType {
                    id: self.event_file.det_id[self.index],
                    acd: self.event_file.acd[self.index],
                    pulse_width: self.event_file.pulse_width[self.index],
                },
                event_type: self.event_file.event_type[self.index],
            };
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }
}
