use blink_hxmt_he::algorithms::saturation::{
    check_byte_offsets, detect_fifo_reset_intervals, detect_silent_drops, diagnose_packets,
    dump_event_details, dump_ptime_utc, extract_packet_infos, extract_second_event_times, solve_events,
    reconstruct_deep_saturation, reconstruct_gaps, reconstruct_met_times,
    reconstruct_silent_drops, reconstruct_with_wrap_tracking,
    reconstruct_with_wrap_tracking_labeled,
    scan_saturation_intervals_raw, BoxReconstructionData, detect_unreliable_intervals,
};
use blink_hxmt_he::io::level_1b::{SciFile, get_eng_filenames, get_sci_filenames, read_stime_offset};
use blink_hxmt_he::io::level_1k::EventFile;
use chrono::prelude::*;
use clap::{Args, Parser, Subcommand};

// ── CLI structs ──────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(about = "HXMT HE analysis toolkit")]
struct Cli {
    #[command(subcommand)]
    command: TopCommands,
}

#[derive(Subcommand)]
enum TopCommands {
    /// Saturation analysis (detection, reconstruction, comparison)
    Sat {
        /// Epoch in YYYY-MM-DDTHH or full ISO 8601 format
        epoch: String,

        /// Filter to a single box (a, b, or c)
        #[arg(long = "box")]
        box_filter: Option<String>,

        #[command(subcommand)]
        command: SatCommands,
    },
    /// TGF search (scan date range for candidate signals)
    Search {
        /// Start date (YYYY-MM-DD)
        from: String,
        /// End date (YYYY-MM-DD)
        to: String,
    },
    /// TGF filter (lightning association for detected signals)
    Filter,
}

#[derive(Subcommand)]
enum SatCommands {
    /// Time reconstruction: solve event MET from 1B raw data (step 1)
    Solve,
    /// Saturation detection: find FIFO resets + silent drops (step 2)
    Detect,
    /// Light curve reconstruction: fill saturated gaps (step 3)
    Reconstruct(ReconstructArgs),
    /// Compare 1B vs 1K event data
    Compare(CompareArgs),
    /// Diagnostic dump tools
    Dump {
        #[command(subcommand)]
        sub: DumpCommands,
    },
}

#[derive(Args)]
struct TimeWindow {
    /// Center time (MET number or UTC datetime, e.g. 339945422.0 or 2022-10-09T13:37:02)
    center_met: String,
    /// Half window size in seconds
    half_window: f64,
}

impl TimeWindow {
    fn met(&self) -> f64 {
        parse_met_or_utc(&self.center_met)
    }
}

#[derive(Args)]
struct PacketRange {
    /// Minimum packet index
    pkt_min: usize,
    /// Maximum packet index
    pkt_max: usize,
}

#[derive(Args)]
struct ReconstructArgs {
    #[command(flatten)]
    window: TimeWindow,
    /// Bin width in seconds
    #[arg(short, long, default_value_t = 1.0)]
    bin_width: f64,
}

#[derive(Args)]
struct CompareArgs {
    #[command(flatten)]
    window: TimeWindow,
    /// Coarse bin width in seconds
    #[arg(long, default_value_t = 1.0)]
    coarse_bin: f64,
    /// Fine bin width in seconds
    #[arg(long, default_value_t = 0.1)]
    fine_bin: f64,
    /// Max lag in ms for cross-correlation
    #[arg(long, default_value_t = 50)]
    max_lag: usize,
    /// Threshold percentage for flagging fine bins
    #[arg(long, default_value_t = 30.0)]
    threshold: f64,
    /// Output CSV format
    #[arg(long)]
    csv: bool,
}

#[derive(Subcommand)]
enum DumpCommands {
    /// Dump event MET times
    Times(TimeWindow),
    /// Dump packet time ranges
    Packets(TimeWindow),
    /// Dump event details
    Events(TimeWindow),
    /// Dump histogram
    Hist(HistArgs),
    /// Per-packet diagnostics
    Diag(TimeWindow),
    /// Dump ptime/UTC mapping
    Ptime(PacketRange),
    /// Check byte offsets for CRC
    CheckOffset(PacketRange),
}

