"""Part 2: 时间重建 — slides 11-21."""

from pptx_helpers import *


def add_slides(prs):
    _slide_11_deep_saturation(prs)
    _slide_12_three_modes(prs)
    _slide_13_nwraps_problem(prs)
    _slide_14a_fifo_congestion_delay(prs)
    _slide_14b_fifo_reset_reorder(prs)
    _slide_14c_wrap_boundary_ambiguity(prs)
    _slide_15_pipeline_overview(prs)
    _slide_16_path3_normal(prs)
    _slide_17a_wrap_tracking_principle(prs)
    _slide_17b_wrap_tracking_details(prs)
    _slide_18_path2_stale_anchor(prs)
    _slide_19a_stale_anchor_problem(prs)
    _slide_19b_backward_solution(prs)
    _slide_20_backward_phases(prs)
    _slide_21_double_ambiguity(prs)


# ── Slide 11: Deep saturation mode ──────────────────────────────────────────

def _slide_11_deep_saturation(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "深度饱和模式",
                  "MCU 主循环时序主导观测数据模式")

    # Left: burst pattern diagram
    add_textbox(slide, Inches(0.5), Inches(1.15), Inches(5.5), Inches(0.4),
                "MCU 主循环时序模式", font_size=16, bold=True)

    # Draw burst/idle bars
    base_x = Inches(0.5)
    bar_y = Inches(1.7)
    bar_h = Inches(0.8)
    cycle_w = Inches(1.7)  # total per cycle

    for i in range(3):
        cx = base_x + cycle_w * i
        # Burst bar (narrow, green) — ~0.5ms out of ~8ms ≈ 6.25%
        burst_w = Inches(0.25)
        draw_bar(slide, cx, bar_y, burst_w, bar_h, GREEN,
                 label="突发读取", font_size=9)
        # Idle bar (wide, gray)
        idle_w = Inches(1.35)
        draw_bar(slide, cx + burst_w + Inches(0.05), bar_y, idle_w, bar_h,
                 GRAY, label="空闲 / MCU 其他操作", font_size=9)

    # Labels below bars
    add_textbox(slide, Inches(0.5), Inches(2.6), Inches(1.0), Inches(0.3),
                "~0.5 ms", font_size=11, color=GREEN, bold=True)
    add_textbox(slide, Inches(1.5), Inches(2.6), Inches(1.2), Inches(0.3),
                "~7.5 ms", font_size=11, color=GRAY, bold=True)

    # Timeline arrow
    add_arrow(slide, Inches(0.5), Inches(2.95), Inches(5.6), Inches(2.95),
              color=DARK_GRAY)
    add_textbox(slide, Inches(4.8), Inches(2.75), Inches(1.0), Inches(0.3),
                "时间", font_size=11, color=DARK_GRAY)

    # Annotation: FIFO overflows during idle
    add_rich_textbox(slide, Inches(0.5), Inches(3.2), Inches(5.5), Inches(3.8), [
        ("深度饱和工作原理：", 14, True),
        (""),
        ("MCU 每 ~8 ms 完成一次主循环", 13),
        ("  - ~0.5 ms：突发读取 FIFO 中 109 个事件（一个 CCSDS 包）", 12),
        ("  - ~7.5 ms：其他任务（管家数据、遥测等）", 12),
        (""),
        ("在 ~7.5 ms 空闲窗口期间：", 13, True, BLUE),
        ("  - FIFO 持续积累探测器事件", 12),
        ("  - FIFO 被填满并反复溢出", 12),
        ("  - 溢出事件被硬件静默丢弃", 12),
        (""),
        ("突发读取仅捕获 MCU 开始读取时", 13),
        ("FIFO 中存活的事件。", 13),
    ])

    # Right: why conventional methods fail
    add_rich_textbox(slide, Inches(6.8), Inches(1.15), Inches(6.0), Inches(5.8), [
        ("为什么传统方法失败：", 16, True, RED),
        (""),
        ("1. 表观事件率具有误导性", 14, True),
        ("   所有相邻包显示 < 15,000 evt/s", 13),
        ("   因为包时间跨度反映的是 MCU 主循环", 12),
        ("   周期（~8 ms），而非物理事件率。", 12),
        ("   真实率可能高 10-100 倍。", 12),
        (""),
        ("2. 无法从数据估计真实率（lambda）", 14, True),
        ("   标准泊松估计需要已知观测窗口。", 13),
        ("   但有效窗口仅为 ~0.5 ms 突发读取，", 12),
        ("   而非 ~8 ms 包跨度。", 12),
        ("   相邻包的计数率同样不可靠。", 12),
        (""),
        ("3. 重建策略", 14, True, GREEN),
        ("   使用突发读取段的计数率作为", 13),
        ("   FIFO 复位模型中 R_true 的近似值。", 13),
        ("   突发读取从满 FIFO 中读取，109 个事件", 12),
        ("   / 0.5 ms 突发给出 R_burst ~ 218,000 evt/s。", 12),
        ("   这是真实物理率的下界。", 12),
    ])


# ── Slide 12: Three modes comparison ────────────────────────────────────────

def _slide_12_three_modes(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "三种饱和模式对比",
                  "同一拥塞过程，不同严重程度")

    col_widths = [Inches(2.2), Inches(2.5), Inches(2.5), Inches(2.5), Inches(2.5)]
    rows = [
        ["", "静默丢弃\n（包内）", "FIFO 复位\n（包间）", "深度饱和\n（突发模式）"],
        ["触发条件", "写入 > 读取\n短暂拥塞", "写入 >> 读取\n循环结束时\nFIFO 仍满", "写入 >>> 读取\nFIFO 始终满"],
        ["数据丢失", "包内事件丢失\n（不可见间隙）", "FIFO 复位之间\n整包丢失", "空闲期间\n大量丢失；\n仅突发读取\n事件存活"],
        ["可检测性", "仅从数据\n无法检测\n（无标记）", "包间出现\n大时间间隙\n（可检测）", "所有相邻包\n均呈现 ~8ms 跨度\n含 ~109 事件"],
        ["重建方法", "无法直接重建；\n统计估计\n丢失事件数", "基于间隙的\n插值；\n从相邻包\n获取 R_true", "使用突发读取率\n作为 R_true；\n插值空闲窗口"],
    ]
    draw_table(slide, Inches(0.7), Inches(1.3), col_widths, rows,
               row_height=Inches(1.1), font_size=11)

    # Bottom summary
    add_box(slide, Inches(1.5), Inches(6.6), Inches(10.0), Inches(0.5),
            "这三种模式是同一 FIFO 拥塞过程的不同严重程度。"
            "静默丢弃 < FIFO 复位 < 深度饱和。",
            fill_color=LIGHT_ORANGE, border_color=ORANGE, font_size=13, bold=True)


# ── Slide 13: The n_wraps problem ───────────────────────────────────────────

