use blink_core::types::MissionElapsedTime;
use blink_hxmt_he::algorithms::saturation::{
    check_byte_offsets, detect_fifo_reset_intervals, detect_silent_drops, diagnose_packets,
    dump_event_details, dump_ptime_utc, extract_packet_infos, reconstruct_deep_saturation,
    reconstruct_gaps, reconstruct_met_times, reconstruct_silent_drops,
    reconstruct_with_wrap_tracking, reconstruct_with_wrap_tracking_labeled,
    scan_saturation_intervals, BoxReconstructionData, detect_unreliable_intervals,
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
    let dump_ptime = args.iter().position(|s| s == "--dump-ptime");
    let detect_sat = args.iter().position(|s| s == "--detect-saturation");
    let reconstruct = args.iter().position(|s| s == "--reconstruct");
    let check_offset = args.iter().position(|s| s == "--check-offset");
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

    if let Some(_pos) = detect_sat {
        println!("box,type,start_met,stop_met,gap_s,pkt_idx,evt_idx,n_lost,log10p");
        for (box_name, sci, offset) in &filtered_boxes {
            // 构建完整的 box 数据（同时用于两种检测）
            let events = reconstruct_met_times(sci, *offset);
            let gaps = detect_fifo_reset_intervals(sci, *offset);
            let packets = extract_packet_infos(sci, *offset);
            let packet_events: Vec<Vec<f64>> = reconstruct_with_wrap_tracking(sci, *offset)
                .into_iter()
                .map(|mut t| { t.sort_by(|a, b| a.partial_cmp(b).unwrap()); t })
                .collect();

            // FIFO reset 输出
            eprintln!("Box {}: {} FIFO reset intervals", box_name, gaps.len());
            for iv in &gaps {
                let r_true = packets
                    .iter()
                    .find(|p| p.pkt_idx == iv.next_pkt_idx)
                    .map(|p| 109.0 / p.span().max(1e-9))
                    .unwrap_or(15797.0);
                let n_lost = (r_true * iv.gap_seconds).round() as usize;
                println!(
                    "{},FifoReset,{:.6},{:.6},{:.6},{},{},{},",
                    box_name, iv.start_met, iv.stop_met, iv.gap_seconds,
                    iv.prev_pkt_idx, iv.next_pkt_idx, n_lost,
                );
            }

            // 静默丢数检测 + 输出
            let unreliable = detect_unreliable_intervals(&gaps, &packets, &packet_events);
            let box_data = BoxReconstructionData { events, gaps, packets, packet_events, unreliable };
            let drops = detect_silent_drops(&box_data);
            eprintln!("Box {}: {} silent drops", box_name, drops.len());
            for d in &drops {
                println!(
                    "{},SilentDrop,{:.6},{:.6},{:.6},{},{},{},{:.1}",
                    box_name, d.start_met, d.stop_met, d.dt,
                    d.pkt_idx, d.evt_idx, d.n_lost, d.log10_p,
                );
            }
        }
    } else if let Some(pos) = reconstruct {
        // --reconstruct <center_met> <half_window> [bin_width]
        // 输出: box,bin_center,observed,reconstructed,filled
        let center_met: f64 = args
            .get(pos + 1)
            .expect("Missing center_met after --reconstruct")
            .parse()
            .expect("center_met must be a float");
        let half_window: f64 = args
            .get(pos + 2)
            .expect("Missing half_window after --reconstruct")
            .parse()
            .expect("half_window must be a float");
        let bin_width: f64 = args
            .get(pos + 3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);

        let met_min = center_met - half_window;
        let met_max = center_met + half_window;

        // 步骤一：为所有 box 准备重建数据
        eprintln!("Preparing reconstruction data...");
        let mut box_data: Vec<(String, BoxReconstructionData)> = Vec::new();
        for (box_name, sci, offset) in &boxes {
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
                box_name, events.len(), gaps.len(), unreliable.len(), packets.len()
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

        // 保存原始事件数（补静默丢数前）
        let original_events: Vec<(String, Vec<f64>)> = box_data
            .iter()
            .map(|(name, data)| (name.clone(), data.events.clone()))
            .collect();

        // 步骤二：两种补全独立进行，都只用原始事件做交叉参考
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

            // 静默丢数重建（用原始事件做参考）
            let drops = detect_silent_drops(&box_data[i].1);
            let sd_results = reconstruct_silent_drops(&box_data[i].1, &drops, &refs);
            let n_sd_filled: usize = sd_results.iter().map(|r| r.n_lost).sum();
            let n_sd_ref = sd_results.iter().filter(|r| r.has_cross_ref).count();
            let mut sd_events: Vec<f64> = sd_results
                .into_iter()
                .flat_map(|r| r.filled_events)
                .collect();
            sd_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // FIFO reset 重建（用原始事件做参考，不含静默丢数补全）
            let gap_results = reconstruct_gaps(&box_data[i].1, &refs);
            let n_gap_filled: usize = gap_results.iter().map(|r| r.n_lost).sum();
            let n_gap_ref = gap_results.iter().filter(|r| r.has_cross_ref).count();
            let mut gap_events: Vec<f64> = gap_results
                .into_iter()
                .flat_map(|r| r.filled_events)
                .collect();
            gap_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // 深度饱和包级修正（用 burst 计数率填充 gap）
            let ds_results = reconstruct_deep_saturation(&box_data[i].1);
            let n_ds_count = ds_results.len();
            let n_ds_filled: usize = ds_results.iter().map(|r| r.n_lost).sum();
            let mut ds_events: Vec<f64> = ds_results
                .into_iter()
                .flat_map(|r| r.filled_events)
                .collect();
            ds_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // 合并静默丢数 + 深度饱和到 sd
            sd_events.extend_from_slice(&ds_events);
            sd_events.sort_by(|a, b| a.partial_cmp(b).unwrap());

            eprintln!(
                "  Box {}: sd={} ({} evt) | deep_sat={} ({} evt) | gaps={} ({} evt, {} ref)",
                box_data[i].0,
                drops.len(), n_sd_filled,
                n_ds_count, n_ds_filled,
                box_data[i].1.gaps.len(), n_gap_filled, n_gap_ref,
            );

            all_sd_filled.push((box_data[i].0.clone(), sd_events));
            all_filled.push((box_data[i].0.clone(), gap_events));
        }

        // 步骤四：输出 binned 光变曲线（分列输出两种补全）
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

            if let Some(ref fb) = filter_box {
                if box_name != fb {
                    continue;
                }
            }

            for w in bins.windows(2) {
                let bin_lo = w[0];
                let bin_hi = w[1];
                let bin_center = (bin_lo + bin_hi) / 2.0;

                let count = |events: &[f64]| -> usize {
                    events.partition_point(|&t| t < bin_hi)
                        - events.partition_point(|&t| t < bin_lo)
                };

                let n_obs = count(obs_events);
                let n_gap = count(gap_events);
                let n_sd = count(sd_events);
                let n_total = n_obs + n_gap + n_sd;

                println!(
                    "{},{:.6},{:.1},{:.1},{:.1},{:.1}",
                    box_name, bin_center,
                    n_obs as f64 / bin_width,
                    n_total as f64 / bin_width,
                    n_gap as f64 / bin_width,
                    n_sd as f64 / bin_width,
                );
            }
        }
    } else if let Some(pos) = dump_ptime {
        let pkt_min: usize = args
            .get(pos + 1)
            .expect("Missing pkt_min")
            .parse()
            .expect("pkt_min must be integer");
        let pkt_max: usize = args
            .get(pos + 2)
            .expect("Missing pkt_max")
            .parse()
            .expect("pkt_max must be integer");

        for (box_name, sci, offset) in &filtered_boxes {
            eprintln!("Box {} pkt {}..{}", box_name, pkt_min, pkt_max);
            dump_ptime_utc(sci, *offset, pkt_min, pkt_max);
        }
    } else if let Some(pos) = dump_diag {
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
        for (box_name, sci, offset) in &filtered_boxes {
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
    } else if let Some(pos) = check_offset {
        // --check-offset mode: 对每个包尝试 0~7 字节偏移，检查 CRC 通过率
        let pkt_min: usize = args
            .get(pos + 1)
            .expect("Missing pkt_min after --check-offset")
            .parse()
            .expect("pkt_min must be an integer");
        let pkt_max: usize = args
            .get(pos + 2)
            .expect("Missing pkt_max after --check-offset")
            .parse()
            .expect("pkt_max must be an integer");

        for (box_name, sci, _offset) in &filtered_boxes {
            eprintln!("Box {} checking offsets for packets {}..{}", box_name, pkt_min, pkt_max);
            check_byte_offsets(sci, pkt_min, pkt_max);
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
