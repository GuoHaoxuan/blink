# SCPMA 投稿 Design：HXMT/HE 1B 饱和恢复

**日期**：2026-05-06
**目标**：将 HXMT/HE 1B 饱和恢复工作投稿至 Science China Physics, Mechanics & Astronomy (SCPMA)
**作者团队**：第一作者郭昊轩，corresponding 导师，作者列表覆盖全 HXMT 团队
**核心约束**：工程数据 validation 完成（~2 周）后立即进入 11 天集中写作期，21 天内投稿

---

## 1. 叙事框架

### 1.1 重心 pivot
- **Headline**：饱和修复（saturation recovery）—— 从 1B 原始数据取回 1K 管线丢失的事件
- **Prerequisite，不是 headline**：1B 事件级时间重建（基于 LIS 算法）
- **理由**：1K 在非饱和区的时间解算已经达到良好水平；论文不能给 reviewer "和 1K 比时间精度"的错觉，否则会被质疑 motivation

### 1.2 Hook（intro 第一段思路）

> Insight-HXMT/HE 是在轨最大有效面积（~5100 cm²）的硬 X 射线探测器之一，但其 FIFO 缓冲区 + 8051 MCU 读出架构在极亮源观测中会饱和——FPGA 端写入被阻塞，事件被静默丢弃。HXMT 标准 1K 数据管线对饱和区域采取保守剔除策略，可能丢失原本可恢复的事件。本工作发展了一套从底层 1B 原始数据出发的事件级时间重建与光变曲线恢复方法，针对中度饱和 GRB 实现近无损恢复，并系统量化了方法在极端饱和（如 GRB 221009A）下的适用边界。

### 1.3 论文 4 条卖点

1. 基于 LIS 的事件级时间重建，对 CRC 幽灵内在鲁棒（无人工阈值/迭代）
2. 分组 LIS + UTC tail 约束的 wrap 消歧，在 SEC 间隙 ≤ 10 s 时唯一性可证
3. 三机箱独立通路 + HE 工程数据形成的多层验证体系
4. 量化 method 失效边界：**(i)** 时标过短（亚毫秒峰，gap 重建固有 1 ms 分辨率）；**(ii)** 通量过强（三 box 同饱和的 ~8% gap、>10 s SEC 间隙、SEE-induced data corruption）

> **注**：4.6% ptime dead zone（ptime 回绕周期 1.0486 s 与整秒的 48.6 ms 偏移构成的 ptime 空间区间）**不是失效边界**——真实事件不应落在此区间，落入此区间的"事件"是 CRC 碰撞幽灵，算法正确排除。dead zone 是 *算法 safeguard*，在 §3 算法描述时简短交代即可，不进 Discussion 失效讨论。

### 1.4 科学价值（一句话，不展开）

> 为后续光谱演化和亚秒级时变分析提供数据基础。

不展开 221009A 物理（涉及其他合作组的 ongoing work，越界）。

---

## 2. Title 与 Abstract

### 2.1 Title（已锁定）

> **Saturation Recovery for Insight-HXMT/HE Bright Burst Observations from Level-1B Raw Data**

### 2.2 Abstract 结构（草拟，~200 词，最终需以 W -1 数字定版后的真实数字打磨）

```
[Background, 2 句]
The Hard X-ray Modulation Telescope (Insight-HXMT) High-Energy detector
(HE), with ~5100 cm² effective area in 20–250 keV, is uniquely sensitive
to bright transients but suffers from FIFO buffer saturation that drops
events without hardware flags, masked by the conservative event filtering
of the standard 1K pipeline.

[Method, 3 句 — 重 saturation recovery，LIS 一句话带过]
We develop a saturation-recovery pipeline operating on raw 1B telemetry.
Lost events are recovered by detecting FIFO-reset gaps from packet-level
timing anomalies and reconstructing them via cross-detector-box shape
functions calibrated against simultaneous unsaturated boxes. Recovery
rests on event-level time reconstruction in 1B data, which we solve with
a longest-increasing-subsequence formulation that is intrinsically
robust to CRC-collision ghost events.

[Results, 3 句 含具体数字 — 数字 W -1 重跑后定版]
On GRB 260226A (moderate saturation), our recovered light curve agrees
with the 1K pipeline to within 3 events out of 549,661 (~5×10⁻⁴%).
Cross-validation against HE's own engineering-data counters and against
Fermi/GBM (260226A), GECAM-C (221009A), and INTEGRAL/SPI-ACS (200415A
magnetar giant flare) confirms the recovered light curves at the few-
percent level. On GRB 221009A (extreme saturation), the method recovers
~96% of CRC-passed events; the remaining ~4% lies in regions where all
three detector boxes are simultaneously saturated, demarcating the
method's limit.

[Implication, 1 句]
The pipeline enables higher-fidelity bright-burst analyses with HE data
and is released as an open-source Rust tool with Zenodo DOI.
```

