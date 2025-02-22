use fitsio::{FitsFile, hdu::FitsHdu, tables::FitsRow};
use fitsio_derive::FitsRow;
use std::cmp::{Ord, Ordering, Reverse};
use std::collections::BinaryHeap;
use std::marker::PhantomData;

#[derive(Default, FitsRow, Clone, PartialEq, Debug)]
struct Row {
    #[fitsio(colname = "TIME")]
    time: f64,
    #[fitsio(colname = "PHA")]
    pha: i16,
}

impl Eq for Row {}

impl PartialOrd for Row {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Row {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.partial_cmp(&other.time).unwrap()
    }
}

struct EventsIter<T> {
    fptr: FitsFile,
    hdu: FitsHdu,
    next_index: usize,
    phantom: PhantomData<T>,
}

impl<T> From<(FitsFile, FitsHdu)> for EventsIter<T> {
    fn from((fptr, hdu): (FitsFile, FitsHdu)) -> Self {
        Self {
            fptr,
            hdu,
            next_index: 0,
            phantom: PhantomData,
        }
    }
}

impl<T: FitsRow> Iterator for EventsIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.hdu.row(&mut self.fptr, self.next_index);
        match row {
            Ok(row) => {
                self.next_index += 1;
                Some(row)
            }
            Err(_) => None,
        }
    }
}

struct EventsGroupIter<T> {
    iters: Vec<EventsIter<T>>,
    buffer: BinaryHeap<Reverse<(T, usize)>>,
}

impl<T: Ord + FitsRow> From<Vec<EventsIter<T>>> for EventsGroupIter<T> {
    fn from(mut iters: Vec<EventsIter<T>>) -> Self {
        let mut buffer = BinaryHeap::<Reverse<(T, usize)>>::new();
        for (i, iter) in iters.iter_mut().enumerate() {
            if let Some(row) = iter.next() {
                buffer.push(Reverse((row, i)));
            }
        }
        Self { iters, buffer }
    }
}

impl<T: Ord + FitsRow> Iterator for EventsGroupIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let Reverse((row, i)) = self.buffer.pop()?;
        if let Some(next_row) = self.iters[i].next() {
            self.buffer.push(Reverse((next_row, i)));
        }
        Some(row)
    }
}
