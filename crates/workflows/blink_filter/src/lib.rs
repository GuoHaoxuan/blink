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
fn associate(signal: &UnifiedSignal) -> Tgf {
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
        signal: signal.clone(),
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

    // 每候选做 2 次 WWLLN 查询（±1s 关联 + ±62s 虚警概率），成本随该时段闪电
    // 密度差几十倍（活跃季 ±62s 窗返回上万条闪电）。静态分块会严重失衡（空段线程
    // 早退、忙段线程拖尾），故用原子取号做工作窃取：每线程反复领下一个待处理下标，
    // 忙闲自动均衡，56 核吃满到最后。结果带原下标收回后排序，保持原顺序。
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let signals_ref = &signals;

    let mut collected: Vec<(usize, Tgf)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..n_threads)
            .map(|_| {
                let next = &next;
                let done = &done;
                scope.spawn(move || {
                    let mut local: Vec<(usize, Tgf)> = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= total {
                            break;
                        }
                        local.push((i, associate(&signals_ref[i])));
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if n % 100_000 == 0 {
                            eprintln!("filter: {n}/{total}");
                        }
                    }
                    local
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });

    collected.sort_by_key(|(i, _)| *i);
    let tgfs: Vec<Tgf> = collected.into_iter().map(|(_, tgf)| tgf).collect();

    eprintln!("filter: {total}/{total} associated, writing tgfs.json");
    let json = serde_json::to_string_pretty(&tgfs).expect("failed to serialize to json");
    // 原子写：先写临时文件再 rename，避免下游（pipeline 的 cp / git）读到半截 json。
    let tmp = format!("tgfs.json.{}.tmp", nanoid::nanoid!(6));
    std::fs::write(&tmp, json).expect("failed to write tgfs.json tmp");
    std::fs::rename(&tmp, "tgfs.json").expect("failed to rename tgfs.json");
}
