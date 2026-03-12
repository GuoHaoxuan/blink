# CRC 碰撞幽灵事件：原理、危害与解决方案

## 1. 背景：HE 载荷的 4-bit CRC

慧眼 HE 载荷每个 CCSDS 科学数据包包含 109 个事例（event slot），每个事例 8 字节。
事例的最后 4 bit 是 CRC 校验码，用于检测数据传输错误。

```
事例格式 (8 字节):
  byte[0]     : channel (能道)
  byte[1-3]   : 依类型不同含义不同
  byte[4-6]   : ptime 的高位编码
  byte[7]     : [7:6]=ptime低2位, [5:4]=类型标识, [3:0]=CRC校验
```

CRC 校验流程：
```
computed_crc = crc_check(row[0..7])   // 4-bit 结果
stored_crc   = row[7] & 0x0F
if computed_crc == stored_crc → 事例通过 CRC，被认为是有效数据
else → 标记为 Pack::Error，丢弃
```

### 4-bit CRC 的固有缺陷

4-bit CRC 只有 16 种可能值。当一个事例的数据被损坏（比如 FIFO 读写冲突、
电磁干扰等导致的 bit 翻转），损坏后的数据经过 CRC 计算，有 **1/16 = 6.25%**
的概率恰好等于存储的 CRC 值。

这种情况下，一个**完全错误的事例会被误判为有效数据**——我们称之为
**"CRC 碰撞幽灵事件"（CRC collision ghost event）**。

## 2. 正常情况下为何不是问题

在正常计数率（~几百 evt/s）下：
- 每个包 109 个事例中，CRC 错误通常 < 5 个
- 碰撞通过的幽灵 ≈ 5 × (1/16) ≈ 0.3 个/包
- 偶尔一个幽灵事件不影响整体重建

## 3. 高计数率下的幽灵泛滥

在极端高亮爆发（如 GRB 260226A，T+19~21s）下：
- 事件产生速率远超 FIFO 读取速率
- FIFO 读写冲突频繁，大量事例数据损坏
- **109 个事例中 100+ 个 CRC 错误**（错误率 >90%）

此时幽灵数量：
```
CRC 错误事例 ≈ 100
碰撞通过数   ≈ 100 / 16 ≈ 6 个
真实通过数   ≈ 109 - 100 = 9 个
```

在 9 个"通过 CRC"的事例中，约 6 个是幽灵！**多数"有效"事例实际是垃圾数据。**

## 4. 幽灵事件如何破坏时间重建

### 4.1 幽灵事件的特征

幽灵事件的 8 字节数据完全是损坏后的随机值，因此：
- **channel**：随机（0~255）
- **ptime**：随机（0~524287）
- **类型标识**：随机（可能被误判为 EVT 或 SEC）

### 4.2 对顺序追踪算法的毒害

WrapTracker 的核心逻辑是通过检测 ptime 回落来判定 wrap：

```
若 ptime < prev_ptime 且差值 > HALF_MOD (262144)
→ 判定发生了一次 wrap，wrap_count += 1
```

当一个幽灵事件携带随机 ptime 被追踪时：

**场景**：prev_ptime = 400000（真实值，接近 PTIME_MOD 上限）
- 幽灵 ptime = 50000（随机小值）
- 差值 = 400000 - 50000 = 350000 > HALF_MOD
- **误判为 wrap！** wrap_count 错误地 +1

此后所有真实事件的 MET 都会偏移 +1.048576s。

更严重的是，幽灵的随机 ptime 成为新的 prev_ptime，可能触发连锁错误：
后续真实事件的 ptime 与这个垃圾 prev_ptime 比较，可能再次误判 wrap。

### 4.3 实际观测到的现象

GRB 260226A 中：
- **Box B (T+19~21s)**：~8000 个 1B 事件被整体偏移 ~1.048s
- **Box C (T+23~26s)**：类似偏移
- 1B 光变曲线相对 1K 出现明显的时间位移
- scatter 图中可见事件"条带"错位

## 5. 为什么旧的 floor 公式对此免疫

