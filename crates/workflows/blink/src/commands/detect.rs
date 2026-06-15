use blink_hxmt_he::algorithms::saturation::{
    detect_fifo_reset_intervals, extract_packet_infos,
};
use blink_hxmt_he::io::level_1b::SciFile;

pub fn cmd_detect(
    filtered_boxes: &[&(String, SciFile, f64)],
    met_min: Option<f64>,
    met_max: Option<f64>,
) {
    let lo = met_min.unwrap_or(f64::NEG_INFINITY);
    let hi = met_max.unwrap_or(f64::INFINITY);
    if let (Some(a), Some(b)) = (met_min, met_max) {
        eprintln!("Detecting saturation in [{:.3}, {:.3}]", a, b);
    } else {
        eprintln!("Detecting saturation in full epoch...");
    }

    println!("box,type,start_met,stop_met,gap_s,pkt_idx,evt_idx,n_lost,log10p");
    for (box_name, sci, offset) in filtered_boxes {
        let gaps = detect_fifo_reset_intervals(sci, *offset);
        let packets = extract_packet_infos(sci, *offset);

        let mut n_fifo = 0;
        for iv in &gaps {
            if iv.stop_met < lo || iv.start_met > hi {
                continue;
            }
            let r_true = packets
                .iter()
                .find(|p| p.pkt_idx == iv.next_pkt_idx)
                .map(|p| 109.0 / p.span().max(1e-9))
                .unwrap_or(15797.0);
            let n_lost = (r_true * iv.gap_seconds).round() as usize;
            println!(
                "{},FifoReset,{:.6},{:.6},{:.6},{},-1,{},0",
                box_name, iv.start_met, iv.stop_met, iv.gap_seconds,
                iv.prev_pkt_idx, n_lost,
            );
            n_fifo += 1;
        }

        eprintln!("  Box {}: {} FIFO resets", box_name, n_fifo);
    }
}
