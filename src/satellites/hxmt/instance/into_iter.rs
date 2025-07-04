use super::{
    super::{data::data_1k::Iter, types::HxmtEvent},
    define::Instance,
};

impl<'a> IntoIterator for &'a Instance {
    type Item = HxmtEvent;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.event_file.into_iter()
    }
}
