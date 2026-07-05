use blink_core::types::{TemporalState, UnifiedSignal};
use blink_hxmt_he::types::HxmtHe;
use blink_lightning::{algorithms::coincidence_prob, database::get_lightnings};
use blink_load::load_all;
// use blink_svom_grm::types::SvomGrm;
use chrono::TimeDelta;
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use uom::si::f64::*;

#[derive(Serialize)]
struct LightningInfo {
    associated: bool,
    coincidence_probability: f64,
}

#[derive(Serialize)]
struct Tgf {
    signal: UnifiedSignal,
    lightning: LightningInfo,
}

/// 对单个候选做 WWLLN 闪电关联 + 虚警概率。每次调用的两个 `get_lightnings`
/// 查询走线程本地只读连接（见 blink_lightning::database），可安全并行。
fn associate(signal: UnifiedSignal) -> Tgf {
    let peak_time = signal.peak_time();
    let position = TemporalState {
        timestamp: peak_time,
        state: signal.position.clone(),
    };
    let lightnings = get_lightnings(
        peak_time - TimeDelta::seconds(1),
        peak_time + TimeDelta::seconds(1),
    )
    .into_iter()
    .filter(|lightning| {
        lightning.is_associated(
            &position,
            TimeDelta::milliseconds(5),
            Length::new::<uom::si::length::kilometer>(800.0),
        )
    })
    .collect::<Vec<_>>();

    Tgf {
        signal,
        lightning: LightningInfo {
            associated: !lightnings.is_empty(),
            coincidence_probability: coincidence_prob(
                &position,
                TimeDelta::milliseconds(5),
                Length::new::<uom::si::length::kilometer>(800.0),
                TimeDelta::minutes(2),
            ),
        },
    }
}

pub fn run() {
    let signals = load_all::<HxmtHe>();
    let total = signals.len();
    eprintln!("filter: {total} candidates to associate");

    // 每候选做 2 次 WWLLN 查询（±1s 关联 + ±62s 虚警概率），单线程要数小时。
    // 用作用域线程按块并行；每线程独立只读连接（thread_local），无锁竞争。
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    let chunk_size = total.div_ceil(n_threads.max(1)).max(1);

    // 切成 owned 块，避免逐候选 clone；块按顺序 flat_map 回来，保持原顺序。
    let mut chunks: Vec<Vec<UnifiedSignal>> = Vec::with_capacity(n_threads);
    let mut iter = signals.into_iter();
    loop {
        let chunk: Vec<UnifiedSignal> = iter.by_ref().take(chunk_size).collect();
        if chunk.is_empty() {
            break;
        }
        chunks.push(chunk);
    }

    let counter = AtomicUsize::new(0);
    let tgfs: Vec<Tgf> = std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let counter = &counter;
                scope.spawn(move || {
                    chunk
                        .into_iter()
                        .map(|signal| {
                            let tgf = associate(signal);
                            let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
                            if n % 100_000 == 0 {
                                eprintln!("filter: {n}/{total}");
                            }
                            tgf
                        })
                        .collect::<Vec<Tgf>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });

    eprintln!("filter: {total}/{total} associated, writing tgfs.json");
    let json = serde_json::to_string_pretty(&tgfs).expect("failed to serialize to json");
    // 原子写：先写临时文件再 rename，避免下游（pipeline 的 cp / git）读到半截 json。
    let tmp = format!("tgfs.json.{}.tmp", nanoid::nanoid!(6));
    std::fs::write(&tmp, json).expect("failed to write tgfs.json tmp");
    std::fs::rename(&tmp, "tgfs.json").expect("failed to rename tgfs.json");
}