---

## 3. 章节大纲（SCPMA Article 双栏 ~13 页）

| 节 | 标题 | 篇幅 | 内容要点 |
|---|---|---|---|
| 1 | Introduction | ~1.5 p | (1.1) HE 在亮源观测中的独特性（5100 cm² 大面积 → 容易饱和的 trade-off 是核心动机）；(1.2) FIFO 饱和机制 + 1K 管线保守剔除策略；(1.3) 1B 数据的机会 + 两个核心挑战（ptime 1.05 s 回绕、4-bit CRC 碰撞）；(1.4) 本文贡献 4 条 |
| 2 | HE Data System and Saturation Mechanisms | ~1.5 p | (2.1) 硬件数据通路 / FIFO + 8051 MCU；(2.2) 包格式 / CRC / ptime / SEC；(2.3) FIFO 复位 vs 静默丢数；(2.4) 1B vs 1K |
| 3 | Event-Level Time Reconstruction | ~2.5 p | **开头 framing："Time reconstruction in 1B is the prerequisite for saturation recovery"**。(3.1) 问题形式化（n 是唯一未知量）；(3.2) SEC 双阶段验证；(3.3) Δstime=1 直接 LIS（含 LIS vs 贪心级联失效）；(3.4) Δstime>1 分组 LIS（含降序处理证明、UTC 约束、wrap 唯一性 skip-wrap-0 + 跨 box 双重验证） |
| 4 | FIFO Reset Detection and Light Curve Recovery | ~2.0 p | **开头 framing："With event times reconstructed, we address the headline problem"**。(4.1) 自适应 gap 阈值；(4.2) 不可信区间标记（FIFO reset / 拥塞宽包 / 包内异常）；(4.3) 跨 box 形状函数 + 校准系数 k 自洽（form & N_lost 同源）；(4.4) 三 box 同饱和的退化处理 |
| 5 | Validation | ~3.0 p | (5.1) 内部一致性 1B vs 1K（GRB 260226A 残差 3 events）；(5.2) **HE 工程数据计数器 cross-check ★Headline**——独立通路、不受 FIFO 饱和影响、同仪器；(5.3) 跨卫星：(5.3.1) 时间系统处理 (TT/UTC/闰秒/光行差符号) / (5.3.2) GBM 260226A / (5.3.3) SPI-ACS 200415A 磁星 / (5.3.4) GECAM-C 221009A；(5.4) 跨 box 互相关唯一性（GRB 221009A T+249–268） |
| 6 | Discussion: Limits and Failure Modes | ~1.5 p | (6.1) 太亮（三 box 同饱和 ~8% / >10 s SEC 间隙 / SEE corruption）；(6.2) 太短（亚毫秒峰 / 1 ms gap 重建分辨率上限）；(6.3) 静默丢数（机制真实但不可检测、贡献可忽略）；(6.4) CRC 失败事件不回收。<br>**Dead zone 不进 Discussion**——在 §3.3 Δstime=1 算法描述时作为"valid range [0, a]"的 boundary safeguard 一句话带过 |
| 7 | Conclusion | ~0.5 p | 4 点结论 + 开源软件 |
| – | References | ~1 p | ~25-30 条 |

---

## 4. Figure 与 Table 预算

### 4.1 Figures（8 张）

| # | 图 | 内容 | 状态 |
|---|---|---|---|
| F1 | Data path + saturation mechanism | 单图，detector → ASIC → FPGA → FIFO A → MCU 链 + FIFO 复位 / 静默丢数示意 | 改造 `fig_datapath.png` |
| F2 | Pipeline overview + SEC validation | 上下两栏，上=三步流程图，下=相位聚类 | 合并 `fig_pipeline.png` + `fig_phase_cluster.png` |
| F3 | LIS vs greedy cascade failure | 两栏，左贪心崩溃 / 右 LIS 正确 | 合并 `fig07_cascade.png` + `fig01_crc_ghost.png` |
| F4 | Δstime>1 grouped LIS | 三栏：(a) candidates 网格 / (b) descending 处理示意 / (c) UTC 剪枝效果 | 合并 `fig08_candidates.png` + `fig09_descending.png` + `fig_utc_constraint.png` |
| F5 | Cross-box light curve reconstruction | GRB 221009A FIFO reset 周围多 box 光变 + 填充事件 | 用 `ac14623` commit 的多 box visualization |
| **F6** | **HE engineering data validation** ★Headline | 工程数据 vs 1B 重建光变（GRB 260226A，三 box）+ ratio 面板 | **新做（W -2/-1 完成）** |
| F7 | Cross-satellite validation 三联图 | (a) HXMT vs GBM 260226A / (b) HXMT vs SPI-ACS 200415A / (c) HXMT vs GECAM 221009A | 合并三个已有 plot script 输出 |
| F8 | Cross-box cross-correlation 唯一性 | GRB 221009A T+249–268 wrap 0 vs wrap +1 残差对比 | 用 `fig13_crossbox.png` |

