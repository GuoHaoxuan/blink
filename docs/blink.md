# blink CLI 使用文档

HXMT HE 饱和分析与 TGF 探测命令行工具。本文档覆盖 `sat` 子命令树
(饱和分析), `search` (TGF 扫描) 和 `filter` (闪电关联) 是独立栈,
本文不涉及。

## 构建与运行环境

```bash
cargo build -p blink --release
./target/release/blink sat <COMMAND>
```

CLI 通过两个环境变量定位数据:

| 变量 | 默认值 | 说明 |
|---|---|---|
| `HXMT_1B_DIR` | `/hxmtfs/data/Archive_tmp/1B` | 1B 原始遥测归档根目录 |
| `HXMT_1K_DIR` | `/hxmt/work/HXMT-DATA/1K` | 1K 标准管线产品根目录 |

服务器上默认值即为真实路径。本地开发需 export:

```bash
export HXMT_1B_DIR=data/1B HXMT_1K_DIR=data/1K
```

目录结构:
- 1B: `<root>/<YYYY>/<YYYYMMDD>/<port>/HXMT_1B_<port>_<YYYYMMDD>T<HH>0000_*.fits`
- 1K: `<root>/Y<YYYYMM>/<YYYYMMDD>-<NNNN>/HXMT_<YYYYMMDD>T<HH>_<type>_FFFFFF_V*_1K.FITS`

Box 端口映射: A=0642/0766, B=0922/1009, C=1686/1781 (Sci/Eng).

## 时间格式

TRIGGER 接受两种格式:

- **MET (Mission Elapsed Time)**: 数字, 例如 `454965169.900` —
  从 2012-01-01T00:00:00 UTC 起的 SI 秒
- **UTC 字符串**: `YYYY-MM-DDTHH:MM:SS.f`, 例如 `2026-06-01T19:12:49.900`

EPOCH 格式: `YYYY-MM-DDTHH` (标识 1B 归档的小时片)。Burst-centric
命令的 EPOCH 从 TRIGGER 自动推导, 不需要单独指定。

## 命令总览

```
blink sat <COMMAND>

  # 单 burst (TRIGGER 必填, 1B 小时从 TRIGGER 推导)
  report <TRIGGER> --before <s> --after <s> -o <DIR>
  detect <TRIGGER> --before <s> --after <s> [--box a|b|c]
  reconstruct <TRIGGER> --before <s> --after <s> [--box a|b|c] [--bin <s>]
  extract <TRIGGER> --before <s> --after <s> [--box a|b|c] [--source 1b|1k]
  compare <TRIGGER> --before <s> --after <s> [--box a|b|c]
                    [--coarse-bin <s>] [--fine-bin <s>] [--csv]

  # 离线扫描 (无 TRIGGER)
  scan --epoch <YYYY-MM-DDTHH> [--box a|b|c]

  # 低层 debug
  dump <SUB> --epoch <YYYY-MM-DDTHH> ...
    times | packets | events | hist | diag    # 窗口型
    ptime | check-offset                       # 包索引范围型
```

`--before` / `--after` 对 burst-centric 命令是**必填**, 没有默认值
(错的窗口会静默丢事件)。

## 子命令详解

### `report` — 完整诊断 pack

为单个 burst 一次性产出可被 Python 直接消费的数据 pack, 用于 burst
饱和诊断 / 论文图表 / 跨仪器对比基准。

```
sat report <TRIGGER> --before <s> --after <s> -o <DIR>
```

**输出**: 在 `<DIR>` 下生成

```
manifest.json                  # T0_MET, T0_UTC, 窗口, 1B/1K FITS 路径, summary
box_a/  events_obs.csv         # 1B 观测事件 (含 det_id / channel / aminfo / pulinfo)
        events_rec.csv         # gap-fill 重建事件 (met only)
        events_1k.csv          # 1K 管线参考 (met, channel, det 0..5 box-local)
        resets.csv             # FIFO reset (start, stop, gap_s, pkt, n_lost, cluster_id)
box_b/  ...
box_c/  ...
```

`cluster_id` 把同 1 秒内的连续 reset 归一组, 便于下游缩放图。

**配套画图脚本**: `scripts/plot_burst_report.py --pack <DIR> -o <PNG>`
或一步到位: `scripts/diag_burst.sh <TRIGGER> --before <s> --after <s> -o <PNG>`

**例**:
```bash
./target/release/blink sat report 2026-06-01T19:12:49.900 \
    --before 50 --after 350 -o /tmp/burst_pack
python3 scripts/plot_burst_report.py --pack /tmp/burst_pack -o plots/burst.png
```

### `detect` — FIFO reset 列表

```
sat detect <TRIGGER> --before <s> --after <s> [--box a|b|c]
```

**输出 (stdout, CSV)**: `box,type,start_met,stop_met,gap_s,pkt_idx,evt_idx,n_lost,log10p`

`n_lost` 来自 cross-box shape-based 估计 (该 reset 的 ref 率 × gap)。
`log10p` 当前固定 0 (silent drop 检测已移除)。

### `reconstruct` — gap-fill 光变曲线

```
sat reconstruct <TRIGGER> --before <s> --after <s> [--box a|b|c] [--bin <s>]
```

