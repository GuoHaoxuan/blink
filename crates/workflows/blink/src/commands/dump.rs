use blink_hxmt_he::algorithms::saturation::{
    check_byte_offsets, diagnose_packets, dump_event_details, dump_ptime_utc,
    extract_second_event_times, reconstruct_met_times, reconstruct_with_wrap_tracking_labeled,
    scan_saturation_intervals_raw,
};
use blink_hxmt_he::io::level_1b::SciFile;

use crate::cli::{DumpBurstArgs, DumpHistArgs};

pub fn cmd_dump_times(
    args: &DumpBurstArgs,
    boxes: &[&(String, SciFile, f64)],
) {
    let met_min = args.met_min();
    let met_max = args.met_max();

    eprintln!(
        "Dumping times in [{:.3}, {:.3}] (trigger={:.3}, before={:.1}, after={:.1})",
        met_min, met_max, args.trigger_met(), args.before, args.after
    );

    println!("# trigger_met={:.6}", args.trigger_met());
    println!("# before={:.1}", args.before);
    println!("# after={:.1}", args.after);

    for (box_name, sci, offset) in boxes {
        eprintln!("Box {} (offset={:.0}) ...", box_name, offset);

        let all_met = reconstruct_met_times(sci, *offset);
        let n_total = all_met.len();

        let filtered: Vec<f64> = all_met
            .into_iter()
            .filter(|&t| t >= met_min && t <= met_max)
            .collect();

        eprintln!(
            "  Box {}: {}/{} events in window",
            box_name, filtered.len(), n_total
        );
        println!(
            "# box={} n_total={} n_window={}",
            box_name, n_total, filtered.len()
        );
        for t in &filtered {
            println!("{:.6}", t);
        }
    }

    println!("# saturation_intervals");
    for (box_name, sci, offset) in boxes {
        let intervals = scan_saturation_intervals_raw(sci, *offset);
        for (start, stop) in &intervals {
            if *stop >= met_min && *start <= met_max {
                println!("SAT,{},{:.6},{:.6}", box_name, start, stop);
            }
        }
    }
}

pub fn cmd_dump_packets(
    args: &DumpBurstArgs,
    filtered_boxes: &[&(String, SciFile, f64)],
    boxes: &[(String, SciFile, f64)],
) {
    let met_min = args.met_min();
    let met_max = args.met_max();

    println!("box,pkt_idx,min_time,max_time,n_events");
    for (box_name, sci, offset) in filtered_boxes {
        let packet_times = reconstruct_with_wrap_tracking_labeled(sci, *offset, box_name);
        for (pkt_idx, times) in packet_times.iter().enumerate() {
            let mut clean: Vec<f64> = times.iter().copied().filter(|t| !t.is_nan()).collect();
            if clean.is_empty() {
                continue;
            }
            clean.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let min_t = clean[0];
            let max_t = clean[clean.len() - 1];
            if max_t >= met_min && min_t <= met_max {
                println!("{},{},{:.6},{:.6},{}", box_name, pkt_idx, min_t, max_t, times.len());
            }
        }
    }
    println!("# second_events");
    for (box_name, sci, offset) in boxes {
        let sec_times = extract_second_event_times(sci, *offset);
        for t in &sec_times {
            if *t >= met_min && *t <= met_max {
                println!("SEC,{},{:.6}", box_name, t);
            }
        }
    }
}

pub fn cmd_dump_events(
    args: &DumpBurstArgs,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    let met_min = args.met_min();
    let met_max = args.met_max();

    eprintln!("Dumping events in [{:.3}, {:.3}]", met_min, met_max);

    println!("# pkt,evt,is_second,ptime,channel,MET,r0,r1,r2,r3,r4,r5,r6,r7");
    for (box_name, sci, offset) in filtered_boxes {
        let events = dump_event_details(sci, *offset, met_min, met_max);
        for evt in &events {
            println!(
                "{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{}",
                box_name, evt.pkt_index, evt.evt_index,
                if evt.is_second { "SEC" } else { "EVT" },
                evt.channel, evt.met,
                evt.raw_bytes[0], evt.raw_bytes[1], evt.raw_bytes[2], evt.raw_bytes[3],
                evt.raw_bytes[4], evt.raw_bytes[5], evt.raw_bytes[6], evt.raw_bytes[7],
            );
        }
    }
}

