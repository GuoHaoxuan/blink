use blink_hxmt_he::algorithms::saturation::reconstruct_met_times;
use blink_hxmt_he::io::level_1b::SciFile;
use blink_hxmt_he::io::level_1k::EventFile;
use chrono::prelude::*;

use crate::cli::CompareArgs;

pub fn cmd_compare(
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
