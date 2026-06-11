use blink_hxmt_he::algorithms::saturation::{
    check_byte_offsets, detect_fifo_reset_intervals, diagnose_packets,
    dump_event_details, dump_ptime_utc, extract_packet_infos, extract_second_event_times, solve_events,
    reconstruct_gaps, reconstruct_met_times,
    reconstruct_with_wrap_tracking,
    reconstruct_with_wrap_tracking_labeled,
    scan_saturation_intervals_raw, BoxReconstructionData, detect_unreliable_intervals,
};
use blink_hxmt_he::io::level_1b::{SciFile, get_eng_filenames, get_sci_filenames, read_stime_offset};
use blink_hxmt_he::io::level_1k::EventFile;
use chrono::prelude::*;
use clap::{Args, Parser, Subcommand};
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write as IoWrite};
use std::path::PathBuf;

// ── CLI structs ──────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(about = "HXMT HE analysis toolkit")]
struct Cli {
    #[command(subcommand)]
    command: TopCommands,
}

#[derive(Subcommand)]
enum TopCommands {
    /// Saturation analysis (detect FIFO resets, reconstruct gaps, generate reports)
    Sat {
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
    /// Full diagnostic data pack for one burst (events, resets, summary)
    Report(ReportArgs),
    /// Detect FIFO resets in a burst window
    Detect(BurstArgs),
    /// Gap-filled light curve (1B + cross-box reconstruction)
    Reconstruct(ReconstructArgs),
    /// Per-event dump from 1B (raw) or 1K pipeline
    Extract(ExtractArgs),
    /// Compare 1B vs 1K event data
    Compare(CompareArgs),
    /// Scan a 1B hour for FIFO resets (no trigger; for offline sweeps)
    Scan(ScanArgs),
    /// Low-level diagnostic dumps
    Dump {
        #[command(subcommand)]
        sub: DumpCommands,
    },
}

/// Shared positional + flags for burst-centric subcommands.
/// EPOCH is derived from TRIGGER (1B archive is per-hour partitioned).
#[derive(Args)]
struct BurstWindow {
    /// Trigger time (MET number or UTC datetime, e.g. 2020-04-15T08:48:05.560)
    trigger: String,
    /// Seconds before trigger
    #[arg(long)]
    before: f64,
    /// Seconds after trigger
    #[arg(long)]
    after: f64,
    /// Filter to a single box (a, b, or c). If omitted, all boxes.
    #[arg(long = "box")]
    box_filter: Option<String>,
}

impl BurstWindow {
    fn trigger_met(&self) -> f64 {
        parse_met_or_utc(&self.trigger)
    }
    fn met_min(&self) -> f64 {
        self.trigger_met() - self.before
    }
    fn met_max(&self) -> f64 {
        self.trigger_met() + self.after
    }
    fn epoch(&self) -> DateTime<Utc> {
        epoch_hour_of_met(self.trigger_met())
    }
}

#[derive(Args)]
struct BurstArgs {
    #[command(flatten)]
    window: BurstWindow,
}

#[derive(Args)]
struct ReportArgs {
    /// Trigger time (MET number or UTC datetime)
    trigger: String,
    /// Seconds before trigger
    #[arg(long)]
    before: f64,
    /// Seconds after trigger
    #[arg(long)]
    after: f64,
    /// Output directory for the data pack
    #[arg(long, short = 'o')]
    out: PathBuf,
}

#[derive(Args)]
struct ReconstructArgs {
    #[command(flatten)]
    window: BurstWindow,
    /// Bin width in seconds
    #[arg(long, default_value_t = 1.0)]
    bin: f64,
}

#[derive(Args)]
struct ExtractArgs {
    #[command(flatten)]
    window: BurstWindow,
    /// Source: 1b (raw with MET reconstruction) or 1k (pipeline)
    #[arg(long, default_value = "1b")]
    source: String,
}

#[derive(Args)]
struct CompareArgs {
    #[command(flatten)]
    window: BurstWindow,
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

#[derive(Args)]
struct ScanArgs {
    /// Epoch in YYYY-MM-DDTHH format
    #[arg(long)]
    epoch: String,
    /// Filter to a single box (a, b, or c). If omitted, all boxes.
    #[arg(long = "box")]
    box_filter: Option<String>,
}

#[derive(Subcommand)]
enum DumpCommands {
    /// Dump event MET times
    Times(DumpBurstArgs),
    /// Dump packet time ranges
    Packets(DumpBurstArgs),
    /// Dump event details
    Events(DumpBurstArgs),
    /// Histogram of events
    Hist(DumpHistArgs),
    /// Per-packet diagnostics
    Diag(DumpBurstArgs),
    /// Dump ptime/UTC mapping for a packet range
    Ptime(DumpRangeArgs),
    /// Check byte offsets for CRC for a packet range
    CheckOffset(DumpRangeArgs),
}

#[derive(Args)]
struct DumpBurstArgs {
    /// Epoch in YYYY-MM-DDTHH format
    #[arg(long)]
    epoch: String,
    /// Trigger time (MET number or UTC datetime)
    trigger: String,
    /// Seconds before trigger
    #[arg(long, default_value_t = 10.0)]
    before: f64,
    /// Seconds after trigger
    #[arg(long, default_value_t = 100.0)]
    after: f64,
    /// Filter to a single box (a, b, or c). If omitted, all boxes.
    #[arg(long = "box")]
    box_filter: Option<String>,
}

impl DumpBurstArgs {
    fn trigger_met(&self) -> f64 { parse_met_or_utc(&self.trigger) }
    fn met_min(&self) -> f64 { self.trigger_met() - self.before }
    fn met_max(&self) -> f64 { self.trigger_met() + self.after }
}

#[derive(Args)]
struct DumpHistArgs {
    #[command(flatten)]
    window: DumpBurstArgs,
    /// Bin width in seconds
    #[arg(long, default_value_t = 0.01)]
    bin: f64,
}

#[derive(Args)]
struct DumpRangeArgs {
    /// Epoch in YYYY-MM-DDTHH format
    #[arg(long)]
    epoch: String,
    /// Minimum packet index
    pkt_min: usize,
    /// Maximum packet index
    pkt_max: usize,
    /// Filter to a single box (a, b, or c). If omitted, all boxes.
    #[arg(long = "box")]
    box_filter: Option<String>,
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
    if let Ok(met) = s.parse::<f64>() {
        return met;
    }
    let ref_time = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let utc = s.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        format!("{}Z", s).parse::<DateTime<Utc>>()
            .or_else(|_| format!("{}:00Z", s).parse::<DateTime<Utc>>())
            .or_else(|_| format!("{}:00:00Z", s).parse::<DateTime<Utc>>())
            .expect("Invalid time format. Use MET number or UTC datetime (e.g. 2020-04-15T08:34:48)")
    });
    let met = (utc - ref_time).num_microseconds().unwrap() as f64 / 1e6;
    eprintln!("  UTC {} -> MET {:.6}", utc.format("%Y-%m-%dT%H:%M:%S"), met);
    met
}

