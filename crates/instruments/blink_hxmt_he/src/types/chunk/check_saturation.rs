use super::Chunk;
use crate::{algorithms::saturation::rec_sci_data, types::HxmtHe};
use blink_core::types::MissionElapsedTime;

impl Chunk {
    pub fn check_saturation(&self, time: MissionElapsedTime<HxmtHe>) -> bool {
        rec_sci_data(time, &self.eng_files[0], &self.sci_files[0])
            && rec_sci_data(time, &self.eng_files[1], &self.sci_files[1])
            && rec_sci_data(time, &self.eng_files[2], &self.sci_files[2])
    }
}
