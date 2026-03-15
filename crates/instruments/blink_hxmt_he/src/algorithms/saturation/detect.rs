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
    for (wi, window) in summaries.windows(2).enumerate() {
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

        // 事件率检查：优先用相邻包，若拥塞包率被拉低则用附近包的率
        let rate_prev = event_rate(&window[0]);
        let rate_next = event_rate(&window[1]);
        let max_rate_adjacent = match (rate_prev, rate_next) {
            (Some(a), Some(b)) => a.max(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => continue,
        };

        let effective_max_rate = if max_rate_adjacent >= MCU_READ_RATE_FLOOR {
            max_rate_adjacent
        } else {
            // 相邻包可能是拥塞宽包（率被拉低），用附近包的率判断
            let mut neighbor_rates: Vec<f64> = Vec::new();
            for offset in 1..=5_usize {
                if wi >= offset {
                    if let Some(r) = event_rate(&summaries[wi - offset]) {
                        neighbor_rates.push(r);
                    }
                }
                if wi + 1 + offset < summaries.len() {
                    if let Some(r) = event_rate(&summaries[wi + 1 + offset]) {
                        neighbor_rates.push(r);
                    }
                }
            }
            neighbor_rates
                .iter()
                .cloned()
                .reduce(f64::max)
                .unwrap_or(max_rate_adjacent)
        };

        if effective_max_rate < MCU_READ_RATE_FLOOR {
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

/// 不可信时间区间（FIFO reset gap 或拥塞宽包的时间覆盖）
#[derive(Debug, Clone)]
pub struct UnreliableInterval {
    pub start: f64,
    pub stop: f64,
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
    /// 每个包内的事件时间（索引 = 原始包号，内部已排序）
    pub packet_events: Vec<Vec<f64>>,
    /// 不可信区间（FIFO reset gap + 拥塞宽包），用于交叉参考时排除
    pub unreliable: Vec<UnreliableInterval>,
}

/// 检测不可信时间区间：FIFO reset gap + 拥塞宽包 + 含静默丢数的包。
///
/// 三种来源：
/// 1. FIFO reset gap：整包丢失的时间段
/// 2. 拥塞宽包：包跨时 > 邻居中位跨时 × 3
/// 3. 含静默丢数的包：包内有泊松异常间隔，整个包的事件分布不可信
pub fn detect_unreliable_intervals(
    gaps: &[SaturationInterval],
    packets: &[PacketInfo],
    packet_events: &[Vec<f64>],
) -> Vec<UnreliableInterval> {
    let mut intervals: Vec<UnreliableInterval> = Vec::new();

    // 1. FIFO reset gap → 不可信
    for g in gaps {
        intervals.push(UnreliableInterval {
            start: g.start_met,
            stop: g.stop_met,
        });
    }

    // 预计算邻居中位跨时（用于宽包和内部不均匀检测）
    let packet_spans: Vec<f64> = packets.iter().map(|p| p.span()).collect();

    for (i, pkt) in packets.iter().enumerate() {
        let span = pkt.span();
        if span < 1e-9 {
            continue;
        }

        // 邻居中位跨时
        let mut neighbor_spans: Vec<f64> = Vec::new();
        for offset in 1..=5_usize {
            if i >= offset && packet_spans[i - offset] > 1e-9 {
                neighbor_spans.push(packet_spans[i - offset]);
            }
            if i + offset < packets.len() && packet_spans[i + offset] > 1e-9 {
                neighbor_spans.push(packet_spans[i + offset]);
            }
        }
        if neighbor_spans.is_empty() {
            continue;
        }
        neighbor_spans.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_span = neighbor_spans[neighbor_spans.len() / 2];

        // 2. 拥塞宽包 → 不可信
        if span > median_span * SPAN_RATIO_THRESHOLD {
            intervals.push(UnreliableInterval {
                start: pkt.min_met,
                stop: pkt.max_met,
            });
            continue;
        }

        // 3. 包内有异常大间隔 → 整个包不可信
        // 用邻居事件率做 λ，检查是否有 log10(p) < -10 的间隔
        let times = &packet_events[pkt.pkt_idx];
        if times.len() < 2 {
            continue;
        }
        let neighbor_rate = EVENTS_PER_PKT / median_span;
        let has_anomalous_interval = times.windows(2).any(|w| {
            let dt = w[1] - w[0];
            let log_p = -neighbor_rate * dt / std::f64::consts::LN_10;
            log_p < LOG10_P_THRESHOLD
        });
        if has_anomalous_interval {
            intervals.push(UnreliableInterval {
                start: pkt.min_met,
                stop: pkt.max_met,
            });
        }
    }

    // 按 start 排序
    intervals.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    intervals
}

fn is_in_unreliable(t: f64, intervals: &[UnreliableInterval]) -> bool {
    intervals.iter().any(|iv| t >= iv.start && t <= iv.stop)
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

        // 汇总所有参考 box 的事件，统一构建形状函数
        // （避免参考 box 之间的先后顺序影响结果）
        let mut has_ref = false;
        for (si, s) in shape.iter_mut().enumerate() {
            let bin_lo = gap_start + si as f64 * actual_sbin;
            let bin_hi = bin_lo + actual_sbin;
            let bin_mid = (bin_lo + bin_hi) / 2.0;

            let mut total_ref_count = 0.0;
            let mut n_valid_refs = 0;

            for ref_data in references {
                // 跳过参考 box 在此处饱和的情况
                if is_in_unreliable(bin_mid, &ref_data.unreliable) {
                    continue;
                }

                // 参考 box 在此 bin 内的事件数
                let lo_idx = ref_data.events.partition_point(|&t| t < bin_lo);
                let hi_idx = ref_data.events.partition_point(|&t| t < bin_hi);
                let count = (hi_idx - lo_idx) as f64;

                if count > 0.0 {
                    let k = calibrate_ratio_sorted(
                        &target.events, &ref_data.events, gap_start, gap_stop, 0.5,
                    );
                    total_ref_count += count * k;
                    n_valid_refs += 1;
                }
            }

            if n_valid_refs > 0 {
                // 多个参考 box 取平均
                *s = total_ref_count / n_valid_refs as f64;
                has_ref = true;
            }
            // 无参考时留空，给插值处理
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


// ── 静默丢数检测与重建 ──────────────────────────────────────────────────────

/// 包内异常间隔（静默丢数候选）
#[derive(Debug, Clone)]
pub struct SilentDrop {
    /// 包索引
    pub pkt_idx: usize,
    /// 间隔起始事件在包内的索引
    pub evt_idx: usize,
    /// 间隔起始 MET
    pub start_met: f64,
    /// 间隔结束 MET
    pub stop_met: f64,
    /// 间隔持续时间（秒）
    pub dt: f64,
    /// 包内正常事件率 λ (evt/s)
    pub lambda: f64,
    /// log₁₀(泊松概率)
    pub log10_p: f64,
    /// 估算丢失事件数
    pub n_lost: usize,
}

/// 静默丢数重建结果
#[derive(Debug, Clone)]
pub struct ReconstructedSilentDrop {
    /// 对应的 SilentDrop
    pub pkt_idx: usize,
    pub evt_idx: usize,
    /// 补全的事件 MET 时间
    pub filled_events: Vec<f64>,
    /// 丢失事件数
    pub n_lost: usize,
    /// 是否使用了交叉参考
    pub has_cross_ref: bool,
}

const LOG10_P_THRESHOLD: f64 = -10.0;
const SPAN_RATIO_THRESHOLD: f64 = 3.0; // 包跨时 > 邻居中位数 × 3 → 拥塞包

/// 检测包内静默丢数（泊松方法 + 拥塞包检测）。
///
/// 两种检测路径：
/// 1. **高率包**：包内事件率 > 邻居事件率的一半时，用包内过滤间隔估算 λ
/// 2. **拥塞宽包**：包跨时 > 邻居中位跨时 × 3 时，用邻居事件率估算 λ
///
/// 两种路径都用泊松概率 log₁₀(p) < -10 判定异常间隔。
pub fn detect_silent_drops(data: &BoxReconstructionData) -> Vec<SilentDrop> {
    let mut drops = Vec::new();

    // 预计算所有包的跨时
    let spans: Vec<(usize, f64)> = data
        .packet_events
        .iter()
        .enumerate()
        .filter_map(|(i, times)| {
            if times.len() < 2 {
                return None;
            }
            let span = times.last().unwrap() - times.first().unwrap();
            if span > 1e-9 {
                Some((i, span))
            } else {
                None
            }
        })
        .collect();

    for (pkt_idx, times) in data.packet_events.iter().enumerate() {
        if times.len() < 2 {
            continue;
        }
        let span = times.last().unwrap() - times.first().unwrap();
        if span < 1e-9 {
            continue;
        }

        // 计算邻居包的中位跨时和中位事件率（前后各 5 个包）
        let neighbor_spans: Vec<f64> = spans
            .iter()
            .filter(|&&(idx, _)| {
                let diff = if idx > pkt_idx { idx - pkt_idx } else { pkt_idx - idx };
                diff > 0 && diff <= 5
            })
            .map(|&(_, s)| s)
            .collect();

        if neighbor_spans.is_empty() {
            continue;
        }

        let mut sorted_spans = neighbor_spans.clone();
        sorted_spans.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_span = sorted_spans[sorted_spans.len() / 2];
        let neighbor_rate = EVENTS_PER_PKT / median_span;

        // 深度饱和区：所有邻居都是拥塞包，无法可靠估算 R_true/lambda
        // 跳过静默丢数检测（由 FIFO reset 重建或粗粒度修正处理）
        if neighbor_rate < MCU_READ_RATE_FLOOR {
            continue;
        }

        // 判断是否需要检测
        let rate = times.len() as f64 / span;
        let is_wide_packet = span > median_span * SPAN_RATIO_THRESHOLD;
        let is_high_rate = rate > neighbor_rate * 0.5;

        if !is_wide_packet && !is_high_rate {
            continue;
        }

        // 计算间隔
        let intervals: Vec<f64> = times.windows(2).map(|w| w[1] - w[0]).collect();

        // 估算 λ：
        // - 拥塞宽包：用邻居事件率（包自身率被丢数拉低了）
        // - 普通高率包：用包内过滤间隔
        let lambda = if is_wide_packet {
            neighbor_rate
        } else {
            let filtered: Vec<f64> = intervals.iter().copied().filter(|&dt| dt < 1e-3).collect();
            if filtered.is_empty() {
                continue;
            }
            1.0 / (filtered.iter().sum::<f64>() / filtered.len() as f64)
        };

        // 检测异常间隔
        for (j, &dt) in intervals.iter().enumerate() {
            let log_p = -lambda * dt / std::f64::consts::LN_10;
            if log_p < LOG10_P_THRESHOLD {
                let n_lost = (lambda * dt - 1.0).round().max(1.0) as usize;
                drops.push(SilentDrop {
                    pkt_idx,
                    evt_idx: j,
                    start_met: times[j],
                    stop_met: times[j + 1],
                    dt,
                    lambda,
                    log10_p: log_p,
                    n_lost,
                });
            }
        }
    }

    drops
}

/// 对检测到的静默丢数进行重建（交叉参考填充）。
pub fn reconstruct_silent_drops(
    target: &BoxReconstructionData,
    drops: &[SilentDrop],
    references: &[&BoxReconstructionData],
) -> Vec<ReconstructedSilentDrop> {
    let mut results = Vec::new();

    for drop in drops {
        let gap_start = drop.start_met;
        let gap_stop = drop.stop_met;
        let n_lost = drop.n_lost;
        if n_lost == 0 {
            continue;
        }

        // 尝试用参考 box 的事件分布
        let mut ref_events_in_gap = Vec::new();
        let mut has_ref = false;

        for ref_data in references {
            // 检查参考 box 在此间隔内是否有事件且未饱和
            if is_in_unreliable(
                (gap_start + gap_stop) / 2.0,
                &ref_data.unreliable,
            ) {
                continue;
            }
            let lo = ref_data.events.partition_point(|&t| t < gap_start);
            let hi = ref_data.events.partition_point(|&t| t <= gap_stop);
            if hi > lo {
                ref_events_in_gap.extend_from_slice(&ref_data.events[lo..hi]);
                has_ref = true;
            }
        }

        // 用 shape bin 方法分配事件（和 FIFO reset 重建一致）
        // 这样即使参考事件只覆盖部分间隔，空 bin 也会被插值填充
        let gap_dur = gap_stop - gap_start;
        let n_sbins = ((gap_dur / SHAPE_BIN_WIDTH).ceil() as usize).max(1);
        let actual_sbin = gap_dur / n_sbins as f64;
        let mut shape = vec![0.0f64; n_sbins];

        if has_ref && !ref_events_in_gap.is_empty() {
            // 用参考事件构建形状函数
            for &t in &ref_events_in_gap {
                let si = ((t - gap_start) / actual_sbin) as usize;
                if si < n_sbins {
                    shape[si] += 1.0;
                }
            }
            // 对空 bin 做插值（防止参考事件集中在部分区域导致空洞）
            interpolate_empty_bins(&mut shape);
        }

        // 如果形状全空（无参考），均匀分配
        if shape.iter().all(|&v| v <= 0.0) {
            shape.iter_mut().for_each(|v| *v = 1.0);
        }

        // 归一化到 n_lost 并分配事件
        let total: f64 = shape.iter().sum();
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

        results.push(ReconstructedSilentDrop {
            pkt_idx: drop.pkt_idx,
            evt_idx: drop.evt_idx,
            filled_events,
            n_lost,
            has_cross_ref: has_ref,
        });
    }

    results
}

fn interpolate_empty_bins(shape: &mut [f64]) {
    let n = shape.len();
    if n == 0 {
        return;
    }
    let filled_vals: Vec<f64> = shape.iter().copied().filter(|&v| v > 0.0).collect();
    if filled_vals.is_empty() || filled_vals.len() == n {
        return;
    }
    // 空 bin 用有值 bin 的均值填充（均匀分布假设）
    // 避免线性插值产生的斜坡伪影
    let mean_val = filled_vals.iter().sum::<f64>() / filled_vals.len() as f64;
    for s in shape.iter_mut() {
        if *s <= 0.0 {
            *s = mean_val;
        }
    }
}