#[derive(Args)]
struct HistArgs {
    #[command(flatten)]
    window: TimeWindow,
    /// Bin width in seconds
    #[arg(short, long, default_value_t = 0.01)]
    bin_width: f64,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// HXMT MET epoch: 2012-01-01T00:00:00 UTC
const MET_EPOCH: &str = "2012-01-01T00:00:00Z";

fn parse_epoch(epoch_str: &str) -> DateTime<Utc> {
    epoch_str.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        format!("{}:00:00Z", epoch_str)
            .parse::<DateTime<Utc>>()
            .expect("Invalid datetime format. Use YYYY-MM-DDTHH or full ISO 8601.")
    })
}

/// Parse a time argument that can be either MET (float) or UTC (datetime string).
/// Returns MET as f64.
fn parse_met_or_utc(s: &str) -> f64 {
    // If it parses as a float, treat as MET
    if let Ok(met) = s.parse::<f64>() {
        return met;
    }
    // Otherwise try UTC datetime
    let ref_time = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let utc = s.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        // Try common short formats
        format!("{}Z", s).parse::<DateTime<Utc>>()
            .or_else(|_| format!("{}:00Z", s).parse::<DateTime<Utc>>())
            .or_else(|_| format!("{}:00:00Z", s).parse::<DateTime<Utc>>())
            .expect("Invalid time format. Use MET number or UTC datetime (e.g. 2020-04-15T08:34:48)")
    });
    let met = (utc - ref_time).num_microseconds().unwrap() as f64 / 1e6;
    eprintln!("  UTC {} -> MET {:.6}", utc.format("%Y-%m-%dT%H:%M:%S"), met);
    met
}

fn load_boxes(epoch: DateTime<Utc>) -> Vec<(String, SciFile, f64)> {
    let sci_pairs = get_sci_filenames(epoch);
    let eng_pairs = get_eng_filenames(epoch);

    sci_pairs
        .iter()
        .filter_map(|(box_name, sci_path)| {
            let sci = SciFile::new(sci_path).ok()?;
            let offset = eng_pairs
                .iter()
                .find(|(bn, _)| bn == box_name)
                .and_then(|(_, eng_path)| read_stime_offset(eng_path).ok())
                .unwrap_or(0.0);
            Some((box_name.clone(), sci, offset))
        })
        .collect()
}

fn filter_boxes<'a>(
    boxes: &'a [(String, SciFile, f64)],
    filter: &Option<String>,
) -> Vec<&'a (String, SciFile, f64)> {
    if let Some(fb) = filter {
        boxes.iter().filter(|(name, _, _)| name.eq_ignore_ascii_case(fb)).collect()
    } else {
        boxes.iter().collect()
    }
}

// ── Command implementations ─────────────────────────────────────────────────

fn cmd_solve(
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    println!("box,type,met,channel,pkt_idx,evt_idx");
    for (box_name, sci, offset) in filtered_boxes {
        let events = solve_events(sci, *offset, None, None);
        let mut n_evt = 0u64;
        let mut n_sec = 0u64;
        for evt in &events {
            let typ = if evt.is_second { "SEC" } else { "EVT" };
            println!(
                "{},{},{:.6},{},{},{}",
                box_name, typ, evt.met, evt.channel, evt.pkt_index, evt.evt_index,
            );
            if evt.is_second { n_sec += 1; } else { n_evt += 1; }
        }
        // CRC 错误数 = 总 slot 数 - 输出数
        let n_total_slots = sci.ccsds.len() * 109;
        let n_err = n_total_slots as u64 - n_evt - n_sec;
        eprintln!("  Box {}: {} events, {} seconds, {} CRC errors",
                  box_name, n_evt, n_sec, n_err);
    }
}

