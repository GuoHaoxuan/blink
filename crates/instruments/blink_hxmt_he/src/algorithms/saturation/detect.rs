use super::rec_sci_data::reconstruct_with_wrap_tracking;
use crate::io::level_1b::SciFile;

/// 饱和类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaturationType {
    /// 整包丢失：FIFOAFullReset 触发，整个 FIFO A 被清空。
    /// 表现为重建时间序列中出现远大于正常包间隔的空洞。
    FifoReset,
}

/// 单个饱和区间
#[derive(Debug, Clone)]
pub struct SaturationInterval {
    /// 空洞起始 MET（前一个包的最晚事件时间）
    pub start_met: f64,
    /// 空洞结束 MET（后一个包的最早事件时间）
    pub stop_met: f64,
    /// 空洞持续时间（秒）
    pub gap_seconds: f64,
    /// 空洞前一个包的索引（排序后）
    pub prev_pkt_idx: usize,
    /// 空洞后一个包的索引（排序后）
    pub next_pkt_idx: usize,
    /// 饱和类型
    pub saturation_type: SaturationType,
}

/// 每个 CCSDS 包的时间摘要
struct PacketTimeSummary {
    /// 原始包索引（在 SciFile.ccsds 中的位置）
    pkt_idx: usize,
    /// 包内最早事件的重建 MET
    min_met: f64,
    /// 包内最晚事件的重建 MET
    max_met: f64,
    /// 包内有效事件数
    n_events: usize,
}

const GAP_FACTOR: f64 = 100.0;

/// MCU 读取速率下限 (events/s)。
/// MCU 以固定速率从 FIFO A 读取：109 events / 6.9ms ≈ 15797 evt/s。
/// 只有当物理事件率超过此值时，FIFO 才可能溢出触发 FIFOAFullReset。
/// 设为 15000 略低于理论值，留一点余量。
const MCU_READ_RATE_FLOOR: f64 = 15000.0;

/// 从单个包的时间跨度和事件数估算平均事件间隔 (秒/事件)。
/// 如果包内时间跨度过小（<1μs）或事件数不足，返回 None。
fn mean_event_interval(summary: &PacketTimeSummary) -> Option<f64> {
    let span = summary.max_met - summary.min_met;
    if span < 1e-6 || summary.n_events < 2 {
        return None;
    }
    Some(span / summary.n_events as f64)
}

/// 从单个包估算事件率 (events/s)。
/// 如果包内时间跨度过小（<1μs）或事件数不足，返回 None。
fn event_rate(summary: &PacketTimeSummary) -> Option<f64> {
    let span = summary.max_met - summary.min_met;
    if span < 1e-6 || summary.n_events < 2 {
        return None;
    }
    Some(summary.n_events as f64 / span)
}

/// 检测整包丢失（FIFO reset）造成的饱和区间。
///
/// 算法：
/// 1. 对每个 CCSDS 包重建所有事件的 MET 时间，提取 (min_met, max_met, n_events)
/// 2. 按 min_met 排序
/// 3. 对每对相邻包，取前后包中平均事件间隔较小的那个（即事件率较高的包）
/// 4. 若前后包的最大事件率 < MCU_READ_RATE_FLOOR → 跳过（FIFO 不可能溢出）
/// 5. 实际 gap > 该间隔 × GAP_FACTOR → 标记为 FifoReset
pub fn detect_fifo_reset_intervals(sci_data: &SciFile, offset: f64) -> Vec<SaturationInterval> {
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);

    let mut summaries: Vec<PacketTimeSummary> = Vec::new();
    for (pkt_idx, times) in packet_times.iter().enumerate() {
        if times.is_empty() {
            continue;
        }
        let min_met = times.iter().cloned().reduce(f64::min).unwrap();
        let max_met = times.iter().cloned().reduce(f64::max).unwrap();
        summaries.push(PacketTimeSummary {
            pkt_idx,
            min_met,
            max_met,
            n_events: times.len(),
        });
    }

    summaries.sort_by(|a, b| a.min_met.partial_cmp(&b.min_met).unwrap());

    let mut intervals = Vec::new();
    for window in summaries.windows(2) {
        let gap = window[1].min_met - window[0].max_met;
        if gap <= 0.0 {
            continue;
        }

        let iv_prev = mean_event_interval(&window[0]);
        let iv_next = mean_event_interval(&window[1]);
        let baseline = match (iv_prev, iv_next) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => continue,
        };

        // 事件率低于 MCU 读取速率 → FIFO 不可能溢出，跳过
        let rate_prev = event_rate(&window[0]);
        let rate_next = event_rate(&window[1]);
        let max_rate = match (rate_prev, rate_next) {
            (Some(a), Some(b)) => a.max(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => continue,
        };
        if max_rate < MCU_READ_RATE_FLOOR {
            continue;
        }

        if gap > baseline * GAP_FACTOR {
            intervals.push(SaturationInterval {
                start_met: window[0].max_met,
                stop_met: window[1].min_met,
                gap_seconds: gap,
                prev_pkt_idx: window[0].pkt_idx,
                next_pkt_idx: window[1].pkt_idx,
                saturation_type: SaturationType::FifoReset,
            });
        }
    }

    intervals
}