### 4.2 Tables（4 张）

| # | 表 | 内容 | 状态 |
|---|---|---|---|
| T1 | UTC tail 约束剪枝效果（ds=19 时不同 utc_tail 下的 max wrap）| 已有 |
| T2 | GRB 221009A 三 box 重建统计（CRC 通过 / 有效 SEC / 解算事件 / 覆盖率）| 已有，**数字需重跑** |
| T3 | 跨 box 参考可用率（92% / 8%）| 已有，**数字需重跑** |
| T4 | 跨卫星验证 summary（暴 / 仪器 / bin / ratio / 光行差）| 新做，从 DESIGN.md 整理 |

---

## 5. 内容缺口清单

main.tex 当前状态到投稿之间需补的工作（按重要性排）：

### 5.1 数字定版（writing W +0 之前必须做完）★

- 跑当前 `blink_cli` 在 GRB 260226A、200415A、221009A 上重新生成所有 metric
- 输出 single source-of-truth：`paper/numbers.csv`（含 commit hash）
- 涉及：abstract 数字、T2-T3 表格、Section 5.1 残差是否还成立、96% 覆盖率是否变了

### 5.2 全新内容（main.tex 完全没有，从 0 写）

| 节 | 内容 | 来源 |
|---|---|---|
| Sec 5.2 | **HE 工程数据 validation 整节** | 工程数据 work + scan_eng_k.py 输出 |
| Sec 5.3.3 | GRB 200415A × SPI-ACS（含磁星巨耀斑、INTEGRAL 高椭圆轨道光行差 −406.6 ms） | DESIGN.md 第 504-538 行 |
| Sec 6.1 扩展 | "Too bright" 失效场景 + 量化（三 box 同饱和 ~8% / SEC 间隙 >10s 占比 / SEE）| 需要新跑 statistic |
| Sec 6.2 扩展 | "Too short" 失效场景 + ASIM/MXGS 1ms 峰值 ratio ~0.10 当反例 | DESIGN.md 第 449 行 |
| Sec 3.3 短句 | Dead zone 作为 [0, a] valid range 的 boundary safeguard 简短交代（一两句），明确 dead zone 事件 = CRC 幽灵、算法正确排除、非方法 limitation | DESIGN.md 第 124 行 |

### 5.3 现有内容需重写

| 节 | 修订 |
|---|---|
| Sec 4 整节 | 从 1 页扩到 2 页，把跨 box 形状函数的 k 校准、空 bin 插值、N_lost 自洽性讲透 |
| Sec 5.3.1 | 时间系统处理（TT/UTC、闰秒、光行差符号——memory 提醒"曾两次搞反"）；DESIGN.md 第 411-438 行有规范 |
| Sec 6 整体 | 从"我们承认局限"改为"我们量化了适用边界" |

### 5.4 Bibliography 扩充（现 3 条 → 目标 ~25-30 条）

由 AI 从 ADS 挖，草稿后给团队过漏掉的：

| 类别 | 关键 references |
|---|---|
| HXMT mission/HE | Zhang 2020 ✓、Chen 2020 mission paper、Liu 2020 HE technical paper |
| GRB 221009A | Burns+ 2023 (LHAASO contemporaneous)、Frederiks+ 2023 (Konus)、An+ 2023 ✓ |
| GRB 200415A 磁星 | Roberts+ 2021、Svinkin+ 2021 |
| Fermi/GBM | Meegan 2009 仪器论文 |
| GECAM | Li+ 2022 仪器论文 |
| INTEGRAL/SPI-ACS | Vedrenne+ 2003、Savchenko+ 2017 (calibration) |
| LIS 算法 | Schensted 1961、Aldous & Diaconis 1999 (patience sorting) |
| 4-bit CRC | Koopman & Chakravarty 2004 |

### 5.5 LaTeX 模板切换

- 现 `revtex4-2` (PRD) → SCPMA 官方模板 `scpma.cls`
- 重做 `\author / \affiliation / \abstract` 宏；section 编号、图表浮动、bib 风格全 migrate
- 工作量：~半天

