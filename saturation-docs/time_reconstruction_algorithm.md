# 慧眼 HE 1B 时间重建算法详解

本文档描述 HXMT HE 载荷 1B 科学数据的时间重建算法。算法将 CCSDS 包内 19-bit `ptime` 计数器恢复为绝对 MET (Mission Elapsed Time)，并通过 wrap tracking 和多遍后处理修正 FIFO 拥塞、复位和 ptime 回绕边界导致的时间错位。

**代码位置**: `crates/instruments/blink_hxmt_he/src/algorithms/saturation/rec_sci_data.rs`

---

## 1. 硬件背景与关键常量

### 数据通路

```
NaI/CsI 探测器 → ASIC → FPGA → FIFO A (M67204H) → MCU (8051) → FIFO B → 1553B → 下传
```

MCU 主循环 (`PDAUA.c`) 逐事件从 FIFO A 读出，填装 CCSDS 包 (882 字节 = 6 头 + 872 载荷(109×8) + 4 UTC 尾)。

### 常量

| 名称 | 值 | 含义 |
|------|-----|------|
| `PTIME_MOD` | 524288 (2^19) | ptime 计数器模值 |
| `WRAP_PERIOD` | 1.048576s | ptime 回绕周期 = PTIME_MOD × 2μs |
| `MET_CORRECTION` | 4.0s | 1B→1K 经验时间校正 |
| `WRAP_THRESHOLD` | 10000 ticks (20ms) | 正常路径 wrap 判定阈值 |
| `ANCHOR_RECENT_PKT_LIMIT` | 35 包 | 锚点"新鲜"判定阈值 |
| `MIN_EVENTS_FOR_MEDIAN` | 50 | 包级 median 可信所需最少有效事件 |
| ptime 分辨率 | 2μs/tick | |
| CRC | 4-bit | 1/16 概率碰撞通过 |

### SEC 锚点

硬件每秒插入一个 `Pack::Second` 事件，记录 `(stime, ptime)`。`stime + offset = MET`。SEC 事件的 `(MET, ptime)` 构成时间重建的锚点 (anchor)。

### 两种丢数模式

1. **包内无痕丢数（常见）**：写速率略>读速率，FIFO 短暂满后恢复，MCU 回到主循环时已不满→不复位。无法检测。
2. **FIFO 复位整包丢失**：写速率>>读速率，HandlePhysicalLVDS 结束后 FIFO 仍满→FIFOAFullReset() 触发复位清空。包间出现大 gap，可检测。

---

## 2. 核心问题：ptime 回绕次数 n_wraps 的确定

每个事件只有 19-bit 的 `ptime`，需要确定它相对于锚点经历了多少次完整回绕。

**通用公式**：
```
raw_delta = event.ptime - anchor_ptime
MET = anchor + (raw_delta + n_wraps × PTIME_MOD) × 2μs + MET_CORRECTION
```

常规条件下 n_wraps 容易确定。但在 GRB 饱和期间：
1. **FIFO 拥塞**：事件滞留导致 `utc_tail` 严重滞后于事件真实时间
2. **FIFO 复位**：MCU 清空 FIFO 后包的文件存储顺序与时间顺序不一致
3. **ptime 回绕边界**：事件 ptime 接近锚点 ptime 时，阈值法无法区分同 wrap 和跨 wrap

---

## 3. Pass 1：逐包时间重建

函数 `reconstruct_with_wrap_tracking()` 对每个 CCSDS 包逐事件重建。三条路径按优先级：

### 路径 1：Wrap Tracking（FIFO 拥塞专用）

**触发条件**：`wrap_tracking_active && !anchor_is_recent && elapsed > WRAP_TRACKING_ELAPSED_THRESHOLD`

当锚点变"陈旧"（距 SEC 超过 35 包）且不在 FIFO 复位后，用 median ptime 跟踪回绕次数。

**核心原理**：FIFO 拥塞期间，MCU 顺序读 FIFO，连续包的 median ptime 单调递增。当 median 回绕（大负 delta），`congestion_wrap_count += 1`。这给出正确的 n_wraps 而不依赖 utc_tail。

**初始化**：SEC 被接受时 `congestion_wrap_count=0`，`prev_median_ptime=当前包median`。在 35 包 "recent" 期间不更新 `prev_median_ptime`，确保激活时首次比较跨越所有 recent 包。

**Phase Correction（相位修正）**：

`congestion_wrap_count` 跟踪的是 median ptime 的回绕次数，但时间计算基于 `anchor_ptime`（SEC 的 ptime）。当两者差距超过半周期时，median 看到的回绕比实际需要的多/少 1 次。

```
delta = anchor_median - anchor_ptime
if delta > PTIME_MOD/2:  phase_correction = -1  (median 多算 1 次)
if delta < -PTIME_MOD/2: phase_correction = +1  (median 少算 1 次)
corrected_wrap_count = congestion_wrap_count + phase_correction
```