/// 每个包的时间摘要（公开版本，用于重建）
#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub pkt_idx: usize,
    pub min_met: f64,
    pub max_met: f64,
    pub n_events: usize,
}

impl PacketInfo {
    pub fn span(&self) -> f64 {
        self.max_met - self.min_met
    }
}

/// 从 SciFile 提取包时间摘要列表（按 min_met 排序）
pub fn extract_packet_infos(sci_data: &SciFile, offset: f64) -> Vec<PacketInfo> {
    let packet_times = reconstruct_with_wrap_tracking(sci_data, offset);
    let mut infos: Vec<PacketInfo> = packet_times
        .iter()
        .enumerate()
        .filter_map(|(pkt_idx, times)| {
            if times.is_empty() {
                return None;
            }
            let min_met = times.iter().cloned().reduce(f64::min).unwrap();
            let max_met = times.iter().cloned().reduce(f64::max).unwrap();
            Some(PacketInfo {
                pkt_idx,
                min_met,
                max_met,
                n_events: times.len(),
            })
        })
        .collect();
    infos.sort_by(|a, b| a.min_met.partial_cmp(&b.min_met).unwrap());
    infos
}

/// 单个 box 的饱和重建数据
#[derive(Debug)]
pub struct BoxReconstructionData {
    /// 原始事件 MET 时间（已排序）
    pub events: Vec<f64>,
    /// FIFO reset 区间
    pub gaps: Vec<SaturationInterval>,
    /// 包信息
    pub packets: Vec<PacketInfo>,
}

/// 重建后的补全事件
#[derive(Debug, Clone)]
pub struct ReconstructedGap {
    /// 对应的 gap 索引
    pub gap_idx: usize,
    /// 补全的事件 MET 时间
    pub filled_events: Vec<f64>,
    /// 估算的 R_true
    pub r_true: f64,
    /// N_lost
    pub n_lost: usize,
    /// 是否使用了交叉参考
    pub has_cross_ref: bool,
}

const EVENTS_PER_PKT: f64 = 109.0;
const SHAPE_BIN_WIDTH: f64 = 0.001; // 1ms

/// 对单个 box 的 FIFO reset gap 进行光变曲线重建。
///
/// 算法：
/// 1. 对每个 gap，用 post-reset 包 span 估算 R_true
/// 2. N_lost = R_true × gap_duration
/// 3. 用参考 box 的事件分布构建形状函数
/// 4. 归一化到 N_lost 后分配事件
pub fn reconstruct_gaps(
    target: &BoxReconstructionData,
    references: &[&BoxReconstructionData],
) -> Vec<ReconstructedGap> {
    let mut results = Vec::new();

    for (gap_idx, gap) in target.gaps.iter().enumerate() {
        let gap_start = gap.start_met;
        let gap_stop = gap.stop_met;
        let gap_dur = gap_stop - gap_start;
        if gap_dur <= 0.0 {
            continue;
        }

        // 步骤一：估算 R_true（从 post-reset 包的 span）
        let r_true = estimate_r_true_for_gap(gap, &target.packets);
        let n_lost = (r_true * gap_dur).round() as usize;
        if n_lost == 0 {
            continue;
        }

        // 步骤二：构建形状 bin
        let n_sbins = ((gap_dur / SHAPE_BIN_WIDTH).ceil() as usize).max(1);
        let actual_sbin = gap_dur / n_sbins as f64;
        let mut shape = vec![0.0f64; n_sbins];

        // 用参考 box 填充形状
        let mut has_ref = false;
        for ref_data in references {
            // 参考 box 在 gap 内的事件
            let ref_start = ref_data
                .events
                .partition_point(|&t| t < gap_start);
            let ref_end = ref_data
                .events
                .partition_point(|&t| t <= gap_stop);
            let ref_in_gap = &ref_data.events[ref_start..ref_end];
            if ref_in_gap.len() < 5 {
                continue;
            }

            // 标定比例因子 k = target_rate / ref_rate（gap 前后 0.5s）
            let k = calibrate_ratio_sorted(
                &target.events, &ref_data.events, gap_start, gap_stop, 0.5,
            );

            // 填充形状函数（只填参考 box 未饱和的 bin）
            for (si, s) in shape.iter_mut().enumerate() {
                if *s > 0.0 {
                    continue; // 已被其他参考填充
                }
                let bin_lo = gap_start + si as f64 * actual_sbin;
                let bin_hi = bin_lo + actual_sbin;
                let bin_mid = (bin_lo + bin_hi) / 2.0;

                // 检查参考 box 在此处是否饱和
                if is_in_any_gap(bin_mid, &ref_data.gaps) {
                    continue;
                }

                // 计算参考 box 在此 bin 内的事件数
                let lo_idx = ref_data.events.partition_point(|&t| t < bin_lo);
                let hi_idx = ref_data.events.partition_point(|&t| t < bin_hi);
                let count = (hi_idx - lo_idx) as f64;
                *s = count * k;
                has_ref = true;
            }
        }

        // 对空 bin 做线性插值
        interpolate_empty_bins(&mut shape);

        // 如果完全没有参考，均匀分配
        if shape.iter().all(|&v| v <= 0.0) {
            shape.iter_mut().for_each(|v| *v = 1.0);
        }

        // 步骤三：归一化到 N_lost 并分配事件
        let total: f64 = shape.iter().sum();
        if total <= 0.0 {
            continue;
        }

        let mut filled_events = Vec::with_capacity(n_lost);
        for (si, &s) in shape.iter().enumerate() {
            let n_in_bin = (s / total * n_lost as f64).round() as usize;
            if n_in_bin > 0 {
                let bin_lo = gap_start + si as f64 * actual_sbin;
                let bin_hi = bin_lo + actual_sbin;
                let step = (bin_hi - bin_lo) / n_in_bin as f64;
                for j in 0..n_in_bin {
                    filled_events.push(bin_lo + (j as f64 + 0.5) * step);
                }
            }
        }

        results.push(ReconstructedGap {
            gap_idx,
            filled_events,
            r_true,
            n_lost,
            has_cross_ref: has_ref,
        });
    }

    results
}

