use super::Chunk;
use crate::algorithms::saturation::scan_saturation_intervals;
use crate::types::HxmtHe;
use blink_core::types::MissionElapsedTime;

type Interval = (MissionElapsedTime<HxmtHe>, MissionElapsedTime<HxmtHe>);

impl Chunk {
    /// 计算三个机箱的饱和时间段并合并（任一机箱饱和即算饱和）。
    pub fn get_saturation_intervals(&self) -> Vec<Interval> {
        let mut all_intervals: Vec<Interval> = Vec::new();
        for ((_, sci_file), (_, offset)) in self.sci_files.iter().zip(self.stime_offsets.iter()) {
            all_intervals.extend(scan_saturation_intervals(sci_file, *offset));
        }

        // 按起始时间排序
        all_intervals.sort_by(|a, b| a.0.cmp(&b.0));

        // 合并有重叠的区间（并集）
        let mut merged: Vec<Interval> = Vec::new();
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

        merged
    }

    /// 判断给定时间点是否处于饱和状态（二分查找）。
    pub fn check_saturation(&self, time: MissionElapsedTime<HxmtHe>) -> bool {
        let intervals = self.get_saturation_intervals();
        is_in_intervals(&intervals, time)
    }
}

/// 二分查找判断时间点是否落在某个饱和区间内。
fn is_in_intervals(intervals: &[Interval], time: MissionElapsedTime<HxmtHe>) -> bool {
    let idx = intervals.partition_point(|interval| interval.1 < time);
    idx < intervals.len() && intervals[idx].0 <= time
}
