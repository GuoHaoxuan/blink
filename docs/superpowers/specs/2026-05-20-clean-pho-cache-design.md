# 干净 PHO 验证数据集 (2020-H1)

**日期**: 2026-05-20
**范围**: 从 `per_sec_parquet/` 抽取 2020-01-01 至 2020-06-30 的子集，过滤后输出**单个 parquet 文件**（仅原始计数，所有归一化下游处理），用于验证更简洁系数的 PHO 重建模型。旧的 `cache_training.py` / `partition_cache.py` / `train_cache.parquet` / `perdet_npz/` 全部弃用——本方案替代。

## 动机

旧 PHO 模型 `PHO ≈ b + c_pure·Sci_pure + c_ACD·Sci_ACD + β·Wide + γ·Large`（5 个系数，per `(box, det)` 拟合）疑似过拟合——相对底层物理而言自由参数太多。我们想在一个**严格筛选过的小子集**上检验更简的模型形态（3 系数、2 系数、固定比例变体），用尽量低的杂质方差暴露系数本质。

两个核心原则：

1. **短时间跨度**（6 个月）规避年级参数漂移：PMT 漏气、HV setpoint 调整、增益漂移
2. **干净的空间与时间**——赤道带（避开辐射带与高纬度带电粒子轰击）、SAA 几何排除、GBM 触发秒排除

时间窗内，本 cache 保留所有可能用到的工程计数列，**列的选取由分析灵活度驱动，而非锁定到某一具体模型形态**——避免过早把建模决策烧进 cache。

## 输入

| 数据源 | 服务器位置 | 用途 |
|---|---|---|
| 每秒 parquet | `/scratchfs/gecam/guohx/blink/per_sec_parquet/YYYYMMDD.parquet` | 每 `(date, box, det, met_sec)` 一行的原始记录 |
| GBM trigger catalog | 从 HEASARC fetch（`https://heasarc.gsfc.nasa.gov/W3Browse/fermi/fermigtrig.html`，表名 `fermigtrig`）→ 本地缓存到 `n_below_study/gbm_triggers.parquet` | 爆发秒排除 |

## 时间窗

**2020-01-01T00:00:00 UTC ≤ date ≤ 2020-06-30T23:59:59 UTC**（端点含，共 182 天）

## 过滤链

按顺序应用，每阶段后记录剩余行数。

### Stage 1 — 探测器工况

- `L_cycles > 50_000`（实活时间 > 0.8 s；剔除秒边界的低活时间样本）
- `HV ∈ (-1100, -900)`（探测器高压正常工作区；超出代表正在升降压或停机）

### Stage 2 — 数据完整性

- `HV`, `Lat`, `Lon` 非 NaN（轨道/HV 缺片在抽取后会以 NaN 出现；昨天 QA 看到某些日期 NaN 率 ~4%）
- 所有 raw counter `≥ 0`：`PHO`, `OOC`, `Wide`, `Large`, `Dt`, `Sci_094`, `Sci_pure_094`, `Sci_ACD1_094`, `Sci_ACDN_094`, `Sci_1s`, `Sci_pure_1s`, `Sci_ACD1_1s`, `Sci_ACDN_1s`（出现负数 = 抽取 bug）
- 分解不变式：`Sci_pure_{w} + Sci_ACD1_{w} + Sci_ACDN_{w} == Sci_{w}`，两个窗 `w ∈ {094, 1s}` 都验

### Stage 3 — 空间过滤

- `|Lat| < 3.0°`（赤道带，避开辐射带和高纬带电粒子轰击）
- `NOT (Lon ∈ [-90°, +30°])`（SAA 几何排除；SAA 中心框约 `Lon ∈ [-90°, +30°], Lat ∈ [-50°, +5°]`，纬度那侧已经被上面 `|Lat| < 3°` 涵盖，所以只剩 Lon 谓词）

### Stage 4 — 爆发过滤

对每个 `met_sec = T` 的行：

- 若 `∃ trigger ∈ GBM` 满足 `|T − trigger.met_sec| ≤ 300`（GBM 任一触发的 ±5 min 内），则剔除

