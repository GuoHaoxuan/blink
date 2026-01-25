// use blink_filter::run;
use blink_core::{traits::Chunk as _, types::MissionElapsedTime};
use blink_hxmt_he::types::Chunk;
use chrono::prelude::*;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    check_hxmt_saturation: String,
}
fn main() {
    let args = Args::parse();

    if !args.check_hxmt_saturation.is_empty() {
        let time = (args.check_hxmt_saturation + "Z")
            .parse::<DateTime<Utc>>()
            .unwrap();
        let chunk = Chunk::from_epoch(&time);
        if chunk.is_err() {
            println!("No data chunk found for the specified time: {}", time);
            return;
        }
        let chunk = chunk.unwrap();
        let met: MissionElapsedTime<blink_hxmt_he::types::HxmtHe> = time.into();
        let saturated = chunk.check_saturation(met);

        if saturated {
            println!(
                "The HXMT/HE data is saturated at the specified time: {}",
                time
            );
        } else {
            println!(
                "The HXMT/HE data is NOT saturated at the specified time: {}",
                time
            );
        }
    }
}
