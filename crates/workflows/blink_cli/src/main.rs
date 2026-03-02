use blink_core::types::MissionElapsedTime;
use blink_hxmt_he::algorithms::saturation::{
    reconstruct_met_times, reconstruct_with_wrap_tracking, scan_saturation_intervals,
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

    if let Some(pos) = dump_times {
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
            if let Some(last) = merged.last_mut() {
                if interval.0 <= last.1 {
                    if interval.1 > last.1 {
                        last.1 = interval.1;
                    }
                    continue;
                }
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