### 5.6 投稿配套

- Cover letter（半页 + reviewer 推荐 3-5 名）
- Code & Data Availability Statement
- ORCID、corresponding author（导师）

---

## 6. 作者 / 时间表 / 开源

### 6.1 作者列表（结构占位，最终由作者团队定）

```
First author:    郭昊轩         (主要工作执行者)
Corresponding:   导师           (按 IHEP 惯例)
Other authors:   HE 望远镜 PI / 核心团队
                 HE 电子学工程师（Sec 2 硬件细节的 reviewer）
                 HXMT 总体团队代表
                 + 实际 contribute 的合作者
```

写作期收 ORCID 表，每人一份。

### 6.2 时间表（21 天）

倒推自"工程数据 done = W +0" 投稿即开始：

```
W -2 ┐ 工程数据收尾（用户主战场）
W -1 ├ 我的并行工作:
     │   - 数字定版（跑 blink_cli + 生成 numbers.csv，commit hash 锁定）
     │   - 写 Section 1-3 英文初稿 (intro / instrument / time recon)
     │   - 准备 F1, F2, F3, F4, F8 (素材已有的图)
     │   - bibliography 初稿（~25-30 条）
     │
W +0 ┐ "投稿倒计时"开始
 D 1-3:  写 Section 4 (Saturation Recovery) — headline 章节
         做 F6 (engineering data validation 图)
 D 4-5:  写 Section 5 (Validation) 全节
         合成 F7 (cross-satellite 三联图)
 D 6-7:  写 Section 6 (Discussion) — 含 too short / too bright / 静默丢数 / CRC
         写 Section 7 (Conclusion)
 D 8:    全文通读，统一术语 (LIS / FIFO / SEC / etc.)
         切换 LaTeX 模板 RevTeX → SCPMA
 D 9:    导师 + 1-2 关键合作者快速 review
 D 10:   Cover letter + 投稿配套材料
 D 11:   提交（含 buffer 0.5 天处理 LaTeX 编译问题）
```

**严格 schedule**。如工程数据 W -1 没 done，整个 W +N 顺延（见 7.1）。

### 6.3 开源策略

| 项目 | 决策 |
|---|---|
| 代码（blink_cli Rust workspace）| GitHub public + **MIT license** |
| Zenodo DOI | 投稿当天 freeze release tag、Zenodo 自动出 DOI、cite 在 paper |
| HXMT 1B 数据 | 不开源（机构数据），upon request to HXMT SOC（hxmtsoc@ihep.ac.cn） |
| 跨卫星参考数据 | 不重发布，给原始下载路径和 query 参数 |
| Plot 脚本（scripts/plot_*）| GitHub 同 repo `scripts/` |
| Python 依赖 | `pyproject.toml` + uv |

**Code & Data Availability Statement 草稿**：

> The reconstruction pipeline is implemented in Rust and released under
> the MIT License at https://github.com/[user]/blink (commit hash X,
> Zenodo DOI Y). Plotting and validation scripts are included in the same
> repository under `scripts/`. Insight-HXMT raw 1B telemetry is the
> property of the Institute of High Energy Physics, Chinese Academy of
> Sciences, and is available upon reasonable request to the HXMT Science
> Operation Center (hxmtsoc@ihep.ac.cn). Cross-validation data are from
> public archives: Fermi/GBM at HEASARC, GECAM-C at the National Space
> Science Data Center, and INTEGRAL/SPI-ACS at the ISDC.

### 6.4 Cover letter 大纲

```
段 1 (~3 句)：
  我们提交关于 HXMT/HE 1B 饱和数据恢复的方法论文。HE 在亮源观测中
  受 FIFO 饱和限制 [问题陈述]。本文系统量化了从 1B 原始数据恢复的
  方法和适用边界 [本文工作]。

段 2 (~3 句):
  Highlights:
  - 中度饱和近无损恢复（260226A 残差 X events）
  - 极端饱和量化（221009A 96% 覆盖率，明确给出方法极限）
  - 三层独立验证（同仪器工程数据 + 跨 box + 跨卫星）

段 3 (~2 句):
  代码开源、数据通路完整记录，便于后续 HXMT 用户和未来类似仪器借鉴。

段 4：
  推荐 reviewer 3-5 名（不与作者团队有合作关系；通常我们写跨卫星
  仪器的同行——例如 GBM、GECAM、INTEGRAL 团队的 instrument paper
  作者；以及 LIS 算法的应用领域 reviewer）。
```

---

## 7. Risks 与 Mitigation

### R1【高】工程数据 W -1 没 done → F6 缺位