GBM trigger 时间从 GBM MET（自 2001-01-01 TT 计秒）转换到 HXMT MET（自 2012-01-01 UTC 计秒）。catalog 一次性 fetch、parquet 缓存、内存里维持一个升序 `int64` 数组用 `np.searchsorted` 查询。

**故意不使用 HXMT 自己的 `tgfs.json`**——按用户判断该 catalog 有质量问题，不作为本次爆发排除的来源。

### Stage 5 — 跨探测器完整性

按 `(date, box, met_sec)` 分组：经过 Stage 1–4 后，**6 个探测器全在**才保留该秒；再按 `(date, met_sec)` 分组：**3 个 box 全在**才保留。

理由：下游模型拟合会用到 box 级 / group 级求和（如 `group_rate = sci_sec_total / livetime`）；缺探头/缺 box 会让这些和被静默低估。

## 派生列

**无**——cache 只存原始计数。所有归一化（速率、死时间修正、ACD 求和）一律下游处理。

**下游约定**（脚本里 inline 计算即可，**所有工程速率使用 per-row length，不要 hardcode 0.94**）：

| 量 | 公式 | 说明 |
|------|------|------|
| `length` | `L_cycles × 16e-6` | 工程周期 wallclock 时长（per row，≈ 0.94s 标称但实际波动 ±0.7%），**不是 livetime** |
| `dt_frac` | `Dt / L_cycles` | 周期内死时间占比 |
| `live_frac` | `1 - dt_frac` | 活时间占比 |
| `pho_rate`（events / 1s wallclock） | `PHO / length` | OOC/Wide/Large 同理用 `length` |
| `sci_rate_094` | `Sci_094 / length` | 0.94s 事件窗口（跟工程周期同窗口对齐） |
| `sci_rate_1s` | `Sci_1s / 1.0 = Sci_1s` | 1s 事件窗口是 extract spec 硬编码，**不**用 length |
| `Sci_ACD_*` | `Sci_ACD1_* + Sci_ACDN_*` | per-window 求和 |

**死时间影响**（per A1005 + 用户领域知识）：
- **PHO / Large** 是前端 trigger 计数，**不受死时间影响**——每次触发都计
- **Wide / Sci** 是事件机产物，**受死时间影响**——死时间内丢失事件

→ 对比时若想统一到 eventizer-visible 计数，把 PHO/Large 乘以 `live_frac`，Wide/Sci 维持原值。

## 输出

**路径**: `n_below_study/clean_2020H1.parquet`（zstd 压缩）

**粒度**: 一行 = 一个 `(date, box, det, met_sec)`，与输入一致

**实际大小**: **261 MB 磁盘**（zstd 后），~3 GB 内存（pandas DataFrame，包含完整姿态/轨道 passthrough）

### 列单（48 列，全部来自 extract，cache 不再添加任何派生列）

| 分组 | 列 | 含义 |
|---|---|---|
| 身份 (4) | `date`, `box`, `det`, `met_sec` | date string, box A/B/C, det 0-5, int64 MET |
| 包头/sanity (6) | `time_float`, `crc_box`, `utc_last_bdc`, `stime_last_bdc`, `error_code`, `bus_time_bdc` | CCSDS 包头字段，故障排查用 |
| 工程计数器（0.94s 周期，6） | `L_cycles`, `PHO`, `OOC`, `Wide`, `Large`, `Dt` (int32) | PDAU 47×20ms 周期累计 |
| 探测器状态（瞬时，1） | `HV` (float32) | 1K HE-HV @ met_sec 采样 |
| 事件 0.94s 窗口 (4) | `Sci_094`, `Sci_pure_094`, `Sci_ACD1_094`, `Sci_ACDN_094` (int32) | 与工程周期同窗口 |
| 事件 1.0s 窗口 (4) | `Sci_1s`, `Sci_pure_1s`, `Sci_ACD1_1s`, `Sci_ACDN_1s` (int32) | 比工程周期多 60ms 的事例 |
| 轨道（瞬时，9） | `X`, `Y`, `Z`, `Vx`, `Vy`, `Vz`, `Lon`, `Lat`, `Alt` | 1K Orbit @ met_sec 采样 |
| 姿态（瞬时，14） | `Ra`, `Dec`, `Delta_Ra`, `Delta_Dec`, `Delta`, `Euler_Phi`, `Euler_Theta`, `Euler_Psi`, `Q1`, `Q2`, `Q3`, `Omega_X`, `Omega_Y`, `Omega_Z` | 1K Att @ met_sec 采样 |

