# ptime 回绕修复：尝试记录

## 背景

HXMT HE 1B 数据的时间重建需要处理 19-bit ptime 计数器的回绕（周期 524288 ticks = 1.048576 秒）。

### 关键概念

| 名称 | 含义 |
|------|------|
| ptime | 19-bit 事例计时器 (2μs/tick)，周期 1.048576s |
| anchor | 最近的 Second 事例的 MET (stime + offset) |
| anchor_ptime | 该 Second 事例的 ptime 值 |
| utc_tail | CCSDS 包尾部 4 字节 LE u32，整秒级时间戳 |
| MET_CORRECTION | 1B→1K 时间校正 = 4.0s |
| PTIME_MOD | 524288 (1 << 19) |
| WRAP_PERIOD | 1.048576s |

### 通用公式框架

```
raw_delta = ptime - anchor_ptime  （可为负）
total_ticks = n_wraps × PTIME_MOD + raw_delta
MET = anchor + total_ticks × 2μs + MET_CORRECTION
```

核心问题：**如何确定 n_wraps？**

---

## 问题 1：原始 prev_ptime 方法的缺陷

原始代码使用顺序追踪：
```rust
if ptime < prev_ptime && (prev_ptime - ptime) > HALF_MOD {
    wrap_count += 1;
}
prev_ptime = ptime;
```

**失败场景**：FIFO 溢出后事例时序混乱。例如 Box A pkt 46016：
- evt 0: ptime=1656 (真正的回绕后事例) → wrap_count=1 ✓
- evt 1: ptime=423970 (回绕前事例，FIFO 延迟到达) → 继承 wrap_count=1 ✗

**后果**：~8000 事例/机箱被错误延后 1.048576 秒。

---

## 问题 2：utc_tail 的特性（实测）

通过对比 1K ground truth 和 1B 数据：

| 属性 | 值 |
|------|-----|
| 数据类型 | 整秒 (floor) |
| 更新频率 | ~1 次/秒 |
| 与 anchor 的关系 | 在同一时间尺度 (stime + offset) |
| 与事例真实时间的关系 | 事例 raw_met ∈ [utc_tail, utc_tail + 2) |
| 滞后特性 | anchor 更新后 utc_tail 可能还停留在旧值，最大滞后 ~1-2 秒 |

**关键观测**（Box A 饱和区域）：
```
Pkt 46079-46090: utc_tail=446726292, anchor=446726291 (旧锚点@295)
Pkt 46091-46100: utc_tail=446726292, anchor=446726292 (新锚点@296，utc_tail 未更新！)
Pkt 46101+:      utc_tail=446726293, anchor=446726292 (新锚点，utc_tail 已更新)
```

---

## 尝试 1：floor((utc_tail - anchor - Δt) / T_wrap) 【当前已提交版本】

```rust
let n_wraps = ((utc_tail - anchor - raw_delta_seconds) / WRAP_PERIOD)
    .floor().max(0.0) as i64;
```

**原理**：utc_tail - anchor 给出从锚点到当前时刻的粗略秒数。减去 raw_delta 后除以回绕周期得到回绕次数。

**结果**：
- ✅ 完美解决了 FIFO 溢出导致的事例乱序问题（pkt 46016 的 evt 1-108）
- ✅ 非饱和区域完全正确
- ❌ 饱和区域残余问题：~978 事例（Box A）从 T+19.0 错位到 T+17.95

**失败原因**：当 anchor 刚更新但 utc_tail 尚未跟进时（pkt 46091-46100），`utc_tail - anchor ≈ 0`，导致 n_wraps 被低估。

**具体失败案例**：
```
anchor = 446726292 (Second@296)
utc_tail = 446726292 (滞后，还没更新到 293)
ptime = 230000, anchor_ptime = 216822
raw_delta = 13178, raw_delta_seconds = 0.026356

n_wraps = floor((292 - 292 - 0.026) / 1.048576) = floor(-0.025) = -1 → max(0) = 0
实际需要 n_wraps = 1 才能到达正确的 T+19 位置
```

