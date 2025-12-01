pub trait Interpolatable {
    fn interpolate(&self, other: &Self, factor: f64) -> Self;
}