fn cmd_detect(filtered_boxes: &[&(String, SciFile, f64)]) {
    println!("box,type,start_met,stop_met,gap_s,pkt_idx,evt_idx,n_lost,log10p");
    for (box_name, sci, offset) in filtered_boxes {
        let events = reconstruct_met_times(sci, *offset);
        let gaps = detect_fifo_reset_intervals(sci, *offset);
        let packets = extract_packet_infos(sci, *offset);
        let packet_events: Vec<Vec<f64>> = reconstruct_with_wrap_tracking(sci, *offset)
            .into_iter()
            .map(|mut t| {
                t.sort_by(|a, b| a.partial_cmp(b).unwrap());
                t
            })
            .collect();

        // FifoReset: pkt_idx = gap 之前的包, evt_idx = -1 (无包内位置)
        eprintln!("Box {}: {} FIFO reset intervals", box_name, gaps.len());
        for iv in &gaps {
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
        }

        // SilentDrop: pkt_idx = 所在包, evt_idx = 包内间隔位置
        let unreliable = detect_unreliable_intervals(&gaps, &packets, &packet_events);
        let box_data = BoxReconstructionData {
            events,
            gaps,
            packets,
            packet_events,
            unreliable,
        };
        let drops = detect_silent_drops(&box_data);
        eprintln!("Box {}: {} silent drops", box_name, drops.len());
        for d in &drops {
            println!(
                "{},SilentDrop,{:.6},{:.6},{:.6},{},{},{},{:.1}",
                box_name, d.start_met, d.stop_met, d.dt, d.pkt_idx, d.evt_idx, d.n_lost,
                d.log10_p,
            );
        }
    }
}

