use anyhow::{Context, Result};

pub struct EngFile {
    // HDU 1: HE_Eng
    // Only extract useful columns
    pub time: Vec<i32>,
    pub bus_time_bdc: Vec<[u8; 6]>,
}

impl EngFile {
    pub fn new(filename: &str) -> Result<Self> {
        let mut fptr = fitsio::FitsFile::open(filename)
            .with_context(|| format!("Failed to open file: {}", filename))?;

        // HDU 1: HE_Eng
        let eng = fptr
            .hdu("HE_Eng")
            .with_context(|| format!("Failed to find HDU HE_Eng in file: {}", filename))?;
        let time = eng.read_col::<i32>(&mut fptr, "Time").with_context(|| {
            format!(
                "Failed to read column Time from HDU HE_Eng in file: {}",
                filename
            )
        })?;
        let bus_time_bdc_raw: Vec<u8> =
            eng.read_col(&mut fptr, "BUS_Time_Bdc").with_context(|| {
                format!(
                    "Failed to read column BUS_Time_Bdc from HDU HE_Eng in file: {}",
                    filename
                )
            })?;
        let mut bus_time_bdc = Vec::with_capacity(bus_time_bdc_raw.len() / 6);
        for chunk in bus_time_bdc_raw.chunks_exact(6) {
            let mut array = [0; 6];
            array.copy_from_slice(chunk);
            bus_time_bdc.push(array);
        }
        Ok(Self { time, bus_time_bdc })
    }
}
