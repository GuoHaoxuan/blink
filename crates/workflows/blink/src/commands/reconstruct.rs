use blink_hxmt_he::algorithms::saturation::{
    detect_fifo_reset_intervals, detect_unreliable_intervals, extract_packet_infos,
    reconstruct_gaps, reconstruct_met_times, reconstruct_with_wrap_tracking,
    BoxReconstructionData,
};
use blink_hxmt_he::io::level_1b::SciFile;

use crate::cli::ReconstructArgs;

pub fn cmd_reconstruct(
    args: &ReconstructArgs,
    boxes: &[(String, SciFile, f64)],
    filter_box: &Option<String>,
) {
    let met_min = args.window.met_min();
    let met_max = args.window.met_max();

    eprintln!("Preparing reconstruction data...");
    let mut box_data: Vec<(String, BoxReconstructionData)> = Vec::new();
    for (box_name, sci, offset) in boxes {
        let events = reconstruct_met_times(sci, *offset);
        let gaps = detect_fifo_reset_intervals(sci, *offset);
        let packets = extract_packet_infos(sci, *offset);
        let packet_events: Vec<Vec<f64>> = reconstruct_with_wrap_tracking(sci, *offset)
            .into_iter()
            .map(|mut times| {
                times.retain(|t| !t.is_nan());
                times.sort_by(|a, b| a.partial_cmp(b).unwrap());
                times
            })
            .collect();
        let unreliable = detect_unreliable_intervals(&gaps, &packets, &packet_events);
        eprintln!(
            "  Box {}: {} events, {} gaps, {} unreliable, {} packets",
            box_name, events.len(), gaps.len(), unreliable.len(), packets.len()
        );
        box_data.push((
            box_name.clone(),
            BoxReconstructionData { events, gaps, packets, packet_events, unreliable },
        ));
    }

    let original_events: Vec<(String, Vec<f64>)> = box_data
        .iter()
        .map(|(name, data)| (name.clone(), data.events.clone()))
        .collect();

    eprintln!("Reconstructing (FIFO reset gaps)...");
    let mut all_filled: Vec<(String, Vec<f64>)> = Vec::new();

    for i in 0..box_data.len() {
        let refs: Vec<&BoxReconstructionData> = box_data
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, (_, d))| d)
            .collect();

        let gap_results = reconstruct_gaps(&box_data[i].1, &refs);
        let n_gap_filled: usize = gap_results.iter().map(|r| r.n_lost).sum();
        let n_gap_ref = gap_results.iter().filter(|r| r.has_cross_ref).count();
        let mut gap_events: Vec<f64> = gap_results
            .into_iter()
            .flat_map(|r| r.filled_events)
            .collect();
        gap_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

        eprintln!(
            "  Box {}: gaps={} ({} evt, {} ref)",
            box_data[i].0, box_data[i].1.gaps.len(), n_gap_filled, n_gap_ref,
        );

        all_filled.push((box_data[i].0.clone(), gap_events));
    }

    println!("box,type,met,channel,pkt_idx,evt_idx");
    for (box_name, _data) in &box_data {
        let obs_events = original_events
            .iter()
            .find(|(n, _)| n == box_name)
            .map(|(_, e)| e.as_slice())
            .unwrap_or(&[]);
        let gap_events = all_filled
            .iter()
            .find(|(n, _)| n == box_name)
            .map(|(_, f)| f.as_slice())
            .unwrap_or(&[]);

        if let Some(fb) = filter_box {
            if !box_name.eq_ignore_ascii_case(fb) {
                continue;
            }
        }

        let mut n_obs = 0u64;
        let mut n_gap = 0u64;

        for &t in obs_events {
            if t >= met_min && t <= met_max {
                println!("{},EVT,{:.6},0,-1,-1", box_name, t);
                n_obs += 1;
            }
        }
        for &t in gap_events {
            if t >= met_min && t <= met_max {
                println!("{},FILL_GAP,{:.6},0,-1,-1", box_name, t);
                n_gap += 1;
            }
        }

        eprintln!(
            "  Box {}: {} observed, {} gap-filled, bin={:.3}s",
            box_name, n_obs, n_gap, args.bin,
        );
    }
}