**时间计算**：使用 `compute_met_with_base_wraps(ptime, anchor_ptime, anchor, corrected_wrap_count, median)`

```
delta_from_median = ptime - median_ptime
wrap_adjust = if delta < -PTIME_MOD/2 { +1 } else if delta > PTIME_MOD/2 { -1 } else { 0 }
total = raw_delta + (corrected_wrap_count + wrap_adjust) × PTIME_MOD
MET = anchor + total × 2μs + MET_CORRECTION
```

这消除了旧 stale path 的逐事件模糊性——当 FIFO 延迟 ≈ (k+0.5)×WRAP_PERIOD 时，旧方法对同包内事件可能分配不同 n_wraps。

### 路径 2：包级过期锚路径

**触发条件**：`!anchor_is_recent && !use_wrap_tracking && elapsed ≥ 1.5×WRAP_PERIOD`

FIFO 复位后锚点过期多个 wrap 周期，但事件是新鲜的（无 FIFO 延迟）。用 `estimate_packet_wraps` 对包的 median ptime 做 best-of-3 估算 `n_base`，再用 `compute_met_with_base_wraps` 处理包内事件。

### 路径 3：正常路径（阈值法）

**触发条件**：`anchor_is_recent` 或 `elapsed < 1.5×WRAP_PERIOD`

锚点新鲜，至多 1 次 wrap。用阈值法判定：

```rust
let raw_delta = ptime as i64 - anchor_ptime as i64;
let adjusted = if raw_delta < -WRAP_THRESHOLD {
    raw_delta + PTIME_MOD  // 正向 wrap
} else if raw_delta > (PTIME_MOD - WRAP_THRESHOLD) {
    raw_delta - PTIME_MOD  // 反向 wrap
} else {
    raw_delta
};
MET = anchor + adjusted × 2μs + MET_CORRECTION
```

**优势**：不依赖 utc_tail，不受滞后影响。
**局限**：当 `|raw_delta| < WRAP_THRESHOLD` 且事件实际跨 wrap 时会误判（Pass 3 修正）。

---

## 4. Wrap Tracking 的安全机制

### FIFO 复位检测 (`fifo_reset_no_wt`)

当 `utc_tail` 向前跳跃 > 3s 时判定为 FIFO 复位（MCU 清空 FIFO）。复位后：
- 事件是新鲜的（无 FIFO 延迟），stale path 的 utc_tail 估算正确
- 拥塞期的 wrap tracking 无效，重置所有状态
- 设 `fifo_reset_no_wt=true` 阻止 wrap tracking 重新激活
- 直到新 SEC 锚点被接受时 `fifo_reset_no_wt` 重置

### 损坏包过滤 (`MIN_EVENTS_FOR_MEDIAN`)

CCSDS 包 109 个事件中只有少数通过 4-bit CRC 时，说明包数据损坏。这些事件的 ptime 不可靠（随机数据碰巧通过 CRC），其 median 可能严重偏离真实值。

保护措施：
- 有效事件 < 50 的包跳过 wrap 检测（不触发 WRAP_INC）
- 不更新 `prev_median_ptime`（避免污染后续比较基准）

### 仅递增规则（无 WRAP_DEC）

FIFO 拥塞期间，MCU 从 FIFO 顺序读出事件，ptime 只会单调递增。不存在反向回绕的物理可能。因此只检测 WRAP_INC（median 大负 delta），不检测 WRAP_DEC。

---

## 5. Pass 2：FIFO 复位后时间倒退修正 (`fix_wrap_reversals`)

### 问题

FIFO 复位后，文件中的包顺序与时间顺序不一致。过期锚路径使用的 `utc_tail` 有偏，导致一批包被放到高一个 WRAP_PERIOD 的位置。

**表现**：一批包的时间比后续包高 ~WRAP_PERIOD（时间倒退）。

### 算法

1. 构建"干净包"序列（span < 0.3s）
2. 扫描相邻干净包间的**反向跳跃** ≈ -WRAP_PERIOD
3. 从跳跃点反向查找属于高层级的包批次
4. **Gap 判据**区分真假倒退：
   - 批次前有大 gap (> 0.3s) → FIFO 复位后的真错位 → 整批下移 -WRAP_PERIOD
   - 批次前无 gap（平滑衔接） → 纯文件重排的假倒退 → 跳过

---

## 6. Pass 3：ptime 回绕边界修正 (`fix_wrap_boundary_dips`)

### 问题

当 `anchor_ptime` 接近 ptime 回绕边界时，一些事件的 `raw_delta` 落在 `[-WRAP_THRESHOLD, +WRAP_THRESHOLD]` 范围内。正常路径将它们留在当前 wrap 层级，但实际属于下一个 wrap 周期。

**表现**：在 ~WRAP_PERIOD 间隔处出现 "dip"——一小批包的时间比前后邻居低一个 WRAP_PERIOD。

### 算法

#### Step 1：Dip Batch 检测与修正

