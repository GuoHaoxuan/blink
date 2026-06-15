use blink_hxmt_he::algorithms::saturation::solve_events;
use blink_hxmt_he::io::level_1b::SciFile;
use blink_hxmt_he::io::level_1k::EventFile;
use chrono::prelude::*;

pub fn cmd_extract_1b(
    filtered_boxes: &[&(String, SciFile, f64)],
    met_min: f64,
    met_max: f64,
) {
    eprintln!("Extracting 1B events in [{:.3}, {:.3}]", met_min, met_max);
    println!("box,type,met,channel,det_id,pkt_idx,evt_idx");
    for (box_name, sci, offset) in filtered_boxes {
        let events = solve_events(sci, *offset, Some(met_min), Some(met_max));
        let mut n_evt = 0u64;
        let mut n_sec = 0u64;
        for evt in &events {
            let typ = if evt.is_second { "SEC" } else { "EVT" };
            println!(
                "{},{},{:.6},{},{},{},{}",
                box_name, typ, evt.met, evt.channel, evt.det_id, evt.pkt_index, evt.evt_index,
            );
            if evt.is_second { n_sec += 1; } else { n_evt += 1; }
        }
        eprintln!("  Box {}: {} events, {} seconds", box_name, n_evt, n_sec);
    }
}

pub fn cmd_extract_1k(
    epoch: DateTime<Utc>,
    box_filter: &Option<String>,
    met_min: f64,
    met_max: f64,
) {
    eprintln!("Loading 1K EventFile...");
    let evt = EventFile::from_epoch(&epoch).expect("Failed to load 1K EventFile");
    let k1_times = evt.times();
    let k1_dets = evt.det_ids();
    let k1_channels = evt.channels();

    let box_ranges: [(&str, u8, u8); 3] = [("A", 0, 5), ("B", 6, 11), ("C", 12, 17)];

    println!("box,type,met,channel,det_id");
    for (bname, d_lo, d_hi) in &box_ranges {
        if let Some(fb) = box_filter {
            if !fb.eq_ignore_ascii_case(bname) {
                continue;
            }
        }
        let mut n = 0u64;
        for i in 0..k1_times.len() {
            let (t, d, ch) = (k1_times[i], k1_dets[i], k1_channels[i]);
            if d >= *d_lo && d <= *d_hi && t >= met_min && t <= met_max {
                println!("{},EVT,{:.6},{},{}", bname, t, ch, d);
                n += 1;
            }
        }
        eprintln!("  Box {}: {} events", bname, n);
    }
}
