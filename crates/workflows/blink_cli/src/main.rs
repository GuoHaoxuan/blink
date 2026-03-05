use blink_core::types::MissionElapsedTime;
use blink_hxmt_he::algorithms::saturation::{
    diagnose_packets, dump_event_details, reconstruct_met_times, reconstruct_with_wrap_tracking,
    scan_saturation_intervals,
};
use blink_hxmt_he::io::level_1b::{
    SciFile, get_eng_filenames, get_sci_filenames, read_stime_offset,
};
use blink_hxmt_he::types::HxmtHe;
use chrono::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 用法: blink_cli <YYYY-MM-DDTHH> [--dump-times <center_met> <half_window>]
    let epoch_str = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("2020-04-15T08:00:00Z");

    let dump_times = args.iter().position(|s| s == "--dump-times");
    let dump_packets = args.iter().position(|s| s == "--dump-packets");
    let dump_hist = args.iter().position(|s| s == "--dump-hist");
    let dump_diag = args.iter().position(|s| s == "--dump-diag");
    let dump_events = args.iter().position(|s| s == "--dump-events");
    let box_filter = args.iter().position(|s| s == "--box");
    let filter_box = box_filter.and_then(|pos| args.get(pos + 1).cloned());

    let epoch = epoch_str.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        format!("{}:00:00Z", epoch_str)
            .parse::<DateTime<Utc>>()
            .expect("Invalid datetime format. Use YYYY-MM-DDTHH or full ISO 8601.")
    });

    eprintln!("Loading files for {}...", epoch.format("%Y-%m-%dT%H"));

    // 加载文件：返回 Vec<(box_name, path)>，支持部分 box 缺失
    let sci_pairs = get_sci_filenames(epoch);
    let eng_pairs = get_eng_filenames(epoch);

    // 构建 (box_name, SciFile, offset) 三元组
    let boxes: Vec<(String, SciFile, f64)> = sci_pairs
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
        .collect();

    eprintln!(
        "  Found {} boxes: {:?}",
        boxes.len(),
        boxes.iter().map(|(n, _, _)| n.as_str()).collect::<Vec<_>>()
    );

    // Filter boxes if specified
    let filtered_boxes: Vec<_> = if let Some(ref fb) = filter_box {
        boxes.iter().filter(|(name, _, _)| name == fb).collect()
    } else {
        boxes.iter().collect()
    };

    if let Some(pos) = dump_diag {
        // --dump-diag mode: 输出 per-packet 诊断信息
        // 用法: --dump-diag <center_met> <half_window>
        let center_met: f64 = args
            .get(pos + 1)
            .expect("Missing center_met")
            .parse()
            .expect("center_met must be a float");
        let half_window: f64 = args
            .get(pos + 2)
            .map(|s| s.parse().unwrap_or(60.0))
            .unwrap_or(60.0);
        let met_min = center_met - half_window;
        let met_max = center_met + half_window;

        println!(
            "box,pkt,n_evt,n_sec,n_sec_valid,n_err,n_out,n_drop,anchor,utc_tail,met_min,met_max"
        );

        for (box_name, sci, offset) in &boxes {
            let diags = diagnose_packets(sci, *offset);
            let mut total_out = 0u64;
            let mut total_drop = 0u64;
            let mut total_err = 0u64;
            let mut total_sec = 0u64;
            let mut total_sec_valid = 0u64;
            let mut pkts_in_window = 0u64;
            let mut pkts_no_anchor = 0u64;

            for d in &diags {
                // 只输出覆盖目标窗口的包
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
                // 只输出有问题的包（CRC错误>0 或有丢弃）
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
                        d.met_min.map_or("-".to_string(), |v| format!("{:.3}", v)),
                        d.met_max.map_or("-".to_string(), |v| format!("{:.3}", v)),
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
    } else if let Some(pos) = dump_hist {
        // --dump-hist mode: 在 Rust 端做 histogram，输出紧凑 CSV
        // 用法: --dump-hist <center_met> <half_window> [bin_width]
        let center_met: f64 = args
            .get(pos + 1)
            .expect("Missing center_met after --dump-hist")
            .parse()
            .expect("center_met must be a float");
        let half_window: f64 = args
            .get(pos + 2)
            .map(|s| s.parse().unwrap_or(60.0))
            .unwrap_or(60.0);
        let bin_width: f64 = args
            .get(pos + 3)
            .map(|s| s.parse().unwrap_or(0.01))
            .unwrap_or(0.01);

        let met_min = center_met - half_window;
        let met_max = center_met + half_window;
        let n_bins = ((met_max - met_min) / bin_width).ceil() as usize;

        eprintln!(
            "Histogram: [{:.3}, {:.3}], bin_width={:.4}s, n_bins={}",
            met_min, met_max, bin_width, n_bins
        );

        let mut hist = vec![0u64; n_bins];
        let mut n_total = 0u64;

        for (box_name, sci, offset) in &filtered_boxes {
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

        // 输出 header
        println!("# center_met={:.6}", center_met);
        println!("# half_window={:.1}", half_window);
        println!("# bin_width={:.6}", bin_width);
        println!("# n_bins={}", n_bins);
        println!("# n_total={}", n_total);

        // 输出 histogram
        println!("# HIST");
        for (i, count) in hist.iter().enumerate() {
            let bin_start = met_min + i as f64 * bin_width;
            println!("{:.6},{}", bin_start, count);
        }

        // 输出饱和区间
        println!("# SAT");
        for (box_name, sci, offset) in &filtered_boxes {
            let intervals =
                blink_hxmt_he::algorithms::saturation::scan_saturation_intervals_raw(sci, *offset);
            for (start, stop) in &intervals {
                if *stop >= met_min && *start <= met_max {
                    println!("SAT,{},{:.6},{:.6}", box_name, start, stop);
                }
            }
        }
    } else if let Some(pos) = dump_events {
        // --dump-events mode: 输出详细事例信息
        let center_met: f64 = args
            .get(pos + 1)
            .expect("Missing center_met after --dump-events")
            .parse()
            .expect("center_met must be a float");
        let half_window: f64 = args
            .get(pos + 2)
            .map(|s| s.parse().unwrap_or(1.0))
            .unwrap_or(1.0);

        let met_min = center_met - half_window;
        let met_max = center_met + half_window;

        eprintln!("Dumping events in [{:.3}, {:.3}]", met_min, met_max);

        println!("# pkt,evt,is_second,ptime,channel,MET,r0,r1,r2,r3,r4,r5,r6,r7");
        for (box_name, sci, offset) in &filtered_boxes {
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
    } else if let Some(pos) = dump_times {
        // --dump-times mode: 输出所有事例 MET 和饱和区间
        let center_met: f64 = args
            .get(pos + 1)
            .expect("Missing center_met after --dump-times")
            .parse()
            .expect("center_met must be a float");
        let half_window: f64 = args
            .get(pos + 2)
            .map(|s| s.parse().unwrap_or(20.0))
            .unwrap_or(20.0);

        let met_min = center_met - half_window;
        let met_max = center_met + half_window;

        eprintln!(
            "Dumping times in [{:.3}, {:.3}] (center={:.3}, half_window={:.1})",
            met_min, met_max, center_met, half_window
        );

        // 输出 header
        println!("# center_met={:.6}", center_met);
        println!("# half_window={:.1}", half_window);

        // 输出事例 MET（每Box一节）
        for (box_name, sci, offset) in &boxes {
            eprintln!("Box {} (offset={:.0}) ...", box_name, offset);

            let all_met = reconstruct_met_times(sci, *offset);
            let n_total = all_met.len();

            // 过滤到窗口内
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

        // 输出饱和区间
        println!("# saturation_intervals");
        for (box_name, sci, offset) in &boxes {
            let intervals =
                blink_hxmt_he::algorithms::saturation::scan_saturation_intervals_raw(sci, *offset);
            for (start, stop) in &intervals {
                if *stop >= met_min && *start <= met_max {
                    println!("SAT,{},{:.6},{:.6}", box_name, start, stop);
                }
            }
        }
    } else if let Some(pos) = dump_packets {
        // --dump-packets mode: 输出每个 CCSDS 包的时间范围
        let center_met: f64 = args
            .get(pos + 1)
            .expect("Missing center_met after --dump-packets")
            .parse()
            .expect("center_met must be a float");
        let half_window: f64 = args
            .get(pos + 2)
            .map(|s| s.parse().unwrap_or(2.0))
            .unwrap_or(2.0);

        let met_min = center_met - half_window;
        let met_max = center_met + half_window;

        println!("box,pkt_idx,min_time,max_time,n_events");
        for (box_name, sci, offset) in &boxes {
            let packet_times = reconstruct_with_wrap_tracking(sci, *offset);
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
                // 只输出窗口内的包
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
        // 输出秒事例时间
        println!("# second_events");
        for (box_name, sci, offset) in &boxes {
            let sec_times =
                blink_hxmt_he::algorithms::saturation::extract_second_event_times(sci, *offset);
            for t in &sec_times {
                if *t >= met_min && *t <= met_max {
                    println!("SEC,{},{:.6}", box_name, t);
                }
            }
        }
    } else {
        // 默认模式: 只输出饱和区间
        let mut all_intervals: Vec<(MissionElapsedTime<HxmtHe>, MissionElapsedTime<HxmtHe>)> =
            Vec::new();
        for (box_name, sci, offset) in &boxes {
            eprintln!("Box {} (offset={:.0})", box_name, offset);
            let intervals = scan_saturation_intervals(sci, *offset);
            eprintln!("  {} intervals", intervals.len());
            all_intervals.extend(intervals);
        }

        all_intervals.sort_by(|a, b| a.0.cmp(&b.0));

        let mut merged: Vec<(MissionElapsedTime<HxmtHe>, MissionElapsedTime<HxmtHe>)> = Vec::new();
        for interval in all_intervals {
            if let Some(last) = merged.last_mut()
                && interval.0 <= last.1 {
                    if interval.1 > last.1 {
                        last.1 = interval.1;
                    }
                    continue;
                }
            merged.push(interval);
        }

        eprintln!("Total merged: {} intervals", merged.len());

        let ref_time = "2012-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        println!("start_met,stop_met");
        for (start, stop) in &merged {
            let start_utc: DateTime<Utc> = (*start).into();
            let stop_utc: DateTime<Utc> = (*stop).into();
            let start_sec = (start_utc - ref_time).num_microseconds().unwrap() as f64 / 1e6;
            let stop_sec = (stop_utc - ref_time).num_microseconds().unwrap() as f64 / 1e6;
            println!("{:.6},{:.6}", start_sec, stop_sec);
        }
    }
}