def _slide_13_nwraps_problem(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "核心问题：回绕次数 n_wraps 的确定",
                  "核心挑战：确定整数回绕计数")

    # Left: MET formula
    add_textbox(slide, Inches(0.5), Inches(1.2), Inches(6.0), Inches(0.4),
                "MET 重建公式", font_size=16, bold=True)

    add_box(slide, Inches(0.5), Inches(1.7), Inches(5.8), Inches(1.0),
            "t_MET = anchor + (n_wraps x PTIME_MOD + p - p_anchor) x 2us + 4.0s",
            fill_color=LIGHT_BLUE, border_color=BLUE, font_size=14, bold=True)

    add_rich_textbox(slide, Inches(0.5), Inches(2.9), Inches(5.8), Inches(3.5), [
        ("各项含义：", 14, True),
        (""),
        ("anchor       SEC 锚点 MET（stime + offset）", 12, False, DARK_GRAY),
        ("n_wraps     整数回绕计数（待求未知量）", 12, True, RED),
        ("PTIME_MOD   524288 = 2^19（计数器模数）", 12, False, DARK_GRAY),
        ("p              事件 ptime（19 位计数器值）", 12, False, DARK_GRAY),
        ("p_anchor    SEC 事件的 ptime", 12, False, DARK_GRAY),
        ("2 us           ptime 时钟分辨率", 12, False, DARK_GRAY),
        ("4.0 s          1B 到 1K 经验修正", 12, False, DARK_GRAY),
        (""),
        ("除 n_wraps 外，其余各项均可直接获取。", 13, True),
        ("n_wraps 决定事件属于哪个 1.05s 窗口。", 13),
    ])

    # Right: sawtooth diagram
    add_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.5), Inches(0.4),
                "ptime 每 ~1.05 秒回绕一次", font_size=16, bold=True)

    origin_x = Inches(7.5)
    origin_y = Inches(5.0)
    plot_w = Inches(5.0)
    plot_h = Inches(2.8)

    draw_sawtooth(slide, origin_x, origin_y, plot_w, plot_h, n_cycles=3)

    # Axis labels
    add_textbox(slide, origin_x - Inches(0.1), origin_y + Inches(0.05),
                Inches(3.0), Inches(0.3),
                "时间（秒）", font_size=11, color=GRAY)
    add_textbox(slide, origin_x - Inches(1.2), origin_y - plot_h + Inches(0.3),
                Inches(1.2), Inches(0.5),
                "ptime\n(0..524287)", font_size=11, color=GRAY)

    # Wrap period labels
    for i in range(3):
        cycle_w = int((plot_w - Inches(0.3)) / 3)
        x = origin_x + cycle_w * i + cycle_w // 2
        add_textbox(slide, x - Inches(0.3), origin_y + Inches(0.2),
                    Inches(0.8), Inches(0.3),
                    f"~1.05s", font_size=10, color=BLUE,
                    alignment=PP_ALIGN.CENTER)

    # Explanation below
    add_rich_textbox(slide, Inches(7.0), Inches(5.3), Inches(5.5), Inches(1.8), [
        ("正常情况下：", 13, True, GREEN),
        ("  锚点新鲜（距上次 SEC <=35 个包）", 12),
        ("  最多发生 1 次回绕", 12),
        ("  简单阈值法即可判定 n_wraps = 0 或 1", 12),
    ])


# ── Slide 14: Why saturation breaks everything ──────────────────────────────