fn estimate_r_true_for_gap(gap: &SaturationInterval, packets: &[PacketInfo]) -> f64 {
    // 优先用 post-reset 包
    if let Some(info) = packets.iter().find(|p| p.pkt_idx == gap.next_pkt_idx) {
        let span = info.span();
        if span > 1e-9 {
            return EVENTS_PER_PKT / span;
        }
    }
    // fallback: pre-reset 包
    if let Some(info) = packets.iter().find(|p| p.pkt_idx == gap.prev_pkt_idx) {
        let span = info.span();
        if span > 1e-9 {
            return EVENTS_PER_PKT / span;
        }
    }
    15797.0
}

fn calibrate_ratio_sorted(
    target_events: &[f64],
    ref_events: &[f64],
    gap_start: f64,
    gap_stop: f64,
    margin: f64,
) -> f64 {
    let count_in_range = |events: &[f64], lo: f64, hi: f64| -> usize {
        let a = events.partition_point(|&t| t < lo);
        let b = events.partition_point(|&t| t < hi);
        b - a
    };
    let n_target = count_in_range(target_events, gap_start - margin, gap_start)
        + count_in_range(target_events, gap_stop, gap_stop + margin);
    let n_ref = count_in_range(ref_events, gap_start - margin, gap_start)
        + count_in_range(ref_events, gap_stop, gap_stop + margin);
    if n_ref > 10 {
        n_target as f64 / n_ref as f64
    } else {
        1.0
    }
}

fn is_in_any_gap(t: f64, gaps: &[SaturationInterval]) -> bool {
    // 线性搜索（gap 数量通常在千量级，可接受）
    gaps.iter().any(|g| t >= g.start_met && t <= g.stop_met)
}

fn interpolate_empty_bins(shape: &mut [f64]) {
    let n = shape.len();
    if n == 0 {
        return;
    }
    // 找有值的 bin
    let filled: Vec<(usize, f64)> = shape
        .iter()
        .enumerate()
        .filter(|(_, v)| **v > 0.0)
        .map(|(i, v)| (i, *v))
        .collect();
    if filled.is_empty() || filled.len() == n {
        return;
    }
    // 线性插值
    for i in 0..n {
        if shape[i] > 0.0 {
            continue;
        }
        // 找左右最近的有值 bin
        let left = filled.iter().rev().find(|&&(idx, _)| idx < i);
        let right = filled.iter().find(|&&(idx, _)| idx > i);
        shape[i] = match (left, right) {
            (Some(&(li, lv)), Some(&(ri, rv))) => {
                let frac = (i - li) as f64 / (ri - li) as f64;
                lv + frac * (rv - lv)
            }
            (Some(&(_, lv)), None) => lv,
            (None, Some(&(_, rv))) => rv,
            (None, None) => 1.0,
        };
    }
}