fn cmd_reconstruct(
    args: &ReconstructArgs,
    boxes: &[(String, SciFile, f64)],
    filter_box: &Option<String>,
) {
    let center_met = args.window.met();
    let half_window = args.window.half_window;
    let bin_width = args.bin_width;

    let met_min = center_met - half_window;
    let met_max = center_met + half_window;

    eprintln!("Preparing reconstruction data...");
    let mut box_data: Vec<(String, BoxReconstructionData)> = Vec::new();
    for (box_name, sci, offset) in boxes {
        let events = reconstruct_met_times(sci, *offset);
        let gaps = detect_fifo_reset_intervals(sci, *offset);
        let packets = extract_packet_infos(sci, *offset);
        let packet_events: Vec<Vec<f64>> = reconstruct_with_wrap_tracking(sci, *offset)
            .into_iter()
            .map(|mut times| {
                times.sort_by(|a, b| a.partial_cmp(b).unwrap());
                times
            })
            .collect();
        let unreliable = detect_unreliable_intervals(&gaps, &packets, &packet_events);
        eprintln!(
            "  Box {}: {} events, {} gaps, {} unreliable, {} packets",
            box_name,
            events.len(),
            gaps.len(),
            unreliable.len(),
            packets.len()
        );
        box_data.push((
            box_name.clone(),
            BoxReconstructionData {
                events,
                gaps,
                packets,
                packet_events,
                unreliable,
            },
        ));
    }

    let original_events: Vec<(String, Vec<f64>)> = box_data
        .iter()
        .map(|(name, data)| (name.clone(), data.events.clone()))
        .collect();

    eprintln!("Reconstructing (silent drops + FIFO reset gaps, independent)...");
    let mut all_sd_filled: Vec<(String, Vec<f64>)> = Vec::new();
    let mut all_filled: Vec<(String, Vec<f64>)> = Vec::new();

    for i in 0..box_data.len() {
        let refs: Vec<&BoxReconstructionData> = box_data
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, (_, d))| d)
            .collect();

        let drops = detect_silent_drops(&box_data[i].1);
        let sd_results = reconstruct_silent_drops(&box_data[i].1, &drops, &refs);
        let n_sd_filled: usize = sd_results.iter().map(|r| r.n_lost).sum();
        let _n_sd_ref = sd_results.iter().filter(|r| r.has_cross_ref).count();
        let mut sd_events: Vec<f64> = sd_results
            .into_iter()
            .flat_map(|r| r.filled_events)
            .collect();
        sd_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let gap_results = reconstruct_gaps(&box_data[i].1, &refs);
        let n_gap_filled: usize = gap_results.iter().map(|r| r.n_lost).sum();
        let n_gap_ref = gap_results.iter().filter(|r| r.has_cross_ref).count();
        let mut gap_events: Vec<f64> = gap_results
            .into_iter()
            .flat_map(|r| r.filled_events)
            .collect();
        gap_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let ds_results = reconstruct_deep_saturation(&box_data[i].1);
        let n_ds_count = ds_results.len();
        let n_ds_filled: usize = ds_results.iter().map(|r| r.n_lost).sum();
        let mut ds_events: Vec<f64> = ds_results
            .into_iter()
            .flat_map(|r| r.filled_events)
            .collect();
        ds_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

        sd_events.extend_from_slice(&ds_events);
        sd_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

        eprintln!(
            "  Box {}: sd={} ({} evt) | deep_sat={} ({} evt) | gaps={} ({} evt, {} ref)",
            box_data[i].0,
            drops.len(),
            n_sd_filled,
            n_ds_count,
            n_ds_filled,
            box_data[i].1.gaps.len(),
            n_gap_filled,
            n_gap_ref,
        );

        all_sd_filled.push((box_data[i].0.clone(), sd_events));
        all_filled.push((box_data[i].0.clone(), gap_events));
    }

    println!("box,bin_center,observed,reconstructed,filled_gap,filled_sd");
    let bins: Vec<f64> = {
        let mut v = Vec::new();
        let mut t = met_min;
        while t < met_max {
            v.push(t);
            t += bin_width;
        }
        v.push(met_max);
        v
    };

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
        let sd_events = all_sd_filled
            .iter()
            .find(|(n, _)| n == box_name)
            .map(|(_, f)| f.as_slice())
            .unwrap_or(&[]);

        if let Some(fb) = filter_box {
            if box_name != fb {
                continue;
            }
        }

        for w in bins.windows(2) {
            let bin_lo = w[0];
            let bin_hi = w[1];
            let bin_center = (bin_lo + bin_hi) / 2.0;

            let count = |events: &[f64]| -> usize {
                events.partition_point(|&t| t < bin_hi) - events.partition_point(|&t| t < bin_lo)
            };

            let n_obs = count(obs_events);
            let n_gap = count(gap_events);
            let n_sd = count(sd_events);
            let n_total = n_obs + n_gap + n_sd;

            println!(
                "{},{:.6},{:.1},{:.1},{:.1},{:.1}",
                box_name,
                bin_center,
                n_obs as f64 / bin_width,
                n_total as f64 / bin_width,
                n_gap as f64 / bin_width,
                n_sd as f64 / bin_width,
            );
        }
    }
}

