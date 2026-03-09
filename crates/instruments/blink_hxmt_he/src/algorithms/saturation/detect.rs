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
