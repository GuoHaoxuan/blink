#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Detector {
    Nai(u8),
    Bgo(u8),
}
