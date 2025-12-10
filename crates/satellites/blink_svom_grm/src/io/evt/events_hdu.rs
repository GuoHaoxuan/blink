pub(super) struct EventsHdu {
    id: u8,
    time: Vec<f64>,
    pi: Vec<i16>,
    gain_type: Vec<u8>,
    dead_time: Vec<f32>,
    evt_type: Vec<u8>,
    anti_coin: Vec<u8>,
    flag: Vec<u8>,
}

impl EventsHdu {
    pub fn from_fptr(fptr: &mut fitsio::FitsFile, id: u8) -> Result<Self, fitsio::errors::Error> {
        let events = fptr.hdu(format!("EVENTS0{}", id).as_str())?;

        let time = events.read_col::<f64>(fptr, "TIME")?;
        let pi = events.read_col::<i16>(fptr, "PI")?;
        let gain_type = events.read_col::<u8>(fptr, "GAIN_TYPE")?;
        let dead_time = events.read_col::<f32>(fptr, "DEAD_TIME")?;
        let evt_type = events.read_col::<u8>(fptr, "EVT_TYPE")?;
        let anti_coin = events.read_col::<u8>(fptr, "ANTI_COIN")?;
        let flag = events.read_col::<u8>(fptr, "FLAG")?;

        Ok(Self {
            id,
            time,
            pi,
            gain_type,
            dead_time,
            evt_type,
            anti_coin,
            flag,
        })
    }
}
