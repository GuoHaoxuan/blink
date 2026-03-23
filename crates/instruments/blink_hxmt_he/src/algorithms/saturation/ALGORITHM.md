# 1B 时间重建算法实现细节

## Step 1: CRC 过滤

对每个 CCSDS 包的 109 个 8 字节 slot：

```
对每个 slot (8 bytes):
    row[0..7] = 8 个字节
    computed_crc = crc_check(row[0..7])
    stored_crc = row[7] & 0x0F

    if computed_crc == stored_crc:
        ptime = ((row[4] & 1) << 18) | (row[5] << 10) | (row[6] << 2) | ((row[7] & 0xC0) >> 6)
        type_bits = row[7] & 0x30

        0x00 或 0x20 → EVT: channel = row[0]
        0x10         → SEC: stime = (row[0]<<24) | (row[1]<<16) | (row[2]<<8) | row[3]
        0x30         → 丢弃
    else:
        → 丢弃
```

输出：`parsed[pkt_idx]` = 该包中通过 CRC 的事件列表。

## Step 2: SEC 验证

目标：从通过 CRC 的 SEC 中区分真 SEC 和幽灵 SEC。

### Phase 1: 相位聚类

对每个 SEC 计算相位：

```
phase = (ptime - stime × 500000) mod 524288
```

原理：FPGA 每秒注入一个 SEC，采样当时的 ptime。ptime 以 2μs/tick 运行，1 秒 = 500000 ticks。所有真 SEC 的相位 ≈ 常数（实测 ±14 ticks）。幽灵 SEC 的相位均匀分布在 [0, 524287]。

排序 + 滑动窗口找最大簇：

```
1. 对所有 SEC 按 phase 排序（排序下标，不移动数据）
2. left = 0
3. for right in 0..n:
     while phase[sorted[right]] - phase[sorted[left]] > 400:
         left++
     if right - left + 1 > best_count:
         best_count = right - left + 1
         best_start = left
4. sorted[best_start .. best_start+best_count] 标记为簇内候选
```

窗口宽度 400：真 SEC 全幅 ≈ 113 ticks，400 远大于此（不漏）。60 个 ghost 落入 400 宽窗口的期望数 = 60 × 400/524288 ≈ 0.05（不误收）。

### Phase 2: stime 升序检查（LIS）

将簇内 SEC 按打包顺序 `(pkt_idx, evt_idx)` 排序。

FIFO 保序 → 打包顺序中 stime 必须严格递增。幽灵 SEC 的 stime 是垃圾值（如 1.5 billion），会打破升序。

用**最长递增子序列（LIS）**找主序列：

```
vals = 簇内 SEC 的 stime 序列（按打包顺序）

Patience sorting 算法（O(n log n)）：
    tails = []      // tails[k] = 长度 k+1 的递增子序列的最小末尾值
    tail_pos = []   // 对应下标
    parent = [None; n]  // 回溯指针

    for i in 0..n:
        pos = tails 中第一个 >= vals[i] 的位置（二分查找）
        if pos == tails.len():
            tails.push(vals[i])   // 延长最长子序列
            tail_pos.push(i)
        else:
            tails[pos] = vals[i]  // 更新为更小的末尾值
            tail_pos[pos] = i
        parent[i] = if pos > 0 { Some(tail_pos[pos-1]) } else { None }

    // 从 tail_pos.last() 回溯，标记 LIS 成员
    idx = tail_pos.last()
    while idx != None:
        in_lis[idx] = true
        idx = parent[idx]

在 LIS 中的 SEC → 有效
不在 LIS 中的 → ghost
```

Phase 1 和 Phase 2 检查不同维度：
- Phase 1：`(stime, ptime)` 的**模运算相位关系**
- Phase 2：`stime` 在打包序中的**单调递增性**

两个检查串联：先用 Phase 1 缩小范围（排除相位不对的 ghost），再用 Phase 2 在簇内用升序排除可能混入的残余 ghost。