- **触发条件**：scan_eng_k 显示 k 在不同 hour 漂移 >15%、multiplicative vs additive 修正方法没收敛、或者死时间修正系数 β 跑不出物理上合理的值
- **影响**：F6 拿不出来，validation 章节 5.2 整节空了
- **Mitigation**：
  - 有 ≥2 个 GRB 工程数据能 reasonably 对上即可放正文
  - 退路：F6 降级为"GRB 260226A only" 单一案例
  - 终极退路：F6 挪到 supplementary，主线变成"跨卫星 + 跨 box"两层
- **决策门槛**：D 5 节点必须决定 F6 状态

### R2【高】数字 W +0 之后还在变

- **触发条件**：算法/重建管线后续 commit 改动了关键 metric
- **Mitigation**：
  - W -2/-1 数字定版的 commit hash 写到 paper Methods section
  - W +0 之后**禁止改算法逻辑**，只 bug 修
  - D 1-9 期间发现的算法问题，记 issue 不动主分支，留 follow-up paper

### R3【中】Reviewer challenge wrap 唯一性

- **触发条件**：reviewer 看 Sec 3.4.3，问"how do you prove uniqueness"
- **Mitigation**：
  - Sec 3.4.3 写好双重证据：skip-wrap-0 LIS 长度严格减少 + 跨 box 互相关 wrap 0 vs wrap +1 残差
  - 准备 ~1 页 supplementary 列出所有 Δstime>1 SEC 对的 skip-wrap test 结果

### R4【中】Magnetar (200415A) scope 质疑

- **触发条件**：reviewer 问 "why is a magnetar in a GRB paper?"
- **Mitigation**：title 已叫 "Bright Burst Observations"（不是 "GRB"）；Sec 5.3.3 第一句 frame 成 "demonstrate method robustness across source classes"

### R5【低】LaTeX 模板迁移破坏交叉引用

- **Mitigation**：D 8 切换模板时先 dry-run 编译

### R6【低】内部 review 翻盘

- **Mitigation**：现在告诉导师 "D 9 review 是 final pass，scope 已定，不接受重写"；如要求大改 → 顺延 1 周

### R7【低】SPI-ACS 时间系统翻车

- **触发条件**：光行差符号或闰秒处理弄反（memory 警示"曾两次搞反"）
- **Mitigation**：DESIGN.md 第 411-438 行规范 + 人肉比对 1B 重建后事件 UTC 与 SPI-ACS UTC 在 ms 量级 align

---

## 8. 决策 Summary

| 维度 | 决策 |
|---|---|
| 期刊 | SCPMA Article |
| Title | Saturation Recovery for Insight-HXMT/HE Bright Burst Observations from Level-1B Raw Data |
| 作者 | 全 HXMT 团队，第一作者郭昊轩，corresponding 导师 |
| 叙事重心 | 饱和修复 = headline，时间重建 = prerequisite |
| Scope | DESIGN.md 当前内容 + 工程数据 validation（headline）+ SPI-ACS 磁星 + 失效边界量化 |
| 章节 | 7 节 ~13 页（Intro 1.5p / DataSystem 1.5p / TimeRecon 2.5p / SatRecovery 2p / Validation 3p / Discussion 1.5p / Conclusion 0.5p） |
| 图表 | 8 figs + 4 tables，F6（工程数据）作为 headline 图 |
| Headline 验证 | F6 工程数据同仪器内部一致性 |
| 时间表 | W -2/-1 工程数据并行 + 数字定版 + Sec 1-3 草稿；W +0 起 11 天写作 + 投稿 |
| 开源 | blink_cli MIT + Zenodo DOI；HXMT 1B 数据 upon request |
| Bib | ~25-30 条，从 ADS 挖；草稿出来给团队过漏掉的 |
| 主要风险 | F6 工程数据 slip / 数字 W+0 后还在改 / wrap 唯一性 challenge |

---

## 9. 中文 main.tex 处置

- **不再翻译/复用其文本**——完全重写英文版
- 保留作为 figure/table 的 placeholder reference（13 张图的素材在 paper/ 目录）
- 旧 main.tex commit message 注明 "superseded by SCPMA English version"，不删除（git history 留痕）

## 10. 下一步

按 brainstorming skill 流程，design 接受后用 writing-plans skill 制定实施计划：分解 W -2 → W +11 的逐日任务，包括：

- 数字定版的具体 `blink_cli` 命令和输出格式
- 各章节的具体 outline（标题级到段落级）
- 每张图的数据源、绘图脚本、最终 PNG 输出
- bibliography 的逐条 lookup
- LaTeX 模板切换的具体步骤
- 投稿前 checklist