/// Convert MET to its containing 1B-archive hour (floored to YYYY-MM-DDTHH:00:00 UTC).
fn epoch_hour_of_met(met: f64) -> DateTime<Utc> {
    let met_epoch = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let utc = met_epoch + chrono::Duration::microseconds((met * 1e6) as i64);
    utc.date_naive()
        .and_hms_opt(utc.hour(), 0, 0)
        .unwrap()
        .and_utc()
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

/// Warn if [met-before, met+after] crosses the hour boundary of `epoch`.
fn warn_if_window_crosses_hour(met: f64, before: f64, after: f64, epoch: DateTime<Utc>) {
    let met_epoch = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let epoch_start_met = (epoch - met_epoch).num_microseconds().unwrap() as f64 / 1e6;
    let epoch_end_met = epoch_start_met + 3600.0;
    if met - before < epoch_start_met || met + after > epoch_end_met {
        eprintln!(
            "warning: window [{:.1}, {:.1}] crosses hour boundary; only loading hour {} ({:.1}..{:.1})",
            met - before, met + after,
            epoch.format("%Y-%m-%dT%H"),
            epoch_start_met, epoch_end_met
        );
    }
}

// ── Command implementations ─────────────────────────────────────────────────

fn cmd_extract_1b(
    filtered_boxes: &[&(String, SciFile, f64)],
    met_min: f64,
    met_max: f64,
) {
    eprintln!("Extracting 1B events in [{:.3}, {:.3}]", met_min, met_max);
    println!("box,type,met,channel,det_id,pkt_idx,evt_idx,aminfo,pulinfo");
    for (box_name, sci, offset) in filtered_boxes {
        let events = solve_events(sci, *offset, Some(met_min), Some(met_max));
        let mut n_evt = 0u64;
        let mut n_sec = 0u64;
        let mut n_err = 0u64;
        let mut n_acd = 0u64;
        for evt in &events {
            let typ = if evt.is_error {
                "CRC"
            } else if evt.is_second {
                "SEC"
            } else {
                "EVT"
            };
            println!(
                "{},{},{:.6},{},{},{},{},{},{}",
                box_name, typ, evt.met, evt.channel, evt.det_id, evt.pkt_index, evt.evt_index,
                evt.aminfo, evt.pulinfo,
            );
            if evt.is_error { n_err += 1; }
            else if evt.is_second { n_sec += 1; }
            else {
                n_evt += 1;
                if evt.aminfo != 0 { n_acd += 1; }
            }
        }
        eprintln!("  Box {}: {} events ({} ACD-flagged, {:.2}%), {} seconds, {} CRC errors",
                  box_name, n_evt, n_acd,
                  if n_evt > 0 { 100.0 * n_acd as f64 / n_evt as f64 } else { 0.0 },
                  n_sec, n_err);
    }
}

fn cmd_extract_1k(
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

fn cmd_detect(
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

fn cmd_reconstruct(
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

fn cmd_dump_times(
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

fn cmd_dump_packets(
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

fn cmd_dump_events(
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

fn cmd_dump_hist(
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

fn cmd_dump_diag(
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

fn cmd_dump_ptime(
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

fn cmd_dump_check_offset(
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

fn cmd_compare(
    args: &CompareArgs,
    boxes: &[(String, SciFile, f64)],
    epoch: DateTime<Utc>,
    filter_box: &Option<String>,
) {
    let met_min = args.window.met_min();
    let met_max = args.window.met_max();

    eprintln!("Loading 1B times...");
    let mut b1: Vec<(String, Vec<f64>)> = Vec::new();
    for (box_name, sci, offset) in boxes {
        if let Some(fb) = filter_box {
            if !box_name.eq_ignore_ascii_case(fb) { continue; }
        }
        let mut times = reconstruct_met_times(sci, *offset);
        times.retain(|t| !t.is_nan());
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!("  1B Box {}: {} events", box_name, times.len());
        b1.push((box_name.clone(), times));
    }

    eprintln!("Loading 1K times...");
    let evt = EventFile::from_epoch(&epoch).expect("Failed to load 1K EventFile");
    let k1_times = evt.times();
    let k1_dets = evt.det_ids();

    let box_ranges: [(&str, u8, u8); 3] = [("A", 0, 5), ("B", 6, 11), ("C", 12, 17)];
    let mut k1: Vec<(String, Vec<f64>)> = Vec::new();
    for (bname, d_lo, d_hi) in &box_ranges {
        if let Some(fb) = filter_box {
            if !bname.eq_ignore_ascii_case(fb) { continue; }
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

    for (bname, k1_times) in &k1 {
        let b1_times = match b1.iter().find(|(n, _)| n == bname) {
            Some((_, t)) => t.as_slice(),
            None => continue,
        };

        if !args.csv {
            println!("\n--- Box {} ---", bname);
            println!("  {:>5} {:>7} {:>7} {:>7} {:>8}", "T+", "1K", "1B", "delta", "delta%");
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
                let note = if n_1k > 50 && n_1b == 0 { "*** HOLE ***" }
                    else if n_1b > 50 && n_1k == 0 { "*** EXTRA ***" }
                    else if delta_pct.abs() > 50.0 && n_1k > 20 { "*** MISMATCH ***" }
                    else if delta_pct.abs() > 20.0 && n_1k > 20 { "** mismatch **" }
                    else if delta_pct.abs() > 10.0 && n_1k > 50 { "* slight *" }
                    else { "" };
                println!(
                    "  T+{:3.0} {:7} {:7} {:+7} {:+8.1}%  {}",
                    t_rel, n_1k, n_1b, delta, delta_pct, note
                );
            }
        }

        if !args.csv {
            println!("Fine bins with |delta| > {:.0}%:", args.threshold);
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

            if n_1k < 3 && n_1b < 3 { continue; }

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

        if !args.csv {
            println!("Cross-correlation:");
            println!(
                "  {:>5} {:>10} {:>8} {:>6} {:>6}",
                "T+", "offset_ms", "corr", "1K_n", "1B_n"
            );
        }

        let cc_bin = 0.001;
        let n_cc_per_sec = (args.coarse_bin / cc_bin).round() as usize;
        let n_coarse_cc = ((met_max - met_min) / args.coarse_bin).ceil() as usize;

        for sec in 0..n_coarse_cc {
            let t0 = met_min + sec as f64 * args.coarse_bin;
            let t1 = t0 + args.coarse_bin;

            let mut k_h = vec![0i64; n_cc_per_sec];
            let mut b_h = vec![0i64; n_cc_per_sec];

            for &t in &k1_times[k1_times.partition_point(|&x| x < t0)
                ..k1_times.partition_point(|&x| x < t1)]
            {
                let idx = ((t - t0) / cc_bin) as usize;
                if idx < n_cc_per_sec { k_h[idx] += 1; }
            }
            for &t in &b1_times[b1_times.partition_point(|&x| x < t0)
                ..b1_times.partition_point(|&x| x < t1)]
            {
                let idx = ((t - t0) / cc_bin) as usize;
                if idx < n_cc_per_sec { b_h[idx] += 1; }
            }

            let k_sum: i64 = k_h.iter().sum();
            let b_sum: i64 = b_h.iter().sum();
            if k_sum < 50 || b_sum < 50 { continue; }

            let n = k_h.len() as f64;
            let k_mean = k_sum as f64 / n;
            let b_mean = b_sum as f64 / n;
            let k_norm: Vec<f64> = k_h.iter().map(|&v| v as f64 - k_mean).collect();
            let b_norm: Vec<f64> = b_h.iter().map(|&v| v as f64 - b_mean).collect();

            let k_std = (k_norm.iter().map(|v| v * v).sum::<f64>() / n).sqrt();
            let b_std = (b_norm.iter().map(|v| v * v).sum::<f64>() / n).sqrt();

            if k_std < 1e-10 || b_std < 1e-10 { continue; }

            let max_lag = args.max_lag;
            let mut best_lag: i64 = 0;
            let mut best_corr: f64 = -1.0;
            let len = k_norm.len();

            for lag in -(max_lag as i64)..=(max_lag as i64) {
                let c = if lag >= 0 {
                    let l = lag as usize;
                    k_norm[l..].iter().zip(b_norm[..len - l].iter())
                        .map(|(a, b)| a * b).sum::<f64>()
                } else {
                    let l = (-lag) as usize;
                    k_norm[..len - l].iter().zip(b_norm[l..].iter())
                        .map(|(a, b)| a * b).sum::<f64>()
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
                let shifted = if best_lag.abs() > 5 { "<-- SHIFTED" } else { "" };
                println!(
                    "  T+{:3.0}  {:+10}  {:8.3} {:6} {:6}  {}",
                    t_rel, best_lag, best_corr, k_sum, b_sum, shifted
                );
            }
        }
    }
}

// ── Report: full diagnostic data pack ────────────────────────────────────────

fn cmd_report(args: &ReportArgs) -> std::io::Result<()> {
    let trigger_met = parse_met_or_utc(&args.trigger);
    let epoch = epoch_hour_of_met(trigger_met);
    warn_if_window_crosses_hour(trigger_met, args.before, args.after, epoch);

    let met_epoch = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let trigger_utc = met_epoch + chrono::Duration::microseconds((trigger_met * 1e6) as i64);
    let met_min = trigger_met - args.before;
    let met_max = trigger_met + args.after;

    eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
    let boxes = load_boxes(epoch);
    eprintln!(
        "  Found {} boxes: {:?}",
        boxes.len(),
        boxes.iter().map(|(n, _, _)| n.as_str()).collect::<Vec<_>>()
    );

    create_dir_all(&args.out)?;

    // ── 1B path discovery (for manifest) ──
    let sci_paths = get_sci_filenames(epoch);
    let eng_paths = get_eng_filenames(epoch);

    // ── Run reconstruction for all boxes (needed for events_obs + events_rec) ──
    eprintln!("Preparing reconstruction data...");
    let mut box_data: Vec<(String, BoxReconstructionData)> = Vec::new();
    for (box_name, sci, offset) in &boxes {
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

    // ── Cross-box gap-fill ──
    eprintln!("Reconstructing (FIFO reset gaps)...");
    let mut filled_per_box: Vec<(String, Vec<f64>, usize, usize)> = Vec::new();
    // (box_name, gap_events, n_lost_total, n_resets_with_cross_ref)
    for i in 0..box_data.len() {
        let refs: Vec<&BoxReconstructionData> = box_data
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, (_, d))| d)
            .collect();
        let gap_results = reconstruct_gaps(&box_data[i].1, &refs);
        let n_lost_total: usize = gap_results.iter().map(|r| r.n_lost).sum();
        let n_ref = gap_results.iter().filter(|r| r.has_cross_ref).count();
        let mut gap_events: Vec<f64> = gap_results
            .into_iter()
            .flat_map(|r| r.filled_events)
            .collect();
        gap_events.sort_by(|a, b| a.partial_cmp(b).unwrap());
        filled_per_box.push((box_data[i].0.clone(), gap_events, n_lost_total, n_ref));
    }

    // ── 1K loading ──
    eprintln!("Loading 1K EventFile...");
    let evt_1k = EventFile::from_epoch(&epoch).expect("Failed to load 1K EventFile");
    let k1_times = evt_1k.times();
    let k1_dets = evt_1k.det_ids();
    let k1_channels = evt_1k.channels();
    let box_ranges: [(&str, u8, u8); 3] = [("A", 0, 5), ("B", 6, 11), ("C", 12, 17)];

    // ── 1B FITS path discovery for manifest ──
    let mut he_eng_files: Vec<(String, String)> = Vec::new();
    for (box_name, _, _) in &boxes {
        if let Some((_, path)) = eng_paths.iter().find(|(b, _)| b == box_name) {
            he_eng_files.push((box_name.clone(), path.clone()));
        }
    }

    // ── Write per-box CSVs + collect summary ──
    let mut summary: Vec<(String, u64, u64, u64, u64, u64)> = Vec::new();
    // (box, n_obs, n_rec, n_resets, n_lost, n_1k)

    for (i, (box_name, sci, offset)) in boxes.iter().enumerate() {
        let box_dir = args.out.join(format!("box_{}", box_name.to_lowercase()));
        create_dir_all(&box_dir)?;

        // events_obs.csv (observed 1B events in window, full detail incl. det_id/aminfo)
        let obs_path = box_dir.join("events_obs.csv");
        let mut w = BufWriter::new(File::create(&obs_path)?);
        writeln!(w, "met,channel,det_id,pkt_idx,evt_idx,aminfo,pulinfo,is_second,is_error")?;
        let detailed = solve_events(sci, *offset, Some(met_min), Some(met_max));
        let mut n_obs = 0u64;
        for e in &detailed {
            writeln!(
                w, "{:.6},{},{},{},{},{},{},{},{}",
                e.met, e.channel, e.det_id, e.pkt_index, e.evt_index,
                e.aminfo, e.pulinfo,
                if e.is_second { 1 } else { 0 },
                if e.is_error { 1 } else { 0 },
            )?;
            if !e.is_second && !e.is_error { n_obs += 1; }
        }
        w.flush()?;

        // events_rec.csv (gap-filled reconstructed events in window)
        let rec_path = box_dir.join("events_rec.csv");
        let mut w = BufWriter::new(File::create(&rec_path)?);
        writeln!(w, "met")?;
        let mut n_rec = 0u64;
        for &t in &filled_per_box[i].1 {
            if t >= met_min && t <= met_max {
                writeln!(w, "{:.6}", t)?;
                n_rec += 1;
            }
        }
        w.flush()?;

        // events_1k.csv (1K pipeline events for this box's det range, in window).
        // det_id is normalized to box-local (0..5) for consistency with events_obs.csv.
        let k1_path = box_dir.join("events_1k.csv");
        let mut w = BufWriter::new(File::create(&k1_path)?);
        writeln!(w, "met,channel,det_id")?;
        let (_, d_lo, d_hi) = box_ranges.iter().find(|(n, _, _)| n == box_name).unwrap();
        let mut n_1k = 0u64;
        for j in 0..k1_times.len() {
            let (t, d, ch) = (k1_times[j], k1_dets[j], k1_channels[j]);
            if d >= *d_lo && d <= *d_hi && t >= met_min && t <= met_max {
                writeln!(w, "{:.6},{},{}", t, ch, d - d_lo)?;
                n_1k += 1;
            }
        }
        w.flush()?;

        // resets.csv (FIFO resets in window, with cluster_id)
        let resets_path = box_dir.join("resets.csv");
        let mut w = BufWriter::new(File::create(&resets_path)?);
        writeln!(w, "start_met,stop_met,gap_s,prev_pkt_idx,next_pkt_idx,n_lost,cluster_id")?;
        let packets = &box_data[i].1.packets;
        let mut in_window: Vec<(f64, f64, f64, usize, usize, usize)> = Vec::new();
        for iv in &box_data[i].1.gaps {
            if iv.stop_met < met_min || iv.start_met > met_max { continue; }
            let r_true = packets.iter()
                .find(|p| p.pkt_idx == iv.next_pkt_idx)
                .map(|p| 109.0 / p.span().max(1e-9))
                .unwrap_or(15797.0);
            let n_lost = (r_true * iv.gap_seconds).round() as usize;
            in_window.push((iv.start_met, iv.stop_met, iv.gap_seconds,
                            iv.prev_pkt_idx, iv.next_pkt_idx, n_lost));
        }
        // Cluster: resets within 1.0 s of each other share cluster_id.
        in_window.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut cluster_id: usize = 0;
        let mut last_stop: f64 = f64::NEG_INFINITY;
        let mut n_lost_total = 0u64;
        for (s, e, g, p, nx, nl) in &in_window {
            if *s - last_stop > 1.0 { cluster_id += 1; }
            writeln!(w, "{:.6},{:.6},{:.6},{},{},{},{}",
                     s, e, g, p, nx, nl, cluster_id)?;
            last_stop = *e;
            n_lost_total += *nl as u64;
        }
        w.flush()?;
        let n_resets = in_window.len() as u64;

        eprintln!(
            "  Box {}: obs={} rec={} resets={} lost={} 1k={}",
            box_name, n_obs, n_rec, n_resets, n_lost_total, n_1k
        );
        summary.push((box_name.clone(), n_obs, n_rec, n_resets, n_lost_total, n_1k));
    }

    // ── manifest.json ──
    let manifest_path = args.out.join("manifest.json");
    let mut w = BufWriter::new(File::create(&manifest_path)?);
    writeln!(w, "{{")?;
    writeln!(w, "  \"trigger_raw\": \"{}\",", json_escape(&args.trigger))?;
    writeln!(w, "  \"trigger_met\": {:.6},", trigger_met)?;
    writeln!(w, "  \"trigger_utc\": \"{}\",", trigger_utc.format("%Y-%m-%dT%H:%M:%S%.3f"))?;
    writeln!(w, "  \"before_s\": {:.3},", args.before)?;
    writeln!(w, "  \"after_s\": {:.3},", args.after)?;
    writeln!(w, "  \"epoch\": \"{}\",", epoch.format("%Y-%m-%dT%H"))?;
    writeln!(w, "  \"met_min\": {:.6},", met_min)?;
    writeln!(w, "  \"met_max\": {:.6},", met_max)?;

    writeln!(w, "  \"boxes\": [{}],",
             boxes.iter().map(|(n, _, _)| format!("\"{}\"", n))
                 .collect::<Vec<_>>().join(", "))?;

    writeln!(w, "  \"level_1b_sci\": {{")?;
    for (i, (b, p)) in sci_paths.iter().enumerate() {
        let comma = if i + 1 < sci_paths.len() { "," } else { "" };
        writeln!(w, "    \"{}\": \"{}\"{}", b, json_escape(p), comma)?;
    }
    writeln!(w, "  }},")?;

    writeln!(w, "  \"level_1b_eng\": {{")?;
    for (i, (b, p)) in he_eng_files.iter().enumerate() {
        let comma = if i + 1 < he_eng_files.len() { "," } else { "" };
        writeln!(w, "    \"{}\": \"{}\"{}", b, json_escape(p), comma)?;
    }
    writeln!(w, "  }},")?;

    // 1K paths (HE-Evt + Orbit) — discovered via blink_core path resolver
    let evt_1k_path = blink_hxmt_he::io::path::get_path(&epoch, "Evt").ok();
    let orbit_1k_path = blink_hxmt_he::io::path::get_path(&epoch, "Orbit").ok();
    writeln!(w, "  \"level_1k_he_evt\": {},",
             evt_1k_path.as_deref().map(|p| format!("\"{}\"", json_escape(p)))
                 .unwrap_or_else(|| "null".to_string()))?;
    writeln!(w, "  \"level_1k_orbit\": {},",
             orbit_1k_path.as_deref().map(|p| format!("\"{}\"", json_escape(p)))
                 .unwrap_or_else(|| "null".to_string()))?;

    writeln!(w, "  \"summary\": {{")?;
    for (i, (b, n_obs, n_rec, n_resets, n_lost, n_1k)) in summary.iter().enumerate() {
        let comma = if i + 1 < summary.len() { "," } else { "" };
        writeln!(w,
            "    \"{}\": {{\"n_obs\": {}, \"n_rec\": {}, \"n_resets\": {}, \"n_lost\": {}, \"n_1k\": {}}}{}",
            b, n_obs, n_rec, n_resets, n_lost, n_1k, comma)?;
    }
    writeln!(w, "  }}")?;
    writeln!(w, "}}")?;
    w.flush()?;

    eprintln!("Wrote pack to {}", args.out.display());
    Ok(())
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    match cli.command {
        TopCommands::Sat { command } => match command {
            SatCommands::Report(args) => {
                cmd_report(&args).expect("report failed");
            }
            SatCommands::Detect(args) => {
                let epoch = args.window.epoch();
                let met = args.window.trigger_met();
                warn_if_window_crosses_hour(met, args.window.before, args.window.after, epoch);
                eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
                let boxes = load_boxes(epoch);
                let filtered = filter_boxes(&boxes, &args.window.box_filter);
                cmd_detect(&filtered, Some(args.window.met_min()), Some(args.window.met_max()));
            }
            SatCommands::Reconstruct(args) => {
                let epoch = args.window.epoch();
                let met = args.window.trigger_met();
                warn_if_window_crosses_hour(met, args.window.before, args.window.after, epoch);
                eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
                let boxes = load_boxes(epoch);
                let filter_box = args.window.box_filter.clone();
                cmd_reconstruct(&args, &boxes, &filter_box);
            }
            SatCommands::Extract(args) => {
                let epoch = args.window.epoch();
                let met = args.window.trigger_met();
                warn_if_window_crosses_hour(met, args.window.before, args.window.after, epoch);
                match args.source.as_str() {
                    "1b" => {
                        eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
                        let boxes = load_boxes(epoch);
                        let filtered = filter_boxes(&boxes, &args.window.box_filter);
                        cmd_extract_1b(&filtered, args.window.met_min(), args.window.met_max());
                    }
                    "1k" => {
                        cmd_extract_1k(epoch, &args.window.box_filter,
                                       args.window.met_min(), args.window.met_max());
                    }
                    other => {
                        eprintln!("error: --source must be '1b' or '1k', got '{}'", other);
                        std::process::exit(2);
                    }
                }
            }
            SatCommands::Compare(args) => {
                let epoch = args.window.epoch();
                let met = args.window.trigger_met();
                warn_if_window_crosses_hour(met, args.window.before, args.window.after, epoch);
                eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
                let boxes = load_boxes(epoch);
                let filter_box = args.window.box_filter.clone();
                cmd_compare(&args, &boxes, epoch, &filter_box);
            }
            SatCommands::Scan(args) => {
                let epoch = parse_epoch(&args.epoch);
                eprintln!("Loading 1B files for {}...", epoch.format("%Y-%m-%dT%H"));
                let boxes = load_boxes(epoch);
                let filtered = filter_boxes(&boxes, &args.box_filter);
                cmd_detect(&filtered, None, None);
            }
            SatCommands::Dump { sub } => match sub {
                DumpCommands::Times(a) => {
                    let epoch = parse_epoch(&a.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.box_filter);
                    cmd_dump_times(&a, &filtered);
                }
                DumpCommands::Packets(a) => {
                    let epoch = parse_epoch(&a.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.box_filter);
                    cmd_dump_packets(&a, &filtered, &boxes);
                }
                DumpCommands::Events(a) => {
                    let epoch = parse_epoch(&a.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.box_filter);
                    cmd_dump_events(&a, &filtered);
                }
                DumpCommands::Hist(a) => {
                    let epoch = parse_epoch(&a.window.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.window.box_filter);
                    cmd_dump_hist(&a, &filtered);
                }
                DumpCommands::Diag(a) => {
                    let epoch = parse_epoch(&a.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.box_filter);
                    cmd_dump_diag(&a, &filtered);
                }
                DumpCommands::Ptime(a) => {
                    let epoch = parse_epoch(&a.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.box_filter);
                    cmd_dump_ptime(&a.epoch, a.pkt_min, a.pkt_max, &filtered);
                }
                DumpCommands::CheckOffset(a) => {
                    let epoch = parse_epoch(&a.epoch);
                    let boxes = load_boxes(epoch);
                    let filtered = filter_boxes(&boxes, &a.box_filter);
                    cmd_dump_check_offset(&a.epoch, a.pkt_min, a.pkt_max, &filtered);
                }
            },
        },
        TopCommands::Search { from, to } => {
            eprintln!("TGF search from {} to {}...", from, to);
            eprintln!("Not implemented yet. Use blink_search crate directly.");
        }
        TopCommands::Filter => {
            eprintln!("TGF filter...");
            eprintln!("Not implemented yet. Use blink_filter crate directly.");
        }
    }
}
