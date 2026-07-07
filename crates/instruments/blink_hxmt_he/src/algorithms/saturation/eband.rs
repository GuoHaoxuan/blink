//! Band-free 能量恢复：给 gap-fill 的 filler 事例确定性地补占位 channel。
//!
//! 与时间摆放同一哲学：不抽样、不引入 RNG。channel 取自**参考箱 in-gap**
//! 分布的等间隔分位（复现谱形，含尾），再用**位反转（van der Corput）**排列
//! 撒到时间有序的 filler 槽上，使 channel 与窗内时间无关（消除排序造成的
//! 假时间-能量漂移）。同一 1B 输入 → 逐字节相同输出。
//!
//! 设计与验证见
//! `docs/superpowers/specs/2026-07-03-eband-gapfill-prototype-design.md`。

/// 1B 原始 8-bit 道址 → wrapped 道址（raw < 20 表示 256+raw）。
/// 与 types::Event::channel() 的 pulse-height wrap 语义一致。
pub fn wrap_channel(raw: u8) -> u16 {
    if raw < 20 {
        raw as u16 + 256
    } else {
        raw as u16
    }
}

/// wrapped 道址 → 1B 原始 8-bit 道址（CSV 输出用，保持原始约定）。
pub fn unwrap_channel(ch: u16) -> u8 {
    if ch >= 256 {
        (ch - 256) as u8
    } else {
        ch as u8
    }
}

/// 从已排序样本里取第 ell 个（共 n 个）等间隔分位值：分位 (ell+0.5)/n。
/// n 个分位铺满经验 CDF，复现分布形状（含尾），零抽样噪声。
/// 样本为空时返回 0（调用方保证非空；空只在无任何参考+无标定窗时发生）。
pub fn quantile_value(sorted: &[u16], ell: usize, n: usize) -> u16 {
    if sorted.is_empty() {
        return 0;
    }
    let n = n.max(1);
    let q = (ell as f64 + 0.5) / n as f64;
    let idx = ((q * sorted.len() as f64) as usize).min(sorted.len() - 1);
    sorted[idx]
}

/// 基-2 radical inverse（van der Corput）：把 i 的二进制位倒过来当小数。
fn radical_inverse2(mut i: usize) -> f64 {
    let mut f = 0.0f64;
    let mut b = 0.5f64;
    while i > 0 {
        f += (i & 1) as f64 * b;
        i >>= 1;
        b *= 0.5;
    }
    f
}

/// 低差异（位反转 / van der Corput）排列：返回长度 n 的向量，
/// ranks[k] = 时间第 k 个槽位该拿的 channel 秩（0=最软）。
/// 读时间顺序时秩序列低差异散开，任意时间子段都拿到软硬均匀混合，
/// 从而 channel 与窗内时间去相关。n 为 2 的幂时即经典比特反转。
pub fn lowdisc_ranks(n: usize) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    let phi: Vec<f64> = (0..n).map(radical_inverse2).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| phi[a].total_cmp(&phi[b]));
    let mut ranks = vec![0usize; n];
    for (k, &idx) in order.iter().enumerate() {
        ranks[idx] = k;
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_roundtrip() {
        assert_eq!(wrap_channel(19), 275);
        assert_eq!(wrap_channel(20), 20);
        assert_eq!(wrap_channel(0), 256);
        assert_eq!(wrap_channel(255), 255);
        assert_eq!(unwrap_channel(275), 19);
        assert_eq!(unwrap_channel(256), 0);
        assert_eq!(unwrap_channel(44), 44);
        for raw in 0u8..=255 {
            assert_eq!(unwrap_channel(wrap_channel(raw)), raw);
        }
    }

    #[test]
    fn quantile_walks_the_cdf() {
        let sorted = [30u16, 31, 40, 41];
        // n == len：等间隔分位逐一取到每个观测值
        let drawn: Vec<u16> = (0..4).map(|l| quantile_value(&sorted, l, 4)).collect();
        assert_eq!(drawn, vec![30, 31, 40, 41]);
        // n=2：分位 0.25 / 0.75 → 取到 31 与 41
        assert_eq!(quantile_value(&sorted, 0, 2), 31);
        assert_eq!(quantile_value(&sorted, 1, 2), 41);
        // n=1：分位 0.5 → 中位
        assert_eq!(quantile_value(&sorted, 0, 1), 40);
        // 空样本兜底
        assert_eq!(quantile_value(&[], 0, 1), 0);
    }

    #[test]
    fn lowdisc_ranks_bit_reversal() {
        // 手推：8 槽位比特反转 = [0,4,2,6,1,5,3,7]
        assert_eq!(lowdisc_ranks(8), vec![0, 4, 2, 6, 1, 5, 3, 7]);
        assert_eq!(lowdisc_ranks(1), vec![0]);
        assert_eq!(lowdisc_ranks(2), vec![0, 1]);
        assert_eq!(lowdisc_ranks(0), Vec::<usize>::new());
    }

    #[test]
    fn lowdisc_ranks_is_a_permutation() {
        for n in [1usize, 2, 3, 5, 7, 8, 13, 64, 100] {
            let r = lowdisc_ranks(n);
            assert_eq!(r.len(), n);
            let mut seen = r.clone();
            seen.sort_unstable();
            assert_eq!(seen, (0..n).collect::<Vec<_>>(), "n={n} not a permutation");
        }
    }

    #[test]
    fn lowdisc_ranks_deterministic() {
        assert_eq!(lowdisc_ranks(50), lowdisc_ranks(50));
    }

    #[test]
    fn lowdisc_first_half_spans_range() {
        // 低差异性：任意前缀均匀散开——前半段应同时含低秩与高秩。
        let r = lowdisc_ranks(8);
        let first_half = &r[..4];
        assert!(first_half.iter().any(|&x| x < 4), "前半无低秩");
        assert!(first_half.iter().any(|&x| x >= 4), "前半无高秩");
    }
}
