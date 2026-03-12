# 慧眼 HE 1B 时间重建算法详解

本文档描述 HXMT HE 载荷 1B 科学数据的时间重建算法。算法将 CCSDS 包内 19-bit `ptime` 计数器恢复为绝对 MET (Mission Elapsed Time)，并通过三遍后处理修正 FIFO 复位和 ptime 回绕边界导致的时间错位。

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
| ptime 分辨率 | 2μs/tick | |
| CRC | 4-bit | 1/16 概率碰撞通过 |

### SEC 锚点

硬件每秒插入一个 `Pack::Second` 事件，记录 `(stime, ptime)`。`stime` 是卫星绝对秒数，`stime + offset = MET`。SEC 事件的 `(MET, ptime)` 构成时间重建的锚点 (anchor)。

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

算法分三遍处理这些问题。

---

## 3. Pass 1：逐事件时间重建 (`compute_met_anchored`)

函数 `reconstruct_with_wrap_tracking()` 对每个 CCSDS 包逐事件重建：

1. 遇到 `Pack::Second` 时更新锚点 `(anchor, anchor_ptime)`
2. 对每个事件调用 `compute_met_anchored(ptime, anchor_ptime, anchor, utc_tail)`

### 两路径选择

基于 `elapsed = utc_tail - anchor`：

#### 正常路径 (elapsed < 1.5 × WRAP_PERIOD)

锚点是新鲜的（距 SEC 不到 1.5 个回绕周期），至多发生 1 次 wrap。用阈值法判定：

```rust
let raw_delta = ptime as i64 - anchor_ptime as i64;
let adjusted_delta = if raw_delta < -WRAP_THRESHOLD {
    raw_delta + PTIME_MOD  // 正向 wrap：ptime 从高值绕回低值
} else if raw_delta > (PTIME_MOD - WRAP_THRESHOLD) {
    raw_delta - PTIME_MOD  // 反向 wrap：锚点 ptime 小，事件 ptime 大但在前一周期
} else {
    raw_delta              // 无 wrap
};
MET = anchor + adjusted_delta × 2μs + MET_CORRECTION
```

**优势**：不依赖 `utc_tail`，不受 utc_tail 滞后影响。
**局限**：当 `|raw_delta| < WRAP_THRESHOLD` 且事件实际跨 wrap 时会误判（Pass 3 修正）。

#### 过期锚路径 (elapsed ≥ 1.5 × WRAP_PERIOD)

FIFO 复位后锚点可能过期多个 wrap 周期。用 `utc_tail` 辅助估算：

```rust
let n_est = round((elapsed - raw_delta × 2μs) / WRAP_PERIOD).max(0);

// Best-of-3：在 n_est-1, n_est, n_est+1 中选距 utc_tail 最近的
for n in [n_est-1, n_est, n_est+1] {
    let met = anchor + (raw_delta + n × PTIME_MOD) × 2μs + MET_CORRECTION;
    // 选 |met - MET_CORRECTION - utc_tail| 最小的
}
```

**优势**：能处理多次 wrap。
**局限**：`utc_tail` 反映包组装时间而非事件时间，FIFO 复位后有偏。

---

## 4. Pass 2：FIFO 复位后时间倒退修正 (`fix_wrap_reversals`)

### 问题

FIFO 复位后，文件中的包顺序与时间顺序不一致。过期锚路径使用的 `utc_tail` 有偏（反映组装时间而非事件时间），导致一批包被放到高一个 WRAP_PERIOD 的位置。

**表现**：一批包的时间比后续包高 ~WRAP_PERIOD（时间倒退）。

### 算法

1. 构建"干净包"序列（span < 0.3s）
2. 扫描相邻干净包间的**反向跳跃** ≈ -WRAP_PERIOD
3. 从跳跃点反向查找属于高层级的包批次
4. **Gap 判据**区分真假倒退：
   - 批次前有大 gap (> 0.3s) → FIFO 复位后的真错位 → 整批下移 -WRAP_PERIOD
   - 批次前无 gap（平滑衔接） → 纯文件重排的假倒退 → 跳过

### 示例 (GRB 221009A)

```
...pkt 46148 T+18.5... pkt 46202 T+19.02  ← 正确位置
   gap=0.024s (平滑) → SKIP
```

---

## 5. Pass 3：ptime 回绕边界修正 (`fix_wrap_boundary_dips`)

### 问题

当锚点的 `anchor_ptime` 接近 ptime 回绕边界时，一些事件的 `raw_delta` 落在 `[-WRAP_THRESHOLD, +WRAP_THRESHOLD]` 范围内。正常路径将它们留在当前 wrap 层级，但它们实际上属于下一个 wrap 周期，应高 WRAP_PERIOD。

**表现**：在 ~WRAP_PERIOD 的时间间隔处出现"dip"——一小批包的时间比前后邻居低一个 WRAP_PERIOD，光变曲线上表现为空隙+跳动。

**根因分析** (以 GRB 260226A Box A 为例)：

```
锚点: anchor_ptime=P, anchor=T+17.98
事件 ptime 经一次完整回绕后: ptime ≈ P + 几百 ticks
raw_delta = ptime - P ≈ 几百 (< WRAP_THRESHOLD)
→ 正常路径不调整 → MET = T+17.98 + 微小偏移
→ 实际应为 T+19.03 (= T+17.98 + WRAP_PERIOD)
```

