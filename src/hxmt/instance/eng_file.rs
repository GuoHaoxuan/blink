use anyhow::Result;

pub(crate) struct EngFile {
    // HDU 1: HE_Eng
    // Only extract useful columns
    pub(crate) time: Vec<i32>,
    pub(crate) bus_time_bdc: Vec<[u8; 6]>,
}

impl EngFile {
    pub(crate) fn new(filename: &str) -> Result<Self> {
        println!("[DEBUG] Open eng file: {}", filename);
        let mut fptr = fitsio::FitsFile::open(filename)?;

        // HDU 1: HE_Eng
        let eng = fptr.hdu("HE_Eng")?;
        let time = eng.read_col::<i32>(&mut fptr, "Time")?;
        let bus_time_bdc_raw: Vec<u8> = eng.read_col(&mut fptr, "BUS_Time_Bdc")?;
        let mut bus_time_bdc = Vec::with_capacity(bus_time_bdc_raw.len() / 6);
        for chunk in bus_time_bdc_raw.chunks_exact(6) {
            let mut array = [0; 6];
            array.copy_from_slice(chunk);
            bus_time_bdc.push(array);
        }
        Ok(Self { time, bus_time_bdc })
    }
}