## Step 3: 1s-SEC 区间内事件解算

对每对在打包顺序中相邻的有效 SEC，只处理 `Δstime = 1` 的。

### 3a. 计算 actual_advance

```
actual_advance = (sec2.ptime - sec1.ptime) mod 524288
```

两个 SEC 之间 ptime 的真实步进量，约 500000 ± 14 ticks。直接从数据算出，自动包含 SEC 采样抖动，不依赖硬编码常数。

### 3b. 收集候选事件，计算 elapsed_fwd

遍历 SEC1（不含）到 SEC2（不含）之间的所有事件（按打包顺序）：

```
对每个候选事件:
    elapsed_fwd = (event.ptime - sec1.ptime) mod 524288
```

`elapsed_fwd` 把 ptime 从环形空间展开到线性空间 [0, 524287]。性质：
- 真事件：elapsed_fwd ∈ [0, actual_advance]，按打包序严格递增
- ptime 回绕在 elapsed_fwd 空间中被"拉直"，不表现为下降
- Ghost：elapsed_fwd 要么在死区（> actual_advance），要么打破升序

### 3c. Pass 1: 死区过滤

```
if elapsed_fwd > actual_advance:
    → 死区 ghost, alive = false
```

死区 = (actual_advance, 524287]，大小 ≈ 24288 ticks（4.6%）。这些 ptime 不对应两个 SEC 之间的任何时刻。

### 3d. Pass 2: 升序检查（LIS）

对通过死区检查的事件，在 elapsed_fwd 序列上做 LIS（和 Step 2 Phase 2 同样的 patience sorting 算法）。

```
vals = 活事件的 elapsed_fwd 序列（按打包顺序）

Patience sorting → 找 LIS

在 LIS 中 → 真事件
不在 LIS 中 → ghost（打破了 elapsed_fwd 的升序）
```

LIS 的优势（相比正向/反向扫描）：不会被序列开头或结尾的 ghost 毒杀。真事件有几千个形成长升序，ghost 只有几个随机插入，LIS 一定找到正确的主序列。

### 3e. 计算 MET

对每个在 LIS 中的事件，从两个 SEC 分别计算 MET，取平均：

```
elapsed_fwd = (event.ptime - sec1.ptime) mod 524288
elapsed_bwd = (sec2.ptime - event.ptime) mod 524288

met_fwd = sec1.stime + offset + MET_CORRECTION + elapsed_fwd × 2μs
met_bwd = sec2.stime + offset + MET_CORRECTION - elapsed_bwd × 2μs

result = (met_fwd + met_bwd) / 2.0
```

每个事件独立计算，只依赖自身 ptime 和两端 SEC。Ghost 无法影响其他事件。

### 3f. SEC 自身的 MET

```
result[sec.pkt_idx][sec.evt_idx] = sec.stime + offset + MET_CORRECTION
```

## 验证结果（GRB 221009A）

| | Box A | Box B | Box C |
|---|---|---|---|
| CRC 通过 | 17,851,510 | 17,551,542 | 17,394,250 |
| 有效 SEC | 3,554 | 3,556 | 3,553 |
| 幽灵 SEC | 52 | 69 | 58 |
| 1s-SEC 对数 | 3,540 | 3,541 | 3,539 |
| 解算事件 | 17,201,701 | 16,905,596 | 16,713,603 |
| 死区 ghost | 98 | 99 | 97 |
| 升序 ghost (LIS) | 11 | 24 | 4 |
| 覆盖率 | 96.4% | 96.3% | 96.1% |
| 未覆盖 (NaN) | 646,255 | 642,390 | 677,094 |

## 未处理

SEC gap > 1s 的区间（约 3.7% 的事件）仍为 NaN。对应：
- 饱和期间 SEC 丢失（gap 2~3s）
- MCU 崩溃间隙（gap 8~20s）