受影响的事件分布在两种包类型中：
1. **混合包** (span ≈ WRAP_PERIOD)：少数事件在错误的低层级，多数在正确的高层级
2. **邻近的纯低层级包** ("dip batch")：整包被错误放在低层级

### 算法

分两步执行：

#### Step 1：Dip Batch 检测与修正

扫描干净包间的**正向跳跃** ≈ +WRAP_PERIOD（从低到高），反向查找被"夹心"的低层级批次：

```
HIGH packets → [LOW dip batch] → HIGH packets
                ↑ 向上移 +WRAP_PERIOD
```

判定条件：
- 正向跳跃 ∈ [0.8×WRAP, 1.2×WRAP]
- 反向查找到前方的 HIGH 包，确认反向跳跃 > 0.5×WRAP（即真正被 HIGH 夹心）
- 反向查找时跳过混合包（span > 0.5×WRAP），避免被阻断

修正动作：
- 整批包上移 +WRAP_PERIOD
- 相邻混合包中的 LOW 事件也一并上移

#### Step 2：混合包修正

对剩余 span ≈ WRAP_PERIOD 的包，根据邻居包的时间层级判定多数/少数簇：

```
将包内事件按 midpoint = min + 0.5×WRAP_PERIOD 分为 LOW/HIGH 两簇
检查两侧最近的非混合邻居包的时间层级
若邻居以 HIGH 为主 → LOW 簇上移 +WRAP_PERIOD
若邻居以 LOW 为主 → HIGH 簇下移 -WRAP_PERIOD
```

### 修正效果 (GRB 260226A)

| Box | 受影响位置 | 修正前逐秒Δ% | 修正后逐秒Δ% | 修正的包 |
|-----|-----------|------------|------------|---------|
| A | T+19 | -5.7% (deficit 689) | 0.0% | pkts 46203-46209 |
| B | T+21, T+24 | -4.7%, -8.9% | 0.0% | pkts 47261-47266, 47573-47581 |
| C | T+19, T+24 | -5.6%, -8.4% | 0.0% | pkts 44921-44927, 45436-45443 |

---

## 6. 三遍处理管道总结

```
CCSDS 包序列
    │
    ▼
Pass 1: compute_met_anchored (逐事件)
    │   正常路径: 阈值法, ≤1 wrap
    │   过期路径: utc_tail 辅助, 多 wrap best-of-3
    │
    ▼
Pass 2: fix_wrap_reversals (逐包批次)
    │   检测反向跳跃 ≈ -WRAP_PERIOD
    │   Gap 判据区分真假倒退
    │   真倒退批次下移 -WRAP_PERIOD
    │
    ▼
Pass 3: fix_wrap_boundary_dips (逐包批次)
    │   Step 1: 检测正向跳跃, 修正 dip batch (+WRAP_PERIOD)
    │   Step 2: 修正混合包内少数簇 (±WRAP_PERIOD)
    │
    ▼
输出: Vec<Vec<f64>> — 每包的重建 MET 时间列表
```

---

## 7. 验证结果

### GRB 200415A (无饱和)
- 1B ≈ 1K，逐秒 Δ=+1 (SEC 事件)，无 wrap reversal 触发，无 dip 触发

### GRB 221009A (中度饱和)
- 非饱和区 1B/1K 完美对齐
- 饱和区 1B 正确缺失（FIFO 丢数）
- 无 wrap reversal 触发，无 dip 触发

### GRB 260226A (重度饱和, 三机箱均饱和)
- 所有 Box 逐秒 Δ=0~4 (0.0%)
- 100ms bin 无任何 >30% 偏差
- Pass 2 触发: 无（此 GRB 中 gap 判据均为平滑衔接 → 跳过）
- Pass 3 触发: Box A 7个包, Box B 14个包, Box C 13个包

---

## 8. 已知局限

1. **4-bit CRC 碰撞**：高计数率时 ~6% 的随机字节序列可能通过 CRC 校验，产生幽灵事件。这些幽灵事件的 ptime 和 channel 是随机的，无法完全过滤。
2. **无痕包内丢数**：当 FIFO 短暂满后恢复（MCU 回到主循环时已不满），丢失的事件无法检测或恢复。
3. **wrap 边界 48.6ms 残余**：WRAP_PERIOD = 1.048576s ≠ 1.000s，因此 wrap 边界不与整秒对齐。Pass 3 修正了可检测的错位，但 raw_delta 恰好等于 0 的极少数事件（概率 ~1/500000）在理论上无法判定归属。

---

## 附录 A：历史尝试记录

详见 [wrap_fix_attempts.md](wrap_fix_attempts.md)。

早期尝试包括：
1. `prev_ptime` 顺序追踪（FIFO 乱序时失败）
2. 纯 `utc_tail` floor 公式的 6 种变体（没有固定偏移能同时适用所有场景）
3. UTC-Bounds 高水位线防御（引入非物理的单调性约束）

最终方案采用 SEC 锚点 + 阈值法 + 三遍后处理，避免了对 `utc_tail` 绝对值的依赖。