---

## 尝试 2：floor((utc_tail + 1 - anchor - Δt) / T_wrap)

```rust
let n_wraps = ((utc_tail + 1.0 - anchor - raw_delta_seconds) / WRAP_PERIOD)
    .floor().max(0.0) as i64;
```

**原理**：Oracle 建议将 utc_tail 视为 floor(真实时间)，因此事例时间 < utc_tail + 1。

**结果**：
- ❌ 严重破坏非饱和区域
- ❌ 整体差异远大于尝试 1

**失败原因**：utc_tail 不是 floor(事例时间)。实测事例可以在 utc_tail 后 2 秒，不是 1 秒。+1 的假设过于保守，同时又对另一些事例过度修正。

---

## 尝试 3：ceil((utc_tail - 1 - anchor - Δt) / T_wrap) 【下界方法】

```rust
let lower = utc_tail - 1.0;
let n_wraps = ((lower - base) / WRAP_PERIOD).ceil().max(0.0) as i64;
```

**原理**：用 utc_tail - 1 作为事例时间的下界，用 ceil 选择最小的合法 n_wraps。

**结果**：
- ✅ 大部分区域正确（T+17.95~19.00 和 T+19.00~19.05 修复）
- ❌ 残余问题：T+18.00~18.05 多出 288 事例，T+19.05~19.10 缺少 325 事例

**失败原因**：下界 utc_tail - 1 对于 anchor 刚更新但 utc_tail 滞后的场景仍然不够。
当 base > lower（即 anchor + raw_delta > utc_tail - 1）时，ceil 返回 0，但实际需要 1。

---

## 尝试 4：round((utc_tail + 0.5 - anchor - Δt) / T_wrap) 【中心点方法】

```rust
let target = utc_tail + 0.5;
let n_wraps = ((target - base) / WRAP_PERIOD).round().max(0.0) as i64;
```

**原理**：用窗口中心 utc_tail + 0.5 作为目标，四舍五入到最近的回绕次数。

**结果**：
- ❌ 全面混乱，几乎每个 bin 都有显著偏差

**失败原因**：round 在边界处的行为不可预测，当 (target - base) / T_wrap 接近 0.5 时会随机跳变。

---

## 尝试 5：floor((utc_tail + 2 - anchor - Δt) / T_wrap)

```rust
let n_wraps = ((utc_tail + 2.0 - anchor - raw_delta_seconds) / WRAP_PERIOD)
    .floor().max(0.0) as i64;
```

**原理**：实测事例最远在 utc_tail + 2 秒内，因此用 +2 作为上界。

**结果**：
- ❌ T+17.85~17.95 区域严重错位（破坏了原本正确的区域）
- 与尝试 2 类似但更严重

**失败原因**：+2 过大，导致本不需要额外回绕的事例被推到了下一个周期。

---

## 尝试 6：ptime HALF_MOD 判断（回归原始逻辑的变体）

```rust
let n_wraps = if raw_delta < 0 && (-raw_delta) > HALF_MOD as i64 { 1 } else { 0 };
```

**原理**：不使用 utc_tail，纯粹根据 ptime 与 anchor_ptime 的距离判断。

**结果**：
- ❌ 完全失败，T+19 区域 5000+ 事例缺失
- ❌ 只能处理 0 或 1 次回绕，无法处理多次回绕

**失败原因**：这本质上是原始 prev_ptime 方法的简化版，无法处理超过 1 个回绕周期的情况。

---

## 核心困难总结

| 方法 | 对旧锚点(utc=292,anc=291) | 对新锚点+滞后utc(utc=292,anc=292) | 对新锚点+正常utc(utc=293,anc=292) |
|------|---------------------------|----------------------------------|----------------------------------|
| floor(utc-anc-Δ) | ✅ n=1 正确 | ❌ n=0 应为 1 | ✅ n=1 正确 |
| floor(utc+1-anc-Δ) | ❌ n=2 过大 | ✅ n=1 正确 | ❌ n=2 过大 |
| floor(utc+2-anc-Δ) | ❌ n=2 过大 | ❌ n=2 过大 | ❌ n=2 过大 |

