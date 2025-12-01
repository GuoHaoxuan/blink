use std::sync::LazyLock;
use uom::si::f64::*;

pub static SPEED_OF_LIGHT: LazyLock<Velocity> =
    LazyLock::new(|| Velocity::new::<uom::si::velocity::meter_per_second>(299_792_458.0));
pub static R_EARTH: LazyLock<Length> =
    LazyLock::new(|| Length::new::<uom::si::length::meter>(6_371_000.0));
pub static LIGHTNING_ALTITUDE: LazyLock<Length> =
    LazyLock::new(|| Length::new::<uom::si::length::meter>(15_000.0));
