use crate::hxmt::detector::{HxmtDetectorType, HxmtScintillator};
use crate::hxmt::event::HxmtEvent;
use crate::types::Time;
use anyhow::{Context, Result};

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
    pub(super) fn new(filename: &str) -> Result<Self> {
        let mut fptr = fitsio::FitsFile::open(filename)
            .with_context(|| format!("Failed to open file: {}", filename))?;

        // HDU 1: Events
        let events = fptr
            .hdu("Events")
            .with_context(|| format!("Failed to find HDU Events in file: {}", filename))?;
        let time = events.read_col::<f64>(&mut fptr, "Time").with_context(|| {
            format!(
                "Failed to read column Time from HDU Events in file: {}",
                filename
            )
        })?;
        let det_id = events
            .read_col::<u8>(&mut fptr, "Det_ID")
            .with_context(|| {
                format!(
                    "Failed to read column Det_ID from HDU Events in file: {}",
                    filename
                )
            })?;
        let channel = events
            .read_col::<u8>(&mut fptr, "Channel")
            .with_context(|| {
                format!(
                    "Failed to read column Channel from HDU Events in file: {}",
                    filename
                )
            })?;
        let pulse_width = events
            .read_col::<u8>(&mut fptr, "Pulse_Width")
            .with_context(|| {
                format!(
                    "Failed to read column Pulse_Width from HDU Events in file: {}",
                    filename
                )
            })?;

        let acd_raw = events.read_col::<u32>(&mut fptr, "ACD").with_context(|| {
            format!(
                "Failed to read column ACD from HDU Events in file: {}",
                filename
            )
        })?;
        let mut acd = Vec::with_capacity(acd_raw.len());
        for &value in &acd_raw {
            let mut array = [false; 18];
            array.iter_mut().enumerate().for_each(|(i, bit)| {
                *bit = ((value >> i) & 1) == 1;
            });
            acd.push(array);
        }

        let event_type = events
            .read_col::<u8>(&mut fptr, "Event_Type")
            .with_context(|| {
                format!(
                    "Failed to read column Event_Type from HDU Events in file: {}",
                    filename
                )
            })?;
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
            let mut energy = self.event_file.channel[self.index] as u16;
            if energy < 20 {
                energy += 256;
            }
            let event = HxmtEvent {
                time: Time::seconds(self.event_file.time[self.index]),
                energy,
                detector: HxmtDetectorType {
                    id: self.event_file.det_id[self.index],
                    veto: self.event_file.acd[self.index]
                        .iter()
                        .map(|&x| x as u8)
                        .sum::<u8>(),
                    scintillator: if self.event_file.pulse_width[self.index] < 75 {
                        HxmtScintillator::NaI
                    } else {
                        HxmtScintillator::CsI
                    },
                    am241: self.event_file.event_type[self.index] == 1,
                },
            };
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }
}
