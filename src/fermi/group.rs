use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::types::{Epoch, Interval};

use super::detector::Detector;
use super::event::Event;
use super::file::{self, File};
use super::Fermi;

pub(crate) struct Group {
    files: Vec<File>,
}

impl Group {
    pub(crate) fn new(data: &[(&str, Detector)]) -> Result<Self, fitsio::errors::Error> {
        let files = data
            .iter()
            .map(|(filename, detector)| File::new(filename, *detector))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { files })
    }

    pub(crate) fn gti(&self) -> Vec<Interval<Epoch<Fermi>>> {
        self.files
            .iter()
            .map(|file| file.gti())
            .reduce(|a, b| {
                let mut res = Vec::new();
                let mut a_iter = a.into_iter().peekable();
                let mut b_iter = b.into_iter().peekable();

                while let (Some(a), Some(b)) = (a_iter.peek(), b_iter.peek()) {
                    let start = a.start.max(b.start);
                    let stop = a.stop.min(b.stop);
                    if start < stop {
                        res.push(Interval { start, stop });
                    }
                    if a.stop < b.stop {
                        a_iter.next();
                    } else {
                        b_iter.next();
                    }
                }
                res
            })
            .unwrap()
    }
}

impl<'a> IntoIterator for &'a Group {
    type Item = Event;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let mut file_iters = self
            .files
            .iter()
            .map(|file| file.into_iter())
            .collect::<Vec<_>>();
        let mut buffer = BinaryHeap::new();
        for (index, file_iter) in file_iters.iter_mut().enumerate() {
            if let Some(event) = file_iter.next() {
                buffer.push(Reverse((event, index)));
            }
        }
        Iter { file_iters, buffer }
    }
}

pub(crate) struct Iter<'a> {
    file_iters: Vec<file::Iter<'a>>,
    buffer: BinaryHeap<Reverse<(Event, usize)>>,
}

impl Iterator for Iter<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(Reverse((event, index))) = self.buffer.pop() {
            if let Some(next_event) = self.file_iters[index].next() {
                self.buffer.push(Reverse((next_event, index)));
            }
            Some(event)
        } else {
            None
        }
    }
}
