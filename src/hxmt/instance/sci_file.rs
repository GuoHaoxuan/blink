use anyhow::{Context, Result};

pub struct SciFile {
    pub ccsds: Vec<[u8; 882]>,
}

impl SciFile {
    pub fn new(filename: &str) -> Result<Self> {
        let mut fptr = fitsio::FitsFile::open(filename)
            .with_context(|| format!("Failed to open file: {}", filename))?;

        // HDU 1: HE_Evt_Src
        let sci = fptr
            .hdu("HE_Evt_Src")
            .with_context(|| format!("Failed to find HDU HE_Evt_Src in file: {}", filename))?;
        let ccsds_raw: Vec<u8> = sci.read_col(&mut fptr, "CCSDS").with_context(|| {
            format!(
                "Failed to read column CCSDS from HDU HE_Evt_Src in file: {}",
                filename
            )
        })?;
        let mut ccsds_array = Vec::with_capacity(ccsds_raw.len() / 882);
        for chunk in ccsds_raw.chunks_exact(882) {
            let mut array = [0; 882];
            array.copy_from_slice(chunk);
            ccsds_array.push(array);
        }
        Ok(Self { ccsds: ccsds_array })
    }
}