共 **48 列**。Filter 链只用 identity + Lat + Lon + L_cycles + HV + 工程/事件计数 这少数几个，其余字段全 passthrough，方便下游做姿态相关分析。

## 实现

**单脚本**: `scripts/build_clean_cache.py`

**模块**（在同一文件内）：

| 模块 | 职责 | 代码量 |
|---|---|---|
| `BurstCatalog`（class） | 没缓存时从 HEASARC fetch；暴露排序后的 MET `int64` 数组；提供 `any_within(met_sec_array, ±300s) -> bool[]` 通过 `np.searchsorted` 实现 | ~80 |
| `apply_filters(df, catalog) -> df` | 顺序应用 Stage 1–5；每阶段后日志记录行数 | ~120 |
| `process_one_day(date_str, out_dir) -> Path` | 加载日 parquet → 过滤 → 写 `partial/{date}.parquet`（不做派生） | ~30 |
| `main()` | `multiprocessing.Pool(processes=8)` 遍历 182 天，concat 所有 partials，写最终 parquet，跑断言 | ~60 |

**依赖**: `pandas`, `pyarrow`, `numpy`, `requests`（HEASARC fetch）, `astropy.time`（MET 时间转换）

**单 hlogin 节点跑，不上 hep_sub**——8 路本地并行通过 `multiprocessing.Pool`，瓶颈在 NFS。预计 5–10 min。

## 错误处理

| 失败模式 | 行为 |
|---|---|
| 某日 parquet 缺失 | 跳过、log warning，继续下一天 |
| GBM catalog fetch 失败 | 硬失败，整个流程终止（不允许 silent fallback） |
| 过滤链导致某天剩 0 行 | 跳过该天的 partial 写入、log info |
| 单个 worker 进程崩 | `multiprocessing` Pool 上报异常 → main 终止；现存 partial 文件保留供检查 |
| 末段断言失败 | 输出文件已写，但 log 高声报错让用户看见 |

**原子性**: 先写到 `n_below_study/clean_2020H1.parquet.tmp`，成功后 `os.rename` 到最终路径。

## 末段断言

脚本在写完后立即检查、任一失败即 exit non-zero：

1. `len(df) >= 1_000_000` — 保守下限（~50 min/天 × 60 × 182 × 18 × ~0.5 留存率 ≈ 5M 期望；1M 是底线）
2. `(df["Lat"].abs() < 3.0).all()` — Stage 3 强制
3. `(~((df["Lon"] >= -90) & (df["Lon"] <= 30))).all()` — Stage 3 强制（所有保留行的 Lon 都在 SAA 框外）
4. 对每个 `w ∈ ["094", "1s"]`：`(df[f"Sci_pure_{w}"] + df[f"Sci_ACD1_{w}"] + df[f"Sci_ACDN_{w}"] == df[f"Sci_{w}"]).all()`

## Smoke 测试流程

正式 182 天批跑前：

1. 单独跑 `process_one_day("20200115")`——选个 mid-window 平淡的一天
2. 打印每个 filter stage 后的行数
3. 打印每个 raw 列的 dtype / NaN 数 / 取值范围
4. 看上去有异常就先调过滤阈值再上全量

## 不在本方案范围内

- **模型拟合本身**——本 spec 只产 cache；PHO 模型验证（具体测哪些更简模型、RMS 比较、绘图）是下游另一个任务
- **其他时间窗**——本次只搞 2020-H1。同一脚本理论上 CLI 改个日期即可重定向到别的窗口，但其他窗口不在本方案的验证范围
- **GECAM catalog**——无关（2020-12 才发射）
- **HXMT 自家 `tgfs.json`**——故意排除（用户对其质量有顾虑）
- **per-(box, det) NPZ 拆分**——旧 `perdet_npz/` 弃用；单 parquet 秒级加载，没必要预拆