问题在于：**没有一个固定偏移量能同时适用于所有三种情况**。

---

## 未尝试的方向

1. **混合方法**：先用 utc_tail 公式计算 n_wraps，再用结果反算 MET，检查 MET 是否在 [utc_tail, utc_tail+2) 窗口内，不在则调整 ±1。

2. **包级批处理**：同一个 CCSDS 包内的事例应该有相同的 n_wraps（因为包内时间跨度仅 ~0.01s）。可以先确定包级的 n_wraps，再应用到包内所有事例。

3. **utc_tail 差分法**：跟踪 utc_tail 的变化来检测真实时间的流逝，而非用 utc_tail 的绝对值。

4. **双锚点回退**：对于 utc_tail 滞后的包（utc_tail == anchor），暂时回退使用上一个锚点来计算，避免 utc_tail - anchor = 0 的问题。

5. **utc_tail 修正**：检测 utc_tail 滞后情况（utc_tail ≈ anchor），将 utc_tail 人为 +1 后再计算。

---

## 最终成功方案 (2026-03-12)

**核心思路**：放弃对 `utc_tail` 绝对值的依赖，改用 SEC 锚点 + 阈值法 + 三遍后处理。

### Pass 1: SEC-anchored 双路径重建

每个事件用最近 SEC 的 `(met, ptime)` 作为锚点。根据 `elapsed = utc_tail - anchor` 选择路径：

#### 正常路径 (elapsed < 1.5×WRAP)
锚点新鲜，至多 1 次 wrap。用阈值法判定：
```rust
if raw_delta < -WRAP_THRESHOLD { raw_delta += PTIME_MOD }
else if raw_delta > PTIME_MOD - WRAP_THRESHOLD { raw_delta -= PTIME_MOD }
```

**优势**：不依赖 utc_tail，免疫 utc_tail 滞后。

#### 过期锚路径 (elapsed ≥ 1.5×WRAP)
FIFO 复位后锚点过期。用 utc_tail 辅助估算 n_wraps，best-of-3 选距 utc_tail 最近的。

**局限**：utc_tail 有偏（反映组装时间），导致部分包高估 +1 wrap。

### Pass 2: fix_wrap_reversals

检测反向跳跃 ≈ -WRAP_PERIOD，用 **gap 判据**区分真假倒退：
- gap > 0.3s → FIFO 复位后真错位 → 整批下移 -WRAP_PERIOD
- gap < 0.3s → 纯文件重排假倒退 → 跳过

### Pass 3: fix_wrap_boundary_dips (新增)

修正 ptime 回绕边界处的错位：

**Step 1 — Dip batch 检测**：
- 扫描正向跳跃 ≈ +WRAP_PERIOD
- 反向查找被 HIGH 包"夹心"的 LOW 批次
- 整批上移 +WRAP_PERIOD，同时修正相邻混合包的 LOW 事件

**Step 2 — 混合包修正**：
- 对 span ≈ WRAP_PERIOD 的包，按邻居层级判定多数/少数簇
- 将少数簇对齐到多数簇 (±WRAP_PERIOD)

### 验证结果

| GRB | Pass 2 触发 | Pass 3 触发 | 逐秒 Δ% |
|-----|-----------|-----------|---------|
| 200415A | 无 | 无 | 0.0% |
| 221009A | 有 (gap判据跳过) | 无 | 0.0% |
| 260226A | 无 | 有 (34包) | 0.0% |

**260226A 修正前后对比**：
- Box A T+19: -5.7% → 0.0%
- Box B T+21/T+24: -4.7%/-8.9% → 0.0%
- Box C T+19/T+24: -5.6%/-8.4% → 0.0%

**关键突破**：
1. 正常路径免疫 utc_tail 滞后
2. Gap 判据区分 FIFO 复位 vs 文件重排
3. 三遍后处理分离不同错位模式，避免相互干扰

详见 [time_reconstruction_algorithm.md](time_reconstruction_algorithm.md)。
