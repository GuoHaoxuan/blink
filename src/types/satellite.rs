use super::Event;

pub(crate) trait Satellite: Ord + Copy {
    type Event: Event<Satellite = Self>;

    fn ref_time() -> &'static hifitime::Epoch;
}