def _slide_14a_fifo_congestion_delay(prs):
    """问题1：FIFO 拥塞导致 utc_tail 滞后 — 用管道比喻详细解释"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "问题 1：FIFO 拥塞 → utc_tail 滞后",
                  "饱和破坏时间重建的第一个原因")

    # Left: 管道比喻
    add_rich_textbox(slide, Inches(0.5), Inches(1.2), Inches(6.0), Inches(1.2), [
        ("把 FIFO 想象成一根管道", 20, True, BLUE),
        ("事件从一端进入，MCU 从另一端读出。管道长度 = 延迟。", 14),
    ])

    # 正常情况示意
    y1 = Inches(2.5)
    add_textbox(slide, Inches(0.5), y1, Inches(2.0), Inches(0.3),
                "正常情况（管道短）：", font_size=13, bold=True, color=GREEN)
    draw_bar(slide, Inches(0.7), y1 + Inches(0.35), Inches(0.8), Inches(0.35),
             GREEN, "事件 T=100s", font_size=9)
    draw_bar(slide, Inches(1.5), y1 + Inches(0.35), Inches(0.6), Inches(0.35),
             LIGHT_GREEN, "短管道", font_size=9, text_color=GREEN, border_color=GREEN)
    draw_bar(slide, Inches(2.1), y1 + Inches(0.35), Inches(1.0), Inches(0.35),
             GREEN, "MCU读出 T~100s", font_size=9)
    add_arrow(slide, Inches(1.5), y1 + Inches(0.52), Inches(2.1), y1 + Inches(0.52),
              color=DARK_GRAY)
    add_textbox(slide, Inches(3.3), y1 + Inches(0.35), Inches(2.5), Inches(0.35),
                "utc_tail ~ 事件真实时间 ✓", font_size=12, bold=True, color=GREEN)

    # 饱和情况示意
    y2 = Inches(3.4)
    add_textbox(slide, Inches(0.5), y2, Inches(2.5), Inches(0.3),
                "饱和时（管道越来越长）：", font_size=13, bold=True, color=RED)
    draw_bar(slide, Inches(0.7), y2 + Inches(0.35), Inches(0.8), Inches(0.35),
             BLUE, "事件 T=100s", font_size=9)
    draw_bar(slide, Inches(1.5), y2 + Inches(0.35), Inches(2.5), Inches(0.35),
             LIGHT_RED, "======= 长管道（积压） =======", font_size=9,
             text_color=RED, border_color=RED)
    draw_bar(slide, Inches(4.0), y2 + Inches(0.35), Inches(1.2), Inches(0.35),
             RED, "MCU读出 T=103s", font_size=9)
    add_arrow(slide, Inches(1.5), y2 + Inches(0.52), Inches(4.0), y2 + Inches(0.52),
              color=RED)
    add_textbox(slide, Inches(5.4), y2 + Inches(0.3), Inches(1.0), Inches(0.5),
                "差了3秒！", font_size=13, bold=True, color=RED)

    # 具体计算说明
    add_rich_textbox(slide, Inches(0.5), Inches(4.3), Inches(5.5), Inches(3.0), [
        ("为什么这会导致 n_wraps 错误？", 16, True, RED),
        ("", 6),
        ("源率 50,000 evt/s，MCU 读速率 ~15,800 evt/s", 13),
        ("→ FIFO 不断积压，延迟持续增长到数秒", 13),
        ("", 6),
        ("SEC 锚点也在 FIFO 里排队。35 个包后锚点「过期」。", 13),
        ("想用 utc_tail 估算回绕次数：", 13),
        ("  n_wraps ~ (utc_tail - anchor_MET) / 1.05s", 13, True, DARK),
        ("", 6),
        ("但 utc_tail 说的是 MCU 读出时间（103s），", 13),
        ("而事件实际产生在 100s。", 13),
        ("多算了 3 / 1.05 ~ 3 次 wrap！", 14, True, RED),
        ("", 6),
        ("→ 事件被放到错误位置（偏差 ~3.15 秒）", 14, True, RED),
    ])

    # Right: 图解
    rx = Inches(7.0)
    add_box(slide, rx, Inches(1.2), Inches(5.8), Inches(5.8),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, rx + Inches(0.2), Inches(1.3), Inches(5.4), Inches(5.5), [
        ("FIFO 延迟如何随时间增长", 16, True, BLUE),
        ("", 8),
        ("  时间   源率    MCU率    FIFO积压   延迟", 12, True, DARK_GRAY),
        ("  T+0s   5000   15800    0 evt     ~0ms", 12, False, GREEN),
        ("  T+1s   20000  15800    4200      ~0.3s", 12),
        ("  T+2s   30000  15800    18400     ~1.2s", 12),
        ("  T+3s   50000  15800    52600     ~3.3s", 12, False, RED),
        ("  T+5s   50000  15800    121000    ~7.7s", 12, False, RED),
        ("", 8),
        ("FIFO 容量只有 455 个事例。", 13),
        ("满了之后新事例被直接丢弃，", 13),
        ("但 MCU 继续读旧的 → 延迟不减反增。", 13),
        ("", 8),
        ("关键问题：", 14, True, RED),
        ("SEC 锚点也有延迟。锚点的 anchor_MET", 13),
        ("是准确的（硬件写入的整秒时间），", 13),
        ("但这个 SEC 是从 FIFO 里读出来的，", 13),
        ("它对应的「当下」其实是几秒前。", 13),
        ("", 6),
        ("用这样的锚点 + utc_tail 做时间重建", 13),
        ("→ utc_tail 过大 → n_wraps 多算 → 事件偏移", 13, True, RED),
    ])


def _slide_14b_fifo_reset_reorder(prs):
    """问题2：FIFO 复位后文件顺序混乱 + 陈旧锚点"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "问题 2：FIFO 复位 → 陈旧锚点 + 文件乱序",
                  "饱和破坏时间重建的第二个原因")

    # Timeline diagram
    add_textbox(slide, Inches(0.5), Inches(1.2), Inches(12), Inches(0.4),
                "FIFO 复位前后发生了什么？（按真实时间排列）", font_size=16, bold=True, color=RED)

    # Draw timeline
    tl_y = Inches(1.8)
    tl_h = Inches(0.5)
    add_line(slide, Inches(0.5), tl_y + tl_h/2, Inches(12.8), tl_y + tl_h/2,
             color=DARK_GRAY, width=Pt(2))

    # Phase 1: congested packets
    draw_bar(slide, Inches(0.5), tl_y, Inches(3.0), tl_h,
             BLUE, "拥塞包 (T=240~241s)", font_size=10)
    add_textbox(slide, Inches(0.5), tl_y + tl_h, Inches(3.0), Inches(0.3),
                "MCU在T=243~244s读出（延迟3s）", font_size=9, color=DARK_GRAY)

    # FIFO reset
    draw_bar(slide, Inches(3.8), tl_y - Inches(0.15), Inches(0.8), tl_h + Inches(0.3),
             RED, "FIFO\n复位!", font_size=10)
    add_textbox(slide, Inches(3.5), tl_y + tl_h, Inches(1.2), Inches(0.3),
                "T=244s 清空所有数据", font_size=9, color=RED)

    # Phase 2: fresh packets
    draw_bar(slide, Inches(4.9), tl_y, Inches(2.5), tl_h,
             GREEN, "新鲜包 (T=244.01s~)", font_size=10)
    add_textbox(slide, Inches(4.9), tl_y + tl_h, Inches(2.5), Inches(0.3),
                "MCU立刻读出（无延迟！）", font_size=9, color=GREEN)

    # Old anchor persists
    draw_bar(slide, Inches(7.8), tl_y, Inches(2.0), tl_h,
             ORANGE, "旧锚点仍在使用", font_size=9)
    add_textbox(slide, Inches(7.8), tl_y + tl_h, Inches(2.5), Inches(0.3),
                "复位前最后接受的SEC仍为当前锚点", font_size=8, color=ORANGE)

    # Fresh SEC
    draw_bar(slide, Inches(10.2), tl_y, Inches(2.0), tl_h,
             GREEN, "新鲜 SEC ✓", font_size=10)
    add_textbox(slide, Inches(10.2), tl_y + tl_h, Inches(2.5), Inches(0.3),
                "真正的新整秒标记", font_size=9, color=GREEN)

    # Problem explanation
    add_rich_textbox(slide, Inches(0.5), Inches(2.9), Inches(6.0), Inches(4.3), [
        ("过期锚点的问题", 18, True, RED),
        ("", 8),
        ("FIFO 复位清空了所有积压数据，复位后的事件全是新鲜的。", 13),
        ("FIFO 里没有旧 SEC 混入新鲜包！队列严格 FIFO。", 13, True, DARK),
        ("", 6),
        ("真正的问题：复位前最后接受的那个 SEC 仍然是", 13),
        ("MCU 的当前锚点。直到下一个整秒边界 FPGA 注入", 13),
        ("新的 SEC，这中间可能有将近 1 秒的事件都在用", 13),
        ("这个过期锚点计算时间。", 13),
        ("", 6),
        ("过期锚点的 (MET, ptime) 本身是正确的，但距离", 13),
        ("现在已过了 ~4 个 wrap 周期（~4 秒），阈值法", 13),
        ("只能处理 ±1 wrap → 估算错误 ±1 wrap。", 13, True, RED),
        ("", 8),
        ("实际例子（221009A Box B T+249s）：", 14, True, ORANGE),
        ("  应该有 9960 个事件", 13),
        ("  修正前：只找到 402 个（-96%！）", 13, True, RED),
        ("  其余 9558 个被错放到了 T+248s（差 1 个 wrap）", 13),
        ("  修正后：恢复到 10065 个（+1.1%）✓", 13, True, GREEN),
    ])

    # Right: 文件顺序 vs 时间顺序
    add_box(slide, Inches(7.0), Inches(2.9), Inches(5.8), Inches(4.3),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.2), Inches(3.0), Inches(5.4), Inches(4.0), [
        ("文件顺序 vs 时间顺序", 16, True, ORANGE),
        ("", 8),
        ("数据文件中的顺序：", 14, True, DARK),
        ("  包 100: 事件来自 T=240s （拥塞，有延迟）", 12),
        ("  包 101: 事件来自 T=241s （拥塞）", 12),
        ("  ------ FIFO 复位 ------", 12, True, RED),
        ("  包 102: 事件来自 T=244s （新鲜！）", 12, True, GREEN),
        ("  包 103: 事件来自 T=244.01s（新鲜）", 12, True, GREEN),
        ("", 8),
        ("看起来连续，实际时间跳了 3 秒！", 14, True, RED),
        ("", 8),
        ("为什么偏移恰好 ~1.05s？", 14, True, DARK),
        ("  复位前最后的 SEC 是在拥塞期接受的", 12),
        ("  它到现在已经过了好几个 wrap 周期", 12),
        ("  路径2用 utc_tail 估算 n_wraps：", 12),
        ("    n = round(elapsed / 1.05)", 12),
        ("  取整有 ±1 误差 → 事件偏移 ±1 个 WRAP", 12),
        ("  → 一整批事件偏早/偏晚恰好 1.05s", 12, True, RED),
    ])