**输出 (stdout, CSV)**: `box,type,met,channel,pkt_idx,evt_idx`,
其中 `type` 为 `EVT` (观测) 或 `FILL_GAP` (重建)。

`--bin` 仅影响诊断日志输出, 不影响事件级别 CSV。

### `extract` — 逐事件 dump

```
sat extract <TRIGGER> --before <s> --after <s> [--box a|b|c] [--source 1b|1k]
```

`--source` 默认 `1b` (1B 原始 + MET 重建), 可设 `1k` (1K 管线导出)。

**输出 (stdout, CSV)**: 两种 source 列略不同
- 1b: `box,type,met,channel,det_id,pkt_idx,evt_idx,aminfo,pulinfo` (det_id 0..5, box-local; aminfo 是 18-bit ACD shield mark; pulinfo 脉宽 µs = pulinfo/48)
- 1k: `box,type,met,channel,det_id` (det_id 0..17, **全局** — 跟 1B 不一致, 是 1K Det_ID 直接字段)

### `compare` — 1B vs 1K 互校

```
sat compare <TRIGGER> --before <s> --after <s> [--box a|b|c]
            [--coarse-bin <s>] [--fine-bin <s>] [--max-lag <ms>] [--csv]
```

默认 `--coarse-bin=1.0` `--fine-bin=0.1` `--max-lag=50`。

**默认输出 (人类可读)**: 粗 bin 表 + 显著 (|delta| > 阈值) 的细 bin +
每秒 1 ms cross-correlation 偏移。

**`--csv`**: 一份扁平表, `box,t_rel,n_1k,n_1b,delta,delta_pct,bin_type`,
其中 `bin_type` ∈ `coarse | fine | cc`。

### `scan` — 整小时离线扫描

```
sat scan --epoch <YYYY-MM-DDTHH> [--box a|b|c]
```

格式与 `detect` 相同, 但覆盖整个小时, 用于扫一段时间是否有候选饱和事件。

### `dump <SUB>` — 低层诊断

所有 dump 子命令需显式 `--epoch <YYYY-MM-DDTHH>` (没有 TRIGGER 自动推导)。

#### `dump times`
逐事件 MET 时间 + 该小时所有饱和区间。用于事件级时序检查。

#### `dump packets`
每个 CCSDS 包的 (min_time, max_time, n_events) + 整数秒事件
(SEC 事件) 列表。用于 packet-边界诊断。

#### `dump events`
完整 8 字节 raw + 解码后字段 (channel/MET/SEC 标记/原始字节)。

#### `dump hist`
事件柱状图 + 饱和区间。
```
sat dump hist --epoch <E> <TRIGGER> --before <s> --after <s> [--bin <s>]
```
默认 bin 10 ms。

#### `dump diag`
每个 packet 的统计: `n_evt, n_sec, n_err, n_dropped, anchor 有无,
UTC_tail, MET min/max`。
**默认仅打印有错/丢的 packet**, 全部打印需 `DUMP_ALL=1` env var。

#### `dump ptime`
ptime ↔ UTC ↔ MET 三元映射, 用于 packet 内部时间一致性 debug。
```
sat dump ptime --epoch <E> <pkt_min> <pkt_max> [--box a|b|c]
```

#### `dump check-offset`
CRC 字节偏移验证。
```
sat dump check-offset --epoch <E> <pkt_min> <pkt_max> [--box a|b|c]
```

## 常见工作流

### 单 burst 完整诊断 (推荐)

```bash
./scripts/diag_burst.sh 2026-06-01T19:12:49.900 \
    --before 50 --after 350 -o plots/burst_0601.png
```

内部就是 `sat report` → `plot_burst_report.py`。pack 默认临时, 加
`--keep-pack <DIR>` 保留。

服务器跑完一条 rsync 把 PNG 回本地最省事。

### 批量验证 (paper figures)

`scripts/freeze_numbers.sh` 已经用新 CLI 重新接通, 跑这个会重生成
论文里所有依赖于 1B 的数字。

### 跨仪器对照

`scripts/plot_hxmt_vs_{gbm,gecam,spiacs}.py` 内部调 `sat reconstruct`
拿 1B 重建光变, 跟 Fermi GBM / GECAM-C / INTEGRAL SPI-ACS 的独立观测
对照。详见 `crates/instruments/blink_hxmt_he/src/algorithms/saturation/DESIGN.md`
里的"跨卫星验证"小节。

## 已知约束

- **跨小时窗口**: TRIGGER 落在 19:59 + `--after 120` 会跨到 20:01, 此时
  CLI 只加载 19 这个小时, 会 warning 但不报错。需要手动跑两个小时再合。
- **B-only 局部饱和**: cross-box 重建依赖至少另一个 box 也看到 burst。
  当饱和只发生在一个 box (例如 2026-06-01 burst 在 B 上的局部突发),
  cross-box k calibration 用 baseline, n_lost 估计可能偏低。无 ground
  truth 验证。详见 `docs/superpowers/specs/` 或 commit 4afc73e 之前的
  讨论。
- **`--source 1k` det_id 不归一化**: 1B 的 det_id 是 0..5 box-local
  (FPGA 编码), 1K 的 det_id 是 0..17 全局。`extract` 输出按原状,
  `report` pack 里的 `events_1k.csv` 则归一化为 0..5 跟 1B 对齐。