旧的 `compute_met` 函数使用 per-event floor 公式：

```rust
let n_wraps = ((utc_tail - anchor - WRAP_PERIOD - raw_delta_seconds) / WRAP_PERIOD)
    .floor()
    .max(0.0) as i64;
```

每个事件独立用 utc_tail 估算 wrap 数，**不依赖 prev_ptime**。
幽灵事件的垃圾 ptime 只影响自身的 MET 计算，不会传播到后续事件。

但 floor 公式在 wrap 边界处不稳定（同一包内相邻事件可能得到不同的 n_wraps），
导致 GRB 221009A 出现包时间重叠问题。

## 6. 解决方案：混合追踪 + floor 校准

### 6.1 核心思路

将两种方法的优势结合：
- **正常区域**：使用顺序追踪（WrapTracker），精确处理 wrap 边界
- **高错误率区域**：跳过追踪，避免幽灵污染
- **区域过渡**：用 floor 公式重新校准 wrap_count，然后恢复顺序追踪

### 6.2 实现细节

#### WrapTracker 增加 `needs_recalibration` 状态

```rust
struct WrapTracker {
    anchor: f64,
    anchor_ptime: u64,
    prev_ptime: u64,
    wrap_count: i64,
    has_anchor: bool,
    needs_recalibration: bool,  // 标记是否需要 floor 校准
}
```

#### 三层防御机制

**第一层：高错误率包跳过追踪**

当一个 CCSDS 包中 CRC 错误数 > 50（约 46%），跳过该包的 ptime 追踪，
并调用 `tracker.mark_skip_zone()` 标记需要校准：

```rust
let skip_tracking = n_errors > MAX_CRC_ERRORS_FOR_TRACKING;
if skip_tracking {
    tracker.mark_skip_zone();
}
```

SEC 锚点更新不受影响（`try_update_anchor` 始终调用，自带 utc_tail 校验）。

**第二层：floor 公式校准**

退出高错误区域后，首个事件的 `track_and_compute` 检测到 `needs_recalibration`，
使用 floor 公式独立估算 wrap_count：

```rust
if self.needs_recalibration {
    let raw_delta = ptime as i64 - self.anchor_ptime as i64;
    let raw_delta_seconds = raw_delta as f64 * 2e-6;
    tentative_wrap = ((utc_tail - self.anchor - WRAP_PERIOD - raw_delta_seconds) / WRAP_PERIOD)
        .floor()
        .max(0.0) as i64;
}
```

校准成功后（通过 utc_tail 校验）恢复正常顺序追踪。

**第三层：utc_tail 合理性校验**

所有模式下，计算出的 MET 都需通过 utc_tail 校验：

```
|met - utc_tail - MET_CORRECTION| > 2.0 → 拒绝，不更新状态
```

这是最后的兜底，防止残余幽灵事件或校准误差。

### 6.3 为什么这个方案有效

| 场景 | 处理方式 | 效果 |
|------|----------|------|
| 正常数据 | 顺序追踪 | wrap 边界精确，无 221009A 重叠 |
| 高错误区内 | 跳过追踪 | 幽灵 ptime 不污染 prev_ptime |
| 高错误→正常过渡 | floor 校准 | 正确恢复 wrap_count（±1 wrap 精度） |
| floor 校准 ±1 误差 | 下一个 SEC 锚点重置 | 自动修正 |

## 7. 阈值选择

`MAX_CRC_ERRORS_FOR_TRACKING = 50`（109 个事例中）

- 正常数据包：CRC 错误 < 5，远低于阈值
- 高错误区域：CRC 错误 > 90，远高于阈值
- 50 的阈值提供了充分的安全裕度

## 8. 验证结果

| GRB | 特征 | 结果 |
|-----|------|------|
| 200415A | 无饱和，正常数据 | 1B = 1K 完全一致 (Δ=0) |
| 221009A | 强饱和，FIFO 复位丢数 | 无包重叠，饱和区正确缺失 |
| 260226A | 高 CRC 错误率区域 | 1B ≈ 1K (每 Box Δ≤3)，偏移消除 |