扫描正向跳跃 ≈ +WRAP_PERIOD，反向查找被 HIGH 包"夹心"的 LOW 批次，整批上移 +WRAP_PERIOD。

#### Step 2：混合包修正

对 span ≈ WRAP_PERIOD 的包，按邻居层级判定多数/少数簇，将少数簇对齐（±WRAP_PERIOD）。

---

## 7. Pass 4：全局排序

当检测到跨包时间倒退（FIFO 复位导致的包重排）时，全局排序所有事件并重分配到包。

---

## 8. 处理管道总结

```
CCSDS 包序列
    │
    ▼
Pass 1: 逐包时间重建 (三路径)
    │   路径1 Wrap Tracking: median 追踪 + phase correction
    │   路径2 Stale Path: utc_tail 辅助 best-of-3
    │   路径3 Normal Path: 阈值法, ≤1 wrap
    │
    ▼
Pass 2: fix_wrap_reversals (逐包批次)
    │   检测反向跳跃 ≈ -WRAP_PERIOD
    │   Gap 判据区分真假倒退
    │
    ▼
Pass 3: fix_wrap_boundary_dips (逐包批次)
    │   Step 1: 检测正向跳跃, 修正 dip batch
    │   Step 2: 修正混合包内少数簇
    │
    ▼
Pass 4: Global sort (按需)
    │   跨包倒退时全局排序
    │
    ▼
输出: Vec<Vec<f64>> — 每包的重建 MET 时间列表
```

---

## 9. 验证结果 (2026-03-13)

### GRB 200415A（无饱和）
- 1B/1K 事件数完全一致（Δ=0），所有 Box 所有 bin 误差 0.0%

### GRB 260226A（中度饱和）
- 1B/1K 差异仅 2 个事件（+0.0%），所有 Box 所有 bin 误差 0.0%
- Pass 3 触发: Box A 7包, Box B 14包, Box C 13包

### GRB 221009A（极端饱和，史上最亮 GRB）
- **Box A**: 1B=3,857,418 vs 1K=3,857,310 (Δ=+108, +0.0%)
- **Box B**: 1B=3,800,510 vs 1K=3,797,574 (Δ=+2,936, +0.1%)
- **Box C**: 1B=3,745,229 vs 1K=3,619,371 (Δ=+125,858, +3.5%)
- 非饱和区 1B/1K 完美对齐
- 饱和边界差异属于 1B/1K 不同数据源固有差异

---

## 10. 已修复问题记录

### Box C T+328 空洞 — WRAP_DEC 移除
**根因**：anchor_ptime≈523221 近 PTIME_MOD 上界，35 包 recent 后 ptime 正常递增到 ~240000，diff > PTIME_MOD/2 误触发 WRAP_DEC。
**修复**：移除 WRAP_DEC 分支。FIFO 拥塞期间 ptime 只单调递增。

### Box B T+261-262 空洞 — 损坏包 median
**根因**：pkt 39981 损坏，109 事件仅 4 个通过 CRC。随机 ptime 的 median=66592（真实≈355000），误触发 WRAP_INC，wraps 从 2 跳到 3。
**修复**：MIN_EVENTS_FOR_MEDIAN=50，有效事件不足的包跳过 wrap 检测。

### Box A T+550 空洞 — Phase correction
**根因**：SEC 的 anchor_ptime=49418 近 0，但该包 median=521070 近 PTIME_MOD。Median 回绕 1 次（521070→236896），但 anchor_ptime 到 236896 不需要回绕。Wrap tracking 多算 1 次。
**修复**：记录 anchor_median_ptime，计算 phase_correction(-1/0/+1)，corrected_wrap_count = congestion_wrap_count + phase_correction。

### FIFO 复位后 wrap tracking 误激活 — fifo_reset_no_wt
**根因**：FIFO 复位后事件是新鲜的，wrap tracking 不应重新激活。
**修复**：UTC_JUMP 时设 fifo_reset_no_wt=true，直到新 SEC 锚点接受时重置。

---

## 11. 已知局限

1. **4-bit CRC 碰撞**：高计数率时 ~6% 随机字节序列可能通过 CRC 校验，产生幽灵事件。
2. **无痕包内丢数**：FIFO 短暂满后恢复，丢失事件无法检测或恢复。
3. **wrap 边界残余**：WRAP_PERIOD ≠ 1.0s，极少数 raw_delta=0 的事件（概率 ~1/500000）无法判定归属。
4. **饱和边界差异**：1B 和 1K 在 FIFO 复位转换处的事件来源不同，存在固有差异。

---

## 附录 A：历史尝试记录

详见 [wrap_fix_attempts.md](wrap_fix_attempts.md)。

早期尝试包括 6 种 utc_tail 公式变体，均因无法同时处理 utc_tail 滞后/正常/过期三种状态而失败。最终方案采用 wrap tracking + SEC 锚点 + 阈值法 + 四遍后处理，消除了对 utc_tail 绝对值的依赖。