def _slide_14c_wrap_boundary_ambiguity(prs):
    """问题3：n_base 取整在 median 接近 anchor_ptime 时不稳定"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "问题 3：n_base 取整边界不稳定",
                  "饱和破坏时间重建的第三个原因")

    # Left: 解释 n_base 估算
    add_rich_textbox(slide, Inches(0.5), Inches(1.2), Inches(6.0), Inches(2.2), [
        ("过期锚路径如何估算 n_wraps？", 18, True, PURPLE),
        ("", 8),
        ("锚点过期后，用 median ptime + utc_tail 估算：", 13),
        ("", 6),
        ("n_est = round( (elapsed - (p_median - p_anchor) * 2us)", 13, True, DARK),
        ("                / WRAP_PERIOD )", 13, True, DARK),
        ("", 6),
        ("关键：这里有一个 round()，当真实值接近 x.5 时，", 13),
        ("取整结果不稳定 — 微小变化可能翻转 n 和 n+1。", 13, True, RED),
    ])

    # Example: stable case
    add_box(slide, Inches(0.5), Inches(3.5), Inches(5.8), Inches(1.5),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(0.7), Inches(3.55), Inches(5.4), Inches(1.4), [
        ("例1：median 远离 anchor — 取整稳定 ✓", 14, True, GREEN),
        ("  anchor_ptime=5000, elapsed=5.75s, median=300000", 12),
        ("  (p_median - p_anchor)*2us = 295000*2us = 0.59s", 12),
        ("  n = round((5.75 - 0.59) / 1.0486) = round(4.92) = 5 ✓", 12),
        ("  相邻包 median 类似 → 都取整到 5 → 稳定", 12),
    ])

    # Example: unstable case
    add_box(slide, Inches(0.5), Inches(5.2), Inches(5.8), Inches(2.0),
            "", fill_color=LIGHT_RED, border_color=RED)
    add_rich_textbox(slide, Inches(0.7), Inches(5.25), Inches(5.4), Inches(1.9), [
        ("例2：median 接近 anchor — 取整翻转! ✗", 14, True, RED),
        ("  anchor_ptime=5000, elapsed=5.75s", 12),
        ("", 6),
        ("  包A median=10000: delta=0.01s", 12),
        ("    n = round((5.75-0.01)/1.0486) = round(5.47) = 5 ✓", 12),
        ("", 4),
        ("  包B median=524000: delta=1.038s", 12, True, RED),
        ("    n = round((5.75-1.038)/1.0486) = round(4.49) = 4 ✗", 12, True, RED),
        ("    应该是 5！差了一整个 WRAP!", 12, True, RED),
    ])

    # Right: 为什么会这样 + 在光变曲线上的表现
    add_box(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(3.0),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(7.2), Inches(1.3), Inches(5.4), Inches(2.8), [
        ("为什么 median 接近 anchor 时会出问题？", 16, True, PURPLE),
        ("", 8),
        ("ptime 每 ~1.05s 绕一圈。在某些包中，", 13),
        ("median ptime 恰好经过 anchor_ptime 附近。", 13),
        ("", 6),
        ("此时 (p_median - p_anchor) 在 ~0 和 ~524288", 13),
        ("之间跳变 → 分子变化 ~1.05s → 取整翻转 ±1", 13),
        ("", 6),
        ("结果：相邻的包被分到不同 wrap 层级，", 13),
        ("在光变曲线上表现为「凹坑」：", 13, True, RED),
        ("  包A (n=5) ████  包B (n=4!) ▃▃  包C (n=5) ████", 12, True, DARK_GRAY),
        ("  正确高度      低了1.05s!     正确高度", 11, False, DARK_GRAY),
    ])

    add_box(slide, Inches(7.0), Inches(4.5), Inches(5.8), Inches(2.8),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.2), Inches(4.6), Inches(5.4), Inches(2.6), [
        ("解决思路预告", 16, True, GREEN),
        ("", 8),
        ("问题 1（utc_tail 不可靠）：", 13, True, BLUE),
        ("  → Wrap Tracking：不依赖 utc_tail，", 12),
        ("    用 median ptime 的单调性追踪 wrap", 12),
        ("", 6),
        ("问题 2（过期锚点持续生效）：", 13, True, ORANGE),
        ("  → 后向 SEC 修正：等新鲜 SEC 来反向纠正", 12),
        ("", 6),
        ("问题 3（n_base 取整不稳定）：", 13, True, PURPLE),
        ("  → Pass 3 后处理：检测凹坑批次并修正 ±1 WRAP", 12),
        ("", 6),
        ("三个问题需要三套不同机制 → 四遍处理管线", 13, True, DARK),
    ])


# ── Slide 15: 4-Pass pipeline overview ───────────────────────────────────────

def _slide_15_pipeline_overview(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "四遍处理管线总览",
                  "路线图：每遍处理解决一类特定错误")

    # Five boxes: Pass 1 + supplement + Pass 2 + Pass 3 + Pass 4
    box_w = Inches(2.1)
    box_h = Inches(4.2)
    box_y = Inches(1.4)
    gap = Inches(0.2)
    start_x = Inches(0.4)

    boxes = [
        ("第 1 遍", "逐包\n时间重建",
         "三条路径：\n"
         "1. Wrap tracking\n   （FIFO 拥塞）\n"
         "2. 过期锚点\n   （FIFO 复位后）\n"
         "3. 阈值法\n   （正常 / 新鲜）",
         LIGHT_BLUE, BLUE),
        ("补充步骤", "反向 SEC\n修正",
         "缓存 FIFO 复位后\n的事件。\n\n"
         "等待新鲜 SEC，\n然后反向修正\n过期锚点误差。\n\n"
         "解决：来自 FIFO\n缓冲区的过期 SEC",
         LIGHT_ORANGE, ORANGE),
        ("第 2 遍", "回绕逆转\n修复",
         "检测包批次间\n~1.05s 的\n时间逆转。\n\n"
         "间隙准则区分\n真实 FIFO 复位\n误差与文件\n乱序。\n\n"
         "解决：文件顺序\n!= 时间顺序",
         LIGHT_RED, RED),
        ("第 3 遍", "边界凹陷\n修复",
         "检测包序列中\n+1 WRAP 跳变。\n\n"
         "修复凹陷批次\n（高邻居间的\n低值包）。\n\n"
         "修复混合包\n（跨度 ~ WRAP）。\n\n"
         "解决：回绕\n边界模糊",
         LIGHT_PURPLE, PURPLE),
        ("第 4 遍", "分段\n排序",
         "在非反向修正\n段内按时间\n排序事件。\n\n"
         "保留反向修正\n的结果。\n\n"
         "解决：残余的\n乱序包",
         LIGHT_GREEN, GREEN),
    ]

    for i, (title, subtitle, desc, fill, border) in enumerate(boxes):
        x = start_x + (box_w + gap) * i
        # Main box
        add_box(slide, x, box_y, box_w, box_h, "",
                fill_color=fill, border_color=border)
        # Title
        add_textbox(slide, x + Inches(0.1), box_y + Inches(0.1),
                    box_w - Inches(0.2), Inches(0.35),
                    title, font_size=15, bold=True, color=border,
                    alignment=PP_ALIGN.CENTER)
        # Subtitle
        add_textbox(slide, x + Inches(0.1), box_y + Inches(0.45),
                    box_w - Inches(0.2), Inches(0.5),
                    subtitle, font_size=12, bold=True, color=DARK,
                    alignment=PP_ALIGN.CENTER)
        # Description
        add_textbox(slide, x + Inches(0.1), box_y + Inches(1.0),
                    box_w - Inches(0.2), Inches(3.0),
                    desc, font_size=10, color=DARK_GRAY)

        # Arrow between boxes
        if i < len(boxes) - 1:
            ax = x + box_w
            ay = box_y + box_h / 2
            add_arrow(slide, ax, ay, ax + gap, ay, color=DARK_GRAY, width=Pt(2.5))

    # Bottom: data flow
    add_textbox(slide, Inches(0.4), Inches(5.8), Inches(11.5), Inches(0.5),
                "输入：含 19 位 ptime 的原始 CCSDS 包    -->    "
                "输出：Vec<Vec<f64>> — 逐包重建的 MET 时间",
                font_size=13, bold=True, color=DARK_GRAY,
                alignment=PP_ALIGN.CENTER)


# ── Slide 16: Path 3 - Normal/threshold ─────────────────────────────────────

def _slide_16_path3_normal(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "路径 3：正常路径（阈值法）",
                  "最简单的情况 — 锚点新鲜时")

    # Left: algorithm
    add_rich_textbox(slide, Inches(0.5), Inches(1.2), Inches(6.0), Inches(5.5), [
        ("路径 3 何时适用？", 15, True, GREEN),
        ("  锚点新鲜：距上次 SEC <= 35 个包", 13),
        ("  经过时间 < 1.5 x WRAP_PERIOD", 13),
        ("  最多发生 1 次回绕", 13),
        (""),
        ("算法：", 15, True),
        ("  raw_delta = p - p_anchor", 13, True),
        (""),
        ("  if raw_delta < -10000:", 13, False, BLUE),
        ("      正向回绕：事件越过 0 边界", 12),
        ("      adjusted = raw_delta + PTIME_MOD", 12),
        (""),
        ("  elif raw_delta > PTIME_MOD - 10000:", 13, False, BLUE),
        ("      锚点在事件之前越过边界", 12),
        ("      adjusted = raw_delta - PTIME_MOD", 12),
        (""),
        ("  else:", 13, False, BLUE),
        ("      同一回绕：adjusted = raw_delta", 12),
        (""),
        ("  theta = 10000 ticks = 20 ms", 13, False, DARK_GRAY),
    ])

    # Right: key insights
    add_rich_textbox(slide, Inches(6.8), Inches(1.2), Inches(5.8), Inches(2.5), [
        ("关键洞察：ptime 严格单调递增", 15, True, BLUE),
        (""),
        ("MCU 顺序读取 FIFO。在拥塞期间，", 13),
        ("ptime 只增不减。不存在反向回绕的", 13),
        ("物理机制。", 13),
        (""),
        ("两种阈值情况都是从不同锚点位置", 13),
        ("观察到的正向回绕：", 13),
        ("  raw_delta < -theta：事件回绕，锚点未回绕", 12, False, DARK_GRAY),
        ("  raw_delta > MOD-theta：锚点回绕，事件未回绕", 12, False, DARK_GRAY),
    ])

    # Why it works / fails
    add_box(slide, Inches(6.8), Inches(4.0), Inches(5.8), Inches(1.3),
            "为什么有效：\n"
            "新鲜锚点 = 最多经过 1 个回绕周期。\n"
            "20ms 阈值可轻松区分 0 次与 1 次回绕。",
            fill_color=LIGHT_GREEN, border_color=GREEN, font_size=13)

    add_box(slide, Inches(6.8), Inches(5.5), Inches(5.8), Inches(1.3),
            "为什么在饱和时失败：\n"
            "锚点变得过期（>35 包，>1.05s 经过）。\n"
            "可能发生多次回绕 — 阈值法无法区分 n=2 和 n=3。",
            fill_color=LIGHT_RED, border_color=RED, font_size=13)


# ── Slide 17a: Path 1 - Wrap Tracking principle ─────────────────────────────

def _slide_17a_wrap_tracking_principle(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "路径 1：Wrap Tracking — 原理与算法",
                  "不依赖 utc_tail，通过 median ptime 的单调性追踪回绕次数")

    # Left: 物理洞察
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(2.5), [
        ("为什么需要 Wrap Tracking？", 16, True, RED),
        ("", 6),
        ("FIFO 拥塞期间：", 13),
        ("  utc_tail 滞后数秒 → 不能用来估算 n_wraps（问题 1）", 13),
        ("  锚点过期 → 阈值法失效（只能 ±1 wrap）", 13),
        ("  需要一个完全不依赖 utc_tail 的方法", 13, True, RED),
        ("", 8),
        ("关键物理洞察", 16, True, GREEN),
        ("  FIFO 是硬件队列，严格先进先出", 13),
        ("  MCU 顺序读取 → 连续包的 median ptime 单调递增", 13),
        ("  median 出现大负跳变 = ptime 绕过了 524287 → 0", 13),
        ("  数跳变次数 = n_wraps。完全不需要 utc_tail！", 13, True, GREEN),
    ])

    # Right: 具体例子
    add_box(slide, Inches(6.8), Inches(1.15), Inches(6.2), Inches(2.5),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(2.3), [
        ("具体例子", 16, True, BLUE),
        ("  SEC 锚点: anchor_ptime = 200000, wrap_count = 0", 12),
        ("", 6),
        ("  包100: median=300000  delta=+100000  正常递增", 12),
        ("  包101: median=400000  delta=+100000  正常", 12),
        ("  包102: median=500000  delta=+100000  正常", 12),
        ("  包103: median=76000   delta=-424000  大负跳变!", 12, True, RED),
        ("    → wrap_count = 1  （第一次回绕）", 12, True, BLUE),
        ("  包104: median=176000  delta=+100000  正常", 12),
        ("  ...", 12),
        ("  包112: median=52000   delta=-438000  大负跳变!", 12, True, RED),
        ("    → wrap_count = 2  （第二次回绕）", 12, True, BLUE),
    ])

    # Bottom: 算法伪代码
    add_box(slide, Inches(0.4), Inches(3.85), Inches(6.2), Inches(3.4),
            "", fill_color=RGBColor(0xF9, 0xF5, 0xF0), border_color=ORANGE)
    add_rich_textbox(slide, Inches(0.6), Inches(3.9), Inches(5.8), Inches(3.2), [
        ("算法", 16, True, ORANGE),
        ("", 6),
        ("初始化（SEC 被接受时）：", 13, True, DARK),
        ("  congestion_wrap_count = 0", 12),
        ("  prev_median = 当前包的 median ptime", 12),
        ("", 6),
        ("每个后续包：", 13, True, DARK),
        ("  median = 这个包 109 个事件的中位数 ptime", 12),
        ("  if (median - prev_median) < -262144:  # 跳了半圈以上", 12),
        ("      congestion_wrap_count += 1         # 多绕了一圈", 12, True, BLUE),
        ("  prev_median = median", 12),
        ("", 6),
        ("每个事件的 MET：", 13, True, DARK),
        ("  n_wraps = congestion_wrap_count (+ phase_correction)", 12),
        ("  MET = anchor_MET + (n_wraps*524288 + p - p_anchor) * 2us + 4.0s", 12),
    ])

    # Bottom right: 为什么用 median
    add_box(slide, Inches(6.8), Inches(3.85), Inches(6.2), Inches(3.4),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.0), Inches(3.9), Inches(5.8), Inches(3.2), [
        ("为什么用 median 而不是单个事件？", 16, True, GREEN),
        ("", 6),
        ("4-bit CRC 碰撞率 1/16 ≈ 6.3%", 13),
        ("  → 109 个事件中 ~7 个是随机数据假阳性", 13),
        ("  → 假阳性的 ptime 是随机值，可能严重偏离", 13),
        ("  → Median 对少数异常值不敏感 ✓", 13),
        ("", 6),
        ("109 个事件的 ptime 跨度只有 ~0.5ms（burst 读取）", 13),
        ("  → median 能很好地代表这个包的时间位置", 13),
        ("", 8),
        ("只递增、不递减：", 14, True, DARK),
        ("  FIFO 拥塞 = MCU 顺序读 = ptime 严格单调递增", 13),
        ("  不存在物理上的反向回绕", 13),
        ("  → 算法只检测 WRAP_INC，没有 WRAP_DEC", 13),
    ])


# ── Slide 17b: Wrap Tracking - phase correction + protections ────────────────

def _slide_17b_wrap_tracking_details(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "路径 1：Wrap Tracking — 相位校正与保护机制",
                  "处理 anchor_ptime 和 median 不在同一半区的情况")

    # Left: Phase correction detailed
    add_box(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(3.5),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(0.6), Inches(1.2), Inches(5.8), Inches(3.3), [
        ("相位校正（Phase Correction）", 16, True, PURPLE),
        ("", 6),
        ("wrap_count 数的是 median 的回绕，但 MET 公式用 anchor_ptime。", 13),
        ("如果两者在 ptime 空间中位置不同，会差 ±1 次。", 13),
        ("", 6),
        ("具体例子：", 14, True, DARK),
        ("  anchor_ptime  = 49418  （接近 0）", 12),
        ("  anchor_median = 521070 （接近 524288）", 12),
        ("", 4),
        ("  median 变化: 521070 → ... → 524287 → 0 → ... → 236000", 12),
        ("  median 回绕了 1 次 → wrap_count = 1", 12),
        ("", 4),
        ("  但从 anchor_ptime=49418 到 236000：", 12),
        ("  raw_delta = 236000 - 49418 = +186582（正数！）", 12, True, RED),
        ("  不需要加 524288！实际 n_wraps 应该是 0！", 12, True, RED),
        ("", 6),
        ("  修正: delta = 521070 - 49418 = 471652", 12),
        ("  |471652| > 524288*0.7 → phase_correction = -1", 12, True, BLUE),
        ("  corrected = 1 + (-1) = 0 ✓", 12, True, GREEN),
    ])

    # Right top: 35-packet freeze
    add_box(slide, Inches(6.8), Inches(1.15), Inches(6.2), Inches(2.0),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(1.8), [
        ("35 包「近期」冻结期", 16, True, ORANGE),
        ("", 6),
        ("SEC 被接受后，前 35 个包用正常阈值法（锚点新鲜）。", 13),
        ("这段时间不更新 prev_median。为什么？", 13),
        ("", 6),
        ("如果每包都更新 prev_median，激活时首次比较只跨 1 包", 12),
        ("  → delta 只有 ~几千 ticks → 分不清正常递增还是噪声", 12),
        ("冻结 35 包后，首次比较跨越所有 recent 包", 12),
        ("  → delta ~120000 ticks → 大且可靠 ✓", 12, True, GREEN),
    ])

    # Right middle: damaged packet
    add_box(slide, Inches(6.8), Inches(3.35), Inches(6.2), Inches(2.0),
            "", fill_color=LIGHT_RED, border_color=RED)
    add_rich_textbox(slide, Inches(7.0), Inches(3.4), Inches(5.8), Inches(1.8), [
        ("损坏包保护：MIN_EVENTS_FOR_MEDIAN = 50", 14, True, RED),
        ("", 6),
        ("221009A 包 39981：只有 4/109 通过 CRC。", 13),
        ("随机 ptime 的 median = 66592（真实 ≈ 355000）", 13),
        ("→ 误触发 WRAP_INC → 后面所有包多算 1 次 → 1 秒空洞", 13),
        ("", 6),
        ("修复：有效事件 < 50 的包跳过 wrap 检测，不更新 prev_median", 12, True, DARK),
        ("221009A 约 32 万包中只有 2 个严重损坏（0.001%）", 12),
    ])

    # Bottom: fifo_reset_no_wt
    add_box(slide, Inches(0.4), Inches(4.85), Inches(12.6), Inches(2.3),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(0.6), Inches(4.9), Inches(12.2), Inches(2.1), [
        ("FIFO 复位后的保护：fifo_reset_no_wt", 16, True, GREEN),
        ("", 6),
        ("Wrap Tracking 的前提：FIFO 拥塞 → MCU 从满的 FIFO 顺序读 → median 单调递增", 14),
        ("", 4),
        ("FIFO 复位后：FIFO 被清空，事件是新鲜的（没有积压和延迟）。", 13),
        ("此时事件不再是从满 FIFO 中顺序读出，Wrap Tracking 的前提不成立！", 13, True, RED),
        ("", 6),
        ("保护：检测到 UTC_JUMP（utc_tail 跳跃 >3s = FIFO 复位）后，设 fifo_reset_no_wt = true，", 13),
        ("阻止 Wrap Tracking 激活。直到新的 SEC 锚点被接受时重置。", 13),
        ("此后改用路径 2（过期锚路径，用 utc_tail 估算 — 因为新鲜事件的 utc_tail 是可靠的）。", 13),
    ])


# ── Slide 18: Path 2 - Stale anchor after FIFO reset ────────────────────────

def _slide_18_path2_stale_anchor(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "路径 2：FIFO 复位后过期锚",
                  "事件是新鲜的，但锚点来自拥塞时代")

    # Scenario
    add_rich_textbox(slide, Inches(0.5), Inches(1.2), Inches(5.8), Inches(3.0), [
        ("场景：", 15, True, ORANGE),
        (""),
        ("FIFO 复位清空缓冲区。", 13),
        ("复位后的事件是新鲜的（无 FIFO 延迟）。", 13),
        ("但锚点是过期的（来自拥塞时代，", 13),
        ("  已过多次回绕）。", 13),
        (""),
        ("无法使用 wrap tracking：", 13, True, RED),
        ("  事件是新鲜的，非拥塞状态。FIFO 未满。", 12),
        ("  连续包的中位数不一定相邻。", 12),
        (""),
        ("可以使用 utc_tail：", 13, True, GREEN),
        ("  事件是新鲜的，utc_tail 是正确的！", 12),
        ("  utc_tail 准确反映事件时间。", 12),
    ])

    # Algorithm
    add_rich_textbox(slide, Inches(0.5), Inches(4.3), Inches(5.8), Inches(2.8), [
        ("算法：三选一估计", 14, True),
        (""),
        ("elapsed = utc_tail - anchor_met", 12),
        ("raw = (p_median - p_anchor) x 2us", 12),
        (""),
        ("n_base = round((elapsed - raw) / WRAP_PERIOD)", 12, True, BLUE),
        (""),
        ("尝试 n_base-1、n_base、n_base+1", 12),
        ("选择使时间残差最小的那个。", 12),
        (""),
        ("为什么三选一：utc_tail 有小偏移", 12, False, DARK_GRAY),
        ("（MCU 处理时间），取整可能偏差 1。", 12, False, DARK_GRAY),
    ])

    # Right: fifo_reset_no_wt flag
    add_box(slide, Inches(6.8), Inches(1.2), Inches(5.8), Inches(2.5),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.0), Inches(1.3), Inches(5.4), Inches(2.2), [
        ("fifo_reset_no_wt 标志", 14, True, GREEN),
        (""),
        ("FIFO 复位后，wrap tracking 不得重新激活。", 13),
        ("事件是新鲜的 \u2014 wrap tracking 的假设", 12),
        ("（FIFO 始终满载、顺序读取）不成立。", 12),
        (""),
        ("检测到 UTC_JUMP 时设 fifo_reset_no_wt = true。", 12),
        ("接受新 SEC 锚点时重置为 false。", 12),
    ])

    # Trigger condition
    add_box(slide, Inches(6.8), Inches(4.0), Inches(5.8), Inches(1.5),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, Inches(7.0), Inches(4.1), Inches(5.4), Inches(1.3), [
        ("触发条件", 14, True, BLUE),
        (""),
        ("!anchor_is_recent（过期）", 12),
        ("  AND !use_wrap_tracking", 12),
        ("  AND elapsed >= 1.5 x WRAP_PERIOD", 12),
    ])

    # FIFO reset detection
    add_box(slide, Inches(6.8), Inches(5.8), Inches(5.8), Inches(1.3),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(5.9), Inches(5.4), Inches(1.1), [
        ("FIFO 复位检测：UTC_JUMP", 14, True, ORANGE),
        (""),
        ("utc_tail 向前跳变 > 3 秒 = FIFO 复位。", 12),
        ("MCU 清空了 FIFO；所有拥塞状态失效。", 12),
    ])


# ── Slide 19: Backward SEC correction ───────────────────────────────────────

def _slide_19a_stale_anchor_problem(prs):
    """为什么过期锚点会导致 ±1 wrap 错误 — 用具体数字解释"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "FIFO 复位后的过期锚点 — 问题详解",
                  "为什么旧锚点的 round() 取整容易出错？")

    # Left: 时间线 + 具体数字
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(3.0), [
        ("时间线上发生了什么", 16, True, RED),
        ("", 6),
        ("T=670.0s  最后一个 SEC 被接受为锚点", 12),
        ("          anchor_MET=670.0, anchor_ptime=200000", 12, False, DARK_GRAY),
        ("          Wrap Tracking 开始计数", 12, False, DARK_GRAY),
        ("", 4),
        ("T=671s  wrap_count=1  (正常工作)", 12),
        ("T=672s  wrap_count=2", 12),
        ("T=673s  wrap_count=3", 12),
        ("", 4),
        ("T=674s  FIFO 复位！清空所有积压数据", 12, True, RED),
        ("        Wrap Tracking 被关闭 (fifo_reset_no_wt=true)", 12),
        ("", 4),
        ("T=674.01s  新鲜事件开始进入空 FIFO", 12, True, GREEN),
        ("           MCU 读出新鲜包 — 但锚点还是 T=670s 的！", 12, True, RED),
        ("           elapsed = 674.01 - 670.0 = 4.01s ~ 3.82 个 wrap", 12),
    ])

    # Right: 为什么 round() 出错
    add_box(slide, Inches(6.8), Inches(1.15), Inches(6.2), Inches(3.0),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(2.8), [
        ("过期锚路径（路径2）的 n_est 计算", 16, True, ORANGE),
        ("", 6),
        ("n_est = round( (utc_tail - anchor_MET", 13, True, DARK),
        ("         - (p_median - p_anchor) * 2us) / WRAP_PERIOD )", 13, True, DARK),
        ("", 6),
        ("代入数字：", 13),
        ("  utc_tail = 674.05s（新鲜事件，准确）", 12),
        ("  anchor_MET = 670.0s（准确）", 12),
        ("  p_median=350000, p_anchor=200000", 12),
        ("", 4),
        ("  分子 = 674.05 - 670.0 - 150000*2us = 4.05 - 0.30 = 3.75", 12),
        ("  n_est = round(3.75 / 1.0486) = round(3.576)", 12, True, DARK),
        ("", 4),
        ("  → 可能是 4... 但 utc_tail 有毫秒级误差", 12),
        ("  → 实际可能是 round(3.48) = 3 ← 差了 1！", 12, True, RED),
    ])

    # Bottom left: 为什么「一整批」都错
    add_box(slide, Inches(0.4), Inches(4.35), Inches(6.2), Inches(3.0),
            "", fill_color=LIGHT_RED, border_color=RED)
    add_rich_textbox(slide, Inches(0.6), Inches(4.4), Inches(5.8), Inches(2.8), [
        ("为什么是「一整批」都错？（-96%）", 16, True, RED),
        ("", 6),
        ("FIFO 复位后到新鲜 SEC 到来（~1秒），可能有十几个包。", 13),
        ("这些包全都：", 13),
        ("  - 用同一个旧锚点（T=670s）", 12),
        ("  - elapsed 差不多（都在 ~4s 左右）", 12),
        ("  - n_est 参数差不多（都在 ~3.5 附近）", 12),
        ("", 6),
        ("如果取整翻转了，整批都差 1 个 wrap：", 13, True, RED),
        ("  应在 T+249s 的 ~10000 个事件 → 全部放到 T+248s", 12),
        ("  T+249 只剩 402 个（边缘碰巧取整对的）→ -96%", 12, True, RED),
    ])

    # Bottom right: 距离 vs 错误率
    add_box(slide, Inches(6.8), Inches(4.35), Inches(6.2), Inches(3.0),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(7.0), Inches(4.4), Inches(5.8), Inches(2.8), [
        ("锚点越远 → 取整越不稳定", 16, True, PURPLE),
        ("", 6),
        ("锚点距离   n_est          离 x.5 多远    出错概率", 13, True, DARK_GRAY),
        ("", 4),
        ("~1s 前    round(0.95)=1   离 0.5 很远    极低 ✓", 12, False, GREEN),
        ("~2s 前    round(1.90)=2   离 1.5 较远    低 ✓", 12, False, GREEN),
        ("~4s 前    round(3.58)=4   离 3.5 很近    高! ✗", 12, False, RED),
        ("~6s 前    round(5.52)=6   离 5.5 很近    很高! ✗", 12, False, RED),
        ("", 6),
        ("结论：过期锚点（~4s）的 round() 参数容易落在", 13),
        ("x.5 附近 → 取整不稳定 → 整批偏移 ±1 wrap", 13, True, RED),
    ])