fn cmd_dump_times(
    window: &TimeWindow,
    boxes: &[(String, SciFile, f64)],
) {
    let met_min = window.met() - window.half_window;
    let met_max = window.met() + window.half_window;

    eprintln!(
        "Dumping times in [{:.3}, {:.3}] (center={:.3}, half_window={:.1})",
        met_min, met_max, window.met(), window.half_window
    );

    println!("# center_met={:.6}", window.met());
    println!("# half_window={:.1}", window.half_window);

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
            box_name,
            filtered.len(),
            n_total
        );
        println!(
            "# box={} n_total={} n_window={}",
            box_name,
            n_total,
            filtered.len()
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

fn cmd_dump_packets(
    window: &TimeWindow,
    filtered_boxes: &[&(String, SciFile, f64)],
    boxes: &[(String, SciFile, f64)],
) {
    let met_min = window.met() - window.half_window;
    let met_max = window.met() + window.half_window;

    println!("box,pkt_idx,min_time,max_time,n_events");
    for (box_name, sci, offset) in filtered_boxes {
        let packet_times = reconstruct_with_wrap_tracking_labeled(sci, *offset, box_name);
        for (pkt_idx, times) in packet_times.iter().enumerate() {
            if times.is_empty() {
                continue;
            }
            let min_t = *times
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let max_t = *times
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max_t >= met_min && min_t <= met_max {
                println!(
                    "{},{},{:.6},{:.6},{}",
                    box_name,
                    pkt_idx,
                    min_t,
                    max_t,
                    times.len()
                );
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

fn cmd_dump_events(
    window: &TimeWindow,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    let met_min = window.met() - window.half_window;
    let met_max = window.met() + window.half_window;

    eprintln!("Dumping events in [{:.3}, {:.3}]", met_min, met_max);

    println!("# pkt,evt,is_second,ptime,channel,MET,r0,r1,r2,r3,r4,r5,r6,r7");
    for (box_name, sci, offset) in filtered_boxes {
        let events = dump_event_details(sci, *offset, met_min, met_max);
        for evt in &events {
            println!(
                "{},{},{},{},{},{:.6},{},{},{},{},{},{},{},{}",
                box_name,
                evt.pkt_index,
                evt.evt_index,
                if evt.is_second { "SEC" } else { "EVT" },
                evt.channel,
                evt.met,
                evt.raw_bytes[0],
                evt.raw_bytes[1],
                evt.raw_bytes[2],
                evt.raw_bytes[3],
                evt.raw_bytes[4],
                evt.raw_bytes[5],
                evt.raw_bytes[6],
                evt.raw_bytes[7],
            );
        }
    }
}

fn cmd_dump_hist(
    args: &HistArgs,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    let center_met = args.window.met();
    let half_window = args.window.half_window;
    let bin_width = args.bin_width;

    let met_min = center_met - half_window;
    let met_max = center_met + half_window;
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
        eprintln!(
            "  Box {}: {}/{} events in window",
            box_name,
            n_box,
            all_met.len()
        );
    }

    eprintln!("  Total in hist: {}", n_total);

    println!("# center_met={:.6}", center_met);
    println!("# half_window={:.1}", half_window);
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

fn cmd_dump_diag(
    window: &TimeWindow,
    boxes: &[(String, SciFile, f64)],
) {
    let met_min = window.met() - window.half_window;
    let met_max = window.met() + window.half_window;

    println!(
        "box,pkt,n_evt,n_sec,n_sec_valid,n_err,n_out,n_drop,anchor,utc_tail,met_min,met_max"
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
            if d.n_error > 0 || d.n_dropped > 0 {
                println!(
                    "{},{},{},{},{},{},{},{},{},{:.3},{},{}",
                    box_name,
                    d.pkt_index,
                    d.n_event,
                    d.n_second,
                    d.n_second_valid,
                    d.n_error,
                    d.n_output,
                    d.n_dropped,
                    if d.has_anchor { "Y" } else { "N" },
                    d.utc_tail,
                    d.met_min
                        .map_or("-".to_string(), |v| format!("{:.3}", v)),
                    d.met_max
                        .map_or("-".to_string(), |v| format!("{:.3}", v)),
                );
            }
        }
        eprintln!(
            "Box {}: {} pkts in window, {} out, {} dropped, {} CRC errors, {}/{} seconds valid, {} pkts lost anchor",
            box_name,
            pkts_in_window,
            total_out,
            total_drop,
            total_err,
            total_sec_valid,
            total_sec,
            pkts_no_anchor
        );
    }
}

fn cmd_dump_ptime(
    range: &PacketRange,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    for (box_name, sci, offset) in filtered_boxes {
        eprintln!("Box {} pkt {}..{}", box_name, range.pkt_min, range.pkt_max);
        dump_ptime_utc(sci, *offset, range.pkt_min, range.pkt_max);
    }
}

fn cmd_dump_check_offset(
    range: &PacketRange,
    filtered_boxes: &[&(String, SciFile, f64)],
) {
    for (box_name, sci, _offset) in filtered_boxes {
        eprintln!(
            "Box {} checking offsets for packets {}..{}",
            box_name, range.pkt_min, range.pkt_max
        );
        check_byte_offsets(sci, range.pkt_min, range.pkt_max);
    }
}

fn cmd_compare(
    args: &CompareArgs,
    boxes: &[(String, SciFile, f64)],
    epoch: DateTime<Utc>,
    filter_box: &Option<String>,
) {
    let center_met = args.window.met();
    let half_window = args.window.half_window;
    let met_min = center_met - half_window;
    let met_max = center_met + half_window;

    // Load 1B times per box
    eprintln!("Loading 1B times...");
    let mut b1: Vec<(String, Vec<f64>)> = Vec::new();
    for (box_name, sci, offset) in boxes {
        if let Some(fb) = filter_box {
            if box_name != fb {
                continue;
            }
        }
        let mut times = reconstruct_met_times(sci, *offset);
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!("  1B Box {}: {} events", box_name, times.len());
        b1.push((box_name.clone(), times));
    }

    // Load 1K times, split by det_id into boxes
    eprintln!("Loading 1K times...");
    let evt = EventFile::from_epoch(&epoch).expect("Failed to load 1K EventFile");
    let k1_times = evt.times();
    let k1_dets = evt.det_ids();

    let box_ranges: [(&str, u8, u8); 3] = [("A", 0, 5), ("B", 6, 11), ("C", 12, 17)];
    let mut k1: Vec<(String, Vec<f64>)> = Vec::new();
    for (bname, d_lo, d_hi) in &box_ranges {
        if let Some(fb) = filter_box {
            if fb != bname {
                continue;
            }
        }
        let mut times: Vec<f64> = k1_times
            .iter()
            .zip(k1_dets.iter())
            .filter(|&(_, &d)| d >= *d_lo && d <= *d_hi)
            .map(|(&t, _)| t)
            .collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!("  1K Box {}: {} events", bname, times.len());
        k1.push((bname.to_string(), times));
    }

    if args.csv {
        println!("box,t_rel,n_1k,n_1b,delta,delta_pct,bin_type");
    }

    // Per-box comparison
    for (bname, k1_times) in &k1 {
        let b1_times = match b1.iter().find(|(n, _)| n == bname) {
            Some((_, t)) => t.as_slice(),
            None => continue,
        };

        // --- Coarse bins ---
        if !args.csv {
            println!("\n--- Box {} ---", bname);
            println!(
                "  {:>5} {:>7} {:>7} {:>7} {:>8}",
                "T+", "1K", "1B", "delta", "delta%"
            );
        }

        let n_coarse = ((met_max - met_min) / args.coarse_bin).ceil() as usize;
        for i in 0..n_coarse {
            let t0 = met_min + i as f64 * args.coarse_bin;
            let t1 = t0 + args.coarse_bin;
            let t_rel = t0 - met_min;

            let n_1k = k1_times.partition_point(|&t| t < t1)
                - k1_times.partition_point(|&t| t < t0);
            let n_1b = b1_times.partition_point(|&t| t < t1)
                - b1_times.partition_point(|&t| t < t0);

            let delta = n_1b as i64 - n_1k as i64;
            let delta_pct = if n_1k > 0 {
                delta as f64 / n_1k as f64 * 100.0
            } else if n_1b > 0 {
                f64::INFINITY
            } else {
                0.0
            };

            if args.csv {
                println!(
                    "{},{:.1},{},{},{},{:.1},coarse",
                    bname, t_rel, n_1k, n_1b, delta, delta_pct
                );
            } else if n_1k >= 5 || n_1b >= 5 {
                let note = if n_1k > 50 && n_1b == 0 {
                    "*** HOLE ***"
                } else if n_1b > 50 && n_1k == 0 {
                    "*** EXTRA ***"
                } else if delta_pct.abs() > 50.0 && n_1k > 20 {
                    "*** MISMATCH ***"
                } else if delta_pct.abs() > 20.0 && n_1k > 20 {
                    "** mismatch **"
                } else if delta_pct.abs() > 10.0 && n_1k > 50 {
                    "* slight *"
                } else {
                    ""
                };
                println!(
                    "  T+{:3.0} {:7} {:7} {:+7} {:+8.1}%  {}",
                    t_rel, n_1k, n_1b, delta, delta_pct, note
                );
            }
        }

        // --- Fine bins with |delta%| > threshold ---
        if !args.csv {
            println!(
                "Fine bins with |delta| > {:.0}%:",
                args.threshold
            );
        }

        let n_fine = ((met_max - met_min) / args.fine_bin).ceil() as usize;
        let mut n_prob = 0;
        for i in 0..n_fine {
            let t0 = met_min + i as f64 * args.fine_bin;
            let t1 = t0 + args.fine_bin;
            let t_rel = t0 - met_min;

            let n_1k = k1_times.partition_point(|&t| t < t1)
                - k1_times.partition_point(|&t| t < t0);
            let n_1b = b1_times.partition_point(|&t| t < t1)
                - b1_times.partition_point(|&t| t < t0);

            if n_1k < 3 && n_1b < 3 {
                continue;
            }

            let delta = n_1b as i64 - n_1k as i64;
            let denom = n_1k.max(1) as f64;
            let delta_pct = delta as f64 / denom * 100.0;

            if args.csv {
                if delta_pct.abs() > args.threshold && n_1k.max(n_1b) > 10 {
                    println!(
                        "{},{:.1},{},{},{},{:.1},fine",
                        bname, t_rel, n_1k, n_1b, delta, delta_pct
                    );
                }
            } else if delta_pct.abs() > args.threshold && n_1k.max(n_1b) > 10 {
                println!(
                    "  T+{:5.1} {:7} {:7} {:+6} {:+7.1}%  *** MISMATCH ***",
                    t_rel, n_1k, n_1b, delta, delta_pct
                );
                n_prob += 1;
            }
        }
        if !args.csv && n_prob == 0 {
            println!("  (none)");
        }

        // --- Cross-correlation ---
        if !args.csv {
            println!("Cross-correlation:");
            println!(
                "  {:>5} {:>10} {:>8} {:>6} {:>6}",
                "T+", "offset_ms", "corr", "1K_n", "1B_n"
            );
        }

        let cc_bin = 0.001; // 1ms bins
        let n_cc_per_sec = (args.coarse_bin / cc_bin).round() as usize;
        let n_coarse_cc = ((met_max - met_min) / args.coarse_bin).ceil() as usize;

        for sec in 0..n_coarse_cc {
            let t0 = met_min + sec as f64 * args.coarse_bin;
            let t1 = t0 + args.coarse_bin;

            // Build 1ms histograms
            let mut k_h = vec![0i64; n_cc_per_sec];
            let mut b_h = vec![0i64; n_cc_per_sec];

            for &t in &k1_times[k1_times.partition_point(|&x| x < t0)
                ..k1_times.partition_point(|&x| x < t1)]
            {
                let idx = ((t - t0) / cc_bin) as usize;
                if idx < n_cc_per_sec {
                    k_h[idx] += 1;
                }
            }
            for &t in &b1_times[b1_times.partition_point(|&x| x < t0)
                ..b1_times.partition_point(|&x| x < t1)]
            {
                let idx = ((t - t0) / cc_bin) as usize;
                if idx < n_cc_per_sec {
                    b_h[idx] += 1;
                }
            }

            let k_sum: i64 = k_h.iter().sum();
            let b_sum: i64 = b_h.iter().sum();
            if k_sum < 50 || b_sum < 50 {
                continue;
            }

            let n = k_h.len() as f64;
            let k_mean = k_sum as f64 / n;
            let b_mean = b_sum as f64 / n;
            let k_norm: Vec<f64> = k_h.iter().map(|&v| v as f64 - k_mean).collect();
            let b_norm: Vec<f64> = b_h.iter().map(|&v| v as f64 - b_mean).collect();

            let k_std = (k_norm.iter().map(|v| v * v).sum::<f64>() / n).sqrt();
            let b_std = (b_norm.iter().map(|v| v * v).sum::<f64>() / n).sqrt();

            if k_std < 1e-10 || b_std < 1e-10 {
                continue;
            }

            let max_lag = args.max_lag;
            let mut best_lag: i64 = 0;
            let mut best_corr: f64 = -1.0;
            let len = k_norm.len();

            for lag in -(max_lag as i64)..=(max_lag as i64) {
                let c = if lag >= 0 {
                    let l = lag as usize;
                    k_norm[l..]
                        .iter()
                        .zip(b_norm[..len - l].iter())
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                } else {
                    let l = (-lag) as usize;
                    k_norm[..len - l]
                        .iter()
                        .zip(b_norm[l..].iter())
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                };
                let c = c / (k_std * b_std * n);
                if c > best_corr {
                    best_corr = c;
                    best_lag = lag;
                }
            }

            let t_rel = t0 - met_min;
            if args.csv {
                println!(
                    "{},{:.0},{},{},{:.3},cc",
                    bname, t_rel, best_lag, best_corr, k_sum
                );
            } else if best_lag != 0 {
                let shifted = if best_lag.abs() > 5 {
                    "<-- SHIFTED"
                } else {
                    ""
                };
                println!(
                    "  T+{:3.0}  {:+10}  {:8.3} {:6} {:6}  {}",
                    t_rel, best_lag, best_corr, k_sum, b_sum, shifted
                );
            }
        }
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    match cli.command {
        TopCommands::Sat { epoch, box_filter, command } => {
            let epoch = parse_epoch(&epoch);
            eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
            let boxes = load_boxes(epoch);
            eprintln!(
                "  Found {} boxes: {:?}",
                boxes.len(),
                boxes.iter().map(|(n, _, _)| n.as_str()).collect::<Vec<_>>()
            );
            let filtered = filter_boxes(&boxes, &box_filter);

            match command {
                SatCommands::Solve => cmd_solve(&filtered),
                SatCommands::Detect => cmd_detect(&filtered),
                SatCommands::Reconstruct(args) => {
                    cmd_reconstruct(&args, &boxes, &box_filter)
                }
                SatCommands::Compare(args) => {
                    cmd_compare(&args, &boxes, epoch, &box_filter)
                }
                SatCommands::Dump { sub } => match sub {
                    DumpCommands::Times(w) => cmd_dump_times(&w, &boxes),
                    DumpCommands::Packets(w) => cmd_dump_packets(&w, &filtered, &boxes),
                    DumpCommands::Events(w) => cmd_dump_events(&w, &filtered),
                    DumpCommands::Hist(a) => cmd_dump_hist(&a, &filtered),
                    DumpCommands::Diag(w) => cmd_dump_diag(&w, &boxes),
                    DumpCommands::Ptime(r) => cmd_dump_ptime(&r, &filtered),
                    DumpCommands::CheckOffset(r) => cmd_dump_check_offset(&r, &filtered),
                },
            }
        }
        TopCommands::Search { from, to } => {
            eprintln!("TGF search from {} to {}...", from, to);
            // TODO: integrate blink_search::search_day for each day in range
            eprintln!("Not implemented yet. Use blink_search crate directly.");
        }
        TopCommands::Filter => {
            eprintln!("TGF filter...");
            // TODO: integrate blink_filter::run()
            eprintln!("Not implemented yet. Use blink_filter crate directly.");
        }
    }
}
