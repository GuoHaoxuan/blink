use super::{
    super::{algorithms::saturation::rec_sci_data, types::Hxmt},
    define::Instance,
};
use crate::types::Time;

impl Instance {
    pub fn check_saturation(&self, time: Time<Hxmt>) -> bool {
        rec_sci_data(time, &self.eng_files[0], &self.sci_files[0])
            && rec_sci_data(time, &self.eng_files[1], &self.sci_files[1])
            && rec_sci_data(time, &self.eng_files[2], &self.sci_files[2])
    }
}