def _slide_19b_backward_solution(prs):
    """解决方案：等新鲜 SEC 来当近距离锚点"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "后向 SEC 修正 — 解决方案",
                  "用近距离的新鲜锚点替代远距离的过期锚点")

    # Timeline diagram
    tl_y = Inches(1.5)
    tl_h = Inches(0.55)
    add_line(slide, Inches(0.3), tl_y + tl_h/2, Inches(12.8), tl_y + tl_h/2,
             color=DARK_GRAY, width=Pt(2))

    draw_bar(slide, Inches(0.3), tl_y, Inches(1.5), tl_h, BLUE,
             label="拥塞包\n(Wrap Tracking)", font_size=8)
    add_line(slide, Inches(2.0), tl_y - Inches(0.1), Inches(2.0), tl_y + tl_h + Inches(0.1),
             color=RED, width=Pt(3))
    add_textbox(slide, Inches(1.8), tl_y - Inches(0.35), Inches(0.5), Inches(0.25),
                "FIFO 复位", font_size=8, bold=True, color=RED, alignment=PP_ALIGN.CENTER)

    draw_bar(slide, Inches(2.2), tl_y, Inches(4.5), tl_h, LIGHT_ORANGE,
             label="待处理事件 — 暂不提交最终 MET，缓存到 pending 列表",
             font_size=9, text_color=DARK, border_color=ORANGE)
    draw_bar(slide, Inches(6.9), tl_y, Inches(1.2), tl_h, GREEN,
             label="新鲜 SEC ✓", font_size=9)
    draw_bar(slide, Inches(8.3), tl_y, Inches(4.5), tl_h, LIGHT_GREEN,
             label="正常包（用新鲜锚点）", font_size=9, text_color=DARK, border_color=GREEN)

    # 距离对比
    add_rich_textbox(slide, Inches(0.4), Inches(2.3), Inches(6.2), Inches(2.5), [
        ("为什么换成新鲜 SEC 就好了？", 18, True, GREEN),
        ("", 6),
        ("旧锚点距离 pending 事件：~4 秒（~3.8 个 wrap）", 14),
        ("  n_est = round(3.58) → 离 3.5 很近 → 容易翻转 ✗", 13, True, RED),
        ("", 8),
        ("新鲜 SEC 距离 pending 事件：~0.5-1.0 秒（~0.5-0.9 个 wrap）", 14),
        ("  n_est = round(0.90) → 离 0.5 较远 → 几乎不会错 ✓", 13, True, GREEN),
        ("", 8),
        ("从「4 秒远的模糊锚点」变成「~1 秒近的可靠锚点」", 14, True, DARK),
    ])

    # 具体步骤
    add_box(slide, Inches(6.8), Inches(2.3), Inches(6.2), Inches(2.5),
            "", fill_color=RGBColor(0xF9, 0xF5, 0xF0), border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(2.35), Inches(5.8), Inches(2.3), [
        ("算法步骤", 16, True, ORANGE),
        ("", 6),
        ("1. FIFO 复位后，路径 2 计算 pending 事件的临时 MET", 13),
        ("   （可能差 ±1 wrap，但先存着）", 12, False, DARK_GRAY),
        ("", 4),
        ("2. 将 pending 事件缓存到 fifo_reset_pending 列表", 13),
        ("", 4),
        ("3. 等新鲜 SEC 到来（下一个整秒边界，~0.5-1.0s 后）", 13),
        ("", 4),
        ("4. 用新鲜 SEC 作为锚点，反向重算所有 pending 事件", 13),
        ("   的 n_wraps（三阶段修正，详见下一页）", 12, False, DARK_GRAY),
    ])

    # Stale batch
    add_box(slide, Inches(0.4), Inches(5.0), Inches(6.2), Inches(2.2),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(0.6), Inches(5.05), Inches(5.8), Inches(2.0), [
        ("特殊情况：连续两次 FIFO 复位", 14, True, PURPLE),
        ("", 6),
        ("如果两次复位之间没等到新鲜 SEC：", 13),
        ("", 4),
        ("T=674s  第一次复位 → 开始缓存 pending", 12),
        ("T=674.5s  第二次复位！还没等到 SEC！", 12, True, RED),
        ("  → 把第一批 pending 保存到 stale_batches", 12),
        ("  → 重新开始新的 pending 列表", 12),
        ("T=675.0s  新鲜 SEC → 先处理 stale_batches 再处理 pending", 12, True, GREEN),
    ])

    # Result
    add_box(slide, Inches(6.8), Inches(5.0), Inches(6.2), Inches(2.2),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.0), Inches(5.05), Inches(5.8), Inches(2.0), [
        ("实际效果（221009A Box B T+249s）", 16, True, GREEN),
        ("", 8),
        ("修正前（用过期锚点）：", 14, True, RED),
        ("  找到 402 / 9960 个事件（-96%）", 13),
        ("  其余 9558 个被错放到 T+248s（差 1 wrap）", 13),
        ("", 6),
        ("修正后（用新鲜 SEC 反向重算）：", 14, True, GREEN),
        ("  找到 10065 / 9960 个事件（+1.1%）✓", 13),
        ("  多出的 ~100 个是 1K 管线额外过滤掉的", 12, False, DARK_GRAY),
    ])


# ── Slide 20: Backward SEC correction - Three phases ────────────────────────

def _slide_20_backward_phases(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "反向 SEC 修正：三阶段修正",
                  "反向刷新如何解决回绕模糊")

    phase_w = Inches(3.8)
    phase_h = Inches(5.0)
    phase_y = Inches(1.3)
    gap = Inches(0.3)

    # Phase 1
    x1 = Inches(0.5)
    add_box(slide, x1, phase_y, phase_w, phase_h,
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, x1 + Inches(0.15), phase_y + Inches(0.1),
                     phase_w - Inches(0.3), phase_h - Inches(0.2), [
        ("阶段 1：置信度分类", 13, True, BLUE),
        (""),
        ("使用新鲜 SEC 为每个缓存包", 12),
        ("计算反向 n_wraps。", 12),
        (""),
        ("estimate_packet_wraps_signed:", 12, True),
        ("  n_est = round((elapsed - raw) /", 11),
        ("          WRAP_PERIOD)", 11),
        ("  允许负数 n（事件在锚点之前）", 11),
        (""),
        ("返回 (n_wraps, margin)：", 12, True),
        ("  margin = (次优 - 最优) / (2*WRAP)", 11),
        ("  0 = 完全模糊", 11),
        ("  0.5 = 完全确信", 11),
        (""),
        ("判定：", 12, True, GREEN),
        ("  margin > 0.05：确信", 12, True, GREEN),
        ("    直接使用反向 n_wraps", 11),
        (""),
        ("  margin <= 0.05：模糊", 12, True, RED),
        ("    进入阶段 2", 11),
    ])

    # Phase 2
    x2 = x1 + phase_w + gap
    add_box(slide, x2, phase_y, phase_w, phase_h,
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, x2 + Inches(0.15), phase_y + Inches(0.1),
                     phase_w - Inches(0.3), phase_h - Inches(0.2), [
        ("阶段 2：正向-反向交叉校验", 13, True, ORANGE),
        (""),
        ("对模糊事件，比较两个方向：", 12),
        (""),
        ("正向（过期锚点）：", 12, True),
        ("  biased_met = 已计算的 MET", 11),
        ("  （可能偏差 ~1 WRAP，但给出", 11),
        ("  近似位置）", 11),
        (""),
        ("反向（新鲜锚点）：", 12, True),
        ("  met_base, met_alt = 两个候选值", 11),
        ("  （相差 +/- WRAP_PERIOD）", 11),
        (""),
        ("关键洞察：", 12, True, GREEN),
        ("过期和新鲜锚点有不同的", 11),
        ("anchor_ptime 值，因此它们的回绕", 11),
        ("边界位于不同位置。", 11),
        ("当反向模糊时，正向通常", 11),
        ("能给出正确方向。", 11),
        (""),
        ("选择最接近 biased_met", 12, True),
        ("（正向估计）的反向候选值。", 12, True),
    ])

    # Phase 3
    x3 = x2 + phase_w + gap
    add_box(slide, x3, phase_y, phase_w, phase_h,
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, x3 + Inches(0.15), phase_y + Inches(0.1),
                     phase_w - Inches(0.3), phase_h - Inches(0.2), [
        ("阶段 3：应用修正", 13, True, GREEN),
        (""),
        ("将修正后的 n_wraps 应用于", 12),
        ("所有待处理事件，使用：", 12),
        ("  compute_met_with_base_wraps()", 11),
        (""),
        ("将修正后的包标记为", 12),
        ("  backward_flushed = true", 12, True),
        ("防止后续遍次对其进行", 12),
        ("\u201c修正\u201d。", 12),
        (""),
        ("过期批次机制：", 13, True, PURPLE),
        (""),
        ("若两次 FIFO 复位之间", 12),
        ("没有新鲜 SEC：", 12),
        (""),
        ("  第 1 批：保存到 stale_batches", 11),
        ("  第 2 批：当前 pending", 11),
        (""),
        ("新鲜 SEC 最终到达时：", 11),
        ("  先处理所有过期批次，", 11),
        ("  再处理当前批次。", 11),
        (""),
        ("防止第 1 批事件丢失。", 12, True, RED),
    ])

    # Bottom note
    add_textbox(slide, Inches(0.5), Inches(6.5), Inches(12.0), Inches(0.5),
                "反向修正后：第 2/3/4 遍跳过 backward_flushed 包，"
                "以避免撤销精确的修正结果。",
                font_size=12, bold=True, color=DARK_GRAY)


# ── Slide 21: Double ambiguity problem ───────────────────────────────────────

def _slide_21_double_ambiguity(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "已知局限：双重模糊",
                  "当两个方向同时模糊时")

    # Problem explanation
    add_rich_textbox(slide, Inches(0.5), Inches(1.2), Inches(5.8), Inches(3.5), [
        ("何时发生？", 15, True, PURPLE),
        (""),
        ("正向-反向交叉校验依赖于过期和新鲜", 13),
        ("锚点具有不同的回绕边界。", 13),
        (""),
        ("但若 anchor_ptime（过期）和 anchor_ptime（新鲜）", 13),
        ("恰好接近，它们的回绕边界重合。", 13),
        (""),
        ("结果：", 14, True, RED),
        ("  两个方向同时模糊。", 13),
        ("  err_a ~ err_b ~ WRAP_PERIOD", 13, True),
        ("  阶段 2 交叉校验无法解决。", 13),
    ])

    # Impact
    add_box(slide, Inches(0.5), Inches(4.9), Inches(5.8), Inches(2.0),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(0.7), Inches(5.0), Inches(5.4), Inches(1.8), [
        ("影响（GRB 221009A Box B）：", 14, True, ORANGE),
        (""),
        ("T+277/278：~3000 个事件仍有 ~1 WRAP 偏移", 13),
        (""),
        ("在 ~4,000,000 个总事件中：", 13),
        ("  3000 / 4,000,000 = 0.075% 受影响", 13, True),
        (""),
        ("对科学分析影响极小。", 13, False, DARK_GRAY),
    ])

    # Why it can't be fixed
    add_rich_textbox(slide, Inches(6.8), Inches(1.2), Inches(5.8), Inches(3.2), [
        ("为什么无法解决：", 15, True, RED),
        (""),
        ("模糊性是根本性的：", 13),
        (""),
        ("  - 19 位 ptime 具有固有的模运算模糊性", 12),
        ("  - 两个可用锚点（过期 + 新鲜）的", 12),
        ("    回绕边界位于相同的 ptime 区域", 12),
        ("  - 数据中不存在其他时间参考", 12),
        (""),
        ("要解决需要：", 13, True, DARK_GRAY),
        ("  - 外部时间参考（如 GPS）", 12, False, DARK_GRAY),
        ("  - 更高精度计数器（>19 位）", 12, False, DARK_GRAY),
        ("  - 数据流中的额外 SEC 事件", 12, False, DARK_GRAY),
        (""),
        ("HXMT HE 1B 数据中均不具备以上条件。", 12),
    ])

    # Summary box
    add_box(slide, Inches(6.8), Inches(4.7), Inches(5.8), Inches(2.2),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.0), Inches(4.8), Inches(5.4), Inches(2.0), [
        ("总体评估", 14, True, GREEN),
        (""),
        ("四遍处理管线在极端饱和条件下", 13),
        ("正确重建 > 99.9% 的事件。", 13, True, GREEN),
        (""),
        ("剩余 0.075% 的双重模糊事件", 12),
        ("局限于窄时间窗口，在典型分档下", 12),
        ("不影响光变曲线形态。", 12),
    ])