pub fn cmd_dump_hist(
    args: &DumpHistArgs,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    let met_min = args.window.met_min();
    let met_max = args.window.met_max();
    let bin_width = args.bin;
    let n_bins = ((met_max - met_min) / bin_width).ceil() as usize;

    eprintln!(
        "Histogram: [{:.3}, {:.3}], bin_width={:.4}s, n_bins={}",
        met_min, met_max, bin_width, n_bins
    );

    let mut hist = vec![0u64; n_bins];
    let mut n_total = 0u64;

    for (box_name, sci, offset) in filtered_boxes {
        let all_met = reconstruct_met_times(sci, *offset);
        let n_box = all_met.len();
        for t in &all_met {
            if *t >= met_min && *t < met_max {
                let idx = ((*t - met_min) / bin_width) as usize;
                if idx < n_bins {
                    hist[idx] += 1;
                    n_total += 1;
                }
            }
        }
        eprintln!("  Box {}: {}/{} events in window", box_name, n_box, all_met.len());
    }

    eprintln!("  Total in hist: {}", n_total);

    println!("# trigger_met={:.6}", met_min);
    println!("# before={:.1}, after={:.1}", args.window.before, args.window.after);
    println!("# bin_width={:.6}", bin_width);
    println!("# n_bins={}", n_bins);
    println!("# n_total={}", n_total);

    println!("# HIST");
    for (i, count) in hist.iter().enumerate() {
        let bin_start = met_min + i as f64 * bin_width;
        println!("{:.6},{}", bin_start, count);
    }

    println!("# SAT");
    for (box_name, sci, offset) in filtered_boxes {
        let intervals = scan_saturation_intervals_raw(sci, *offset);
        for (start, stop) in &intervals {
            if *stop >= met_min && *start <= met_max {
                println!("SAT,{},{:.6},{:.6}", box_name, start, stop);
            }
        }
    }
}

pub fn cmd_dump_diag(
    args: &DumpBurstArgs,
    boxes: &[&(String, SciFile, f64)],
) {
    let met_min = args.met_min();
    let met_max = args.met_max();

    println!(
        "box,pkt,n_evt,n_sec,n_sec_valid,n_err,n_out,n_drop,anchor,utc_tail,met_min,met_max,n_0x5a"
    );

    for (box_name, sci, offset) in boxes {
        let diags = diagnose_packets(sci, *offset);
        let mut total_out = 0u64;
        let mut total_drop = 0u64;
        let mut total_err = 0u64;
        let mut total_sec = 0u64;
        let mut total_sec_valid = 0u64;
        let mut pkts_in_window = 0u64;
        let mut pkts_no_anchor = 0u64;

        for d in &diags {
            let pkt_met_min = d.met_min.unwrap_or(d.utc_tail);
            let pkt_met_max = d.met_max.unwrap_or(d.utc_tail);
            let in_window = pkt_met_max >= met_min && pkt_met_min <= met_max;
            let utc_in_window = d.utc_tail >= met_min && d.utc_tail <= met_max;
            if !in_window && !utc_in_window {
                continue;
            }
            pkts_in_window += 1;
            total_out += d.n_output as u64;
            total_drop += d.n_dropped as u64;
            total_err += d.n_error as u64;
            total_sec += d.n_second as u64;
            total_sec_valid += d.n_second_valid as u64;
            if !d.has_anchor {
                pkts_no_anchor += 1;
            }
            let dump_all = std::env::var("DUMP_ALL").map(|v| v == "1").unwrap_or(false);
            if dump_all || d.n_error > 0 || d.n_dropped > 0 {
                println!(
                    "{},{},{},{},{},{},{},{},{},{:.3},{},{},{}",
                    box_name, d.pkt_index, d.n_event, d.n_second, d.n_second_valid,
                    d.n_error, d.n_output, d.n_dropped,
                    if d.has_anchor { "Y" } else { "N" },
                    d.utc_tail,
                    d.met_min.map_or("-".to_string(), |v| format!("{:.3}", v)),
                    d.met_max.map_or("-".to_string(), |v| format!("{:.3}", v)),
                    d.n_0x5a,
                );
            }
        }
        eprintln!(
            "Box {}: {} pkts in window, {} out, {} dropped, {} CRC errors, {}/{} seconds valid, {} pkts lost anchor",
            box_name, pkts_in_window, total_out, total_drop, total_err,
            total_sec_valid, total_sec, pkts_no_anchor
        );
    }
}

pub fn cmd_dump_ptime(
    epoch: &str,
    pkt_min: usize,
    pkt_max: usize,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    for (box_name, sci, offset) in filtered_boxes {
        eprintln!("[{}] Box {} pkt {}..{}", epoch, box_name, pkt_min, pkt_max);
        dump_ptime_utc(sci, *offset, pkt_min, pkt_max);
    }
}

pub fn cmd_dump_check_offset(
    epoch: &str,
    pkt_min: usize,
    pkt_max: usize,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    for (box_name, sci, _offset) in filtered_boxes {
        eprintln!("[{}] Box {} checking offsets for packets {}..{}", epoch, box_name, pkt_min, pkt_max);
        check_byte_offsets(sci, pkt_min, pkt_max);
    }
}
