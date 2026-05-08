"""Slides 1-10: Introduction and hardware data path."""

from pptx_helpers import *


def add_slides(prs):
    _slide_01_title(prs)
    _slide_02_outline(prs)
    _slide_03_he_overview(prs)
    _slide_04_1b_vs_1k(prs)
    _slide_05_data_path(prs)
    _slide_06_mcu_deep_dive(prs)
    _slide_07_ccsds_format(prs)
    _slide_08_fifo_a(prs)
    _slide_09_silent_drop(prs)
    _slide_10_fifo_reset(prs)


# ── Slide 1 ────────────────────────────────────────────────────────────────────
def _slide_01_title(prs):
    layout = prs.slide_layouts[6]
    slide = prs.slides.add_slide(layout)
    bg = slide.background.fill
    bg.solid()
    bg.fore_color.rgb = TITLE_BG

    add_textbox(slide, Inches(1.0), Inches(2.0), Inches(11.33), Inches(1.5),
                "慧眼卫星高能望远镜 1B 级数据\n饱和分析与光变曲线重建",
                font_size=40, bold=True, color=WHITE, alignment=PP_ALIGN.CENTER)

    add_textbox(slide, Inches(1.0), Inches(4.0), Inches(11.33), Inches(0.6),
                "郭昊轩",
                font_size=24, color=WARM_LIGHT, alignment=PP_ALIGN.CENTER)

    add_textbox(slide, Inches(1.0), Inches(4.6), Inches(11.33), Inches(0.6),
                "中国科学院高能物理研究所",
                font_size=18, color=WARM_DESC, alignment=PP_ALIGN.CENTER)


# ── Slide 2 ────────────────────────────────────────────────────────────────────
def _slide_02_outline(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "报告提纲")

    sections = [
        ("1", "背景与动机", "HXMT HE 概述、1B vs 1K 数据、科学需求"),
        ("2", "硬件数据通路与饱和机制", "MCU 架构、FIFO 行为、两种丢包模式"),
        ("3", "时间重建算法", "ptime 回绕处理、Second 锚点、跨包拼接"),
        ("4", "饱和检测与光变曲线重建", "FIFO 复位检测、Silent drop 检测、深度饱和重建"),
        ("5", "验证结果", "GRB 200415A / 221009A / 260226A 光变曲线对比"),
        ("6", "结论与展望", ""),
    ]

    for i, (num, title, desc) in enumerate(sections):
        y = Inches(1.5) + Inches(i * 0.9)
        # Number circle
        add_box(slide, Inches(1.0), y, Inches(0.6), Inches(0.6), num,
                fill_color=BLUE, border_color=BLUE, font_size=20, bold=True, text_color=WHITE)
        # Title
        add_textbox(slide, Inches(1.8), y, Inches(4.0), Inches(0.6),
                    title, font_size=22, bold=True, color=DARK)
        # Description
        if desc:
            add_textbox(slide, Inches(1.8), y + Inches(0.35), Inches(9.0), Inches(0.5),
                        desc, font_size=14, color=GRAY)


# ── Slide 3 ────────────────────────────────────────────────────────────────────
def _slide_03_he_overview(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "背景：HXMT 高能望远镜 (HE)")

    # Left column - HE specifications
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(4.0), Inches(0.5),
                "HE 仪器参数", font_size=20, bold=True)

    spec_lines = [
        ("能量范围：20 - 250 keV", 16, False, DARK),
        ("总探测面积：5100 cm²", 16, False, DARK),
        ("探测器：18 个 NaI(Tl)/CsI(Na)", 16, False, DARK),
        ("分为 3 个机箱（各 6 个探测器）", 16, False, DARK),
        ("每机箱独立数据通路", 16, False, DARK),
        ("硬件时间分辨率：2 \u03bcs", 16, False, DARK),
        ("每包最大事例数：109", 16, False, DARK),
    ]
    add_rich_textbox(slide, Inches(0.5), Inches(1.9), Inches(5.5), Inches(3.5),
                     spec_lines, default_size=16)

    # Right column - WHY saturation matters
    add_textbox(slide, Inches(6.5), Inches(1.3), Inches(6.0), Inches(0.5),
                "为什么需要处理饱和？", font_size=20, bold=True, color=RED)

    issue_data = [
        ("光变曲线不完整", "无法准确测量 GRB 时间结构（上升沿、峰值、变化时标）"),
        ("计数率偏低", "饱和期间测得计数率远低于真实值，能谱分析出错"),
        ("1K 管线过滤过激", "标准管线在 FIFO 复位边界做激进过滤，额外丢失有效数据"),
        ("1B 是唯一完整数据源", "保留了所有硬件时间戳和 CRC 通过的原始事例"),
    ]

    for i, (title, desc) in enumerate(issue_data):
        y = Inches(1.9) + Inches(i * 1.1)
        add_box(slide, Inches(6.5), y, Inches(6.0), Inches(0.9),
                "", fill_color=LIGHT_RED, border_color=RED)
        add_textbox(slide, Inches(6.7), y + Inches(0.05), Inches(5.6), Inches(0.35),
                    title, font_size=15, bold=True, color=RED)
        add_textbox(slide, Inches(6.7), y + Inches(0.4), Inches(5.6), Inches(0.45),
                    desc, font_size=13, color=DARK)


# ── Slide 4 ────────────────────────────────────────────────────────────────────
def _slide_04_1b_vs_1k(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "1B 原始数据 vs 1K 标准数据")

    # 1B column
    add_box(slide, Inches(0.5), Inches(1.4), Inches(5.8), Inches(0.6),
            "1B 数据（原始遥测）", fill_color=BLUE, border_color=BLUE,
            font_size=18, bold=True, text_color=WHITE)

    lines_1b = [
        ("最底层原始数据，完整 CCSDS 包结构", 15, False, DARK),
        ("保留所有硬件时间戳（ptime 原始值）", 15, False, DARK),
        ("包含所有通过 CRC 校验的事例", 15, False, DARK),
        ("未做任何时间校正或事件筛选", 15, False, DARK),
        ("完整保留饱和区信息", 15, False, GREEN),
    ]
    add_rich_textbox(slide, Inches(0.7), Inches(2.2), Inches(5.4), Inches(2.5),
                     lines_1b, default_size=15)

    # 1K column
    add_box(slide, Inches(7.0), Inches(1.4), Inches(5.8), Inches(0.6),
            "1K 数据（标准管线）", fill_color=PURPLE, border_color=PURPLE,
            font_size=18, bold=True, text_color=WHITE)

    lines_1k = [
        ("经标准管线处理，时间校正+事件筛选", 15, False, DARK),
        ("转为 MET 绝对时间", 15, False, DARK),
        ("在 FIFO 复位边界做激进过滤", 15, False, RED),
        ("管线不知道事件是否来自饱和区", 15, False, RED),
        ("盲目过滤 \u2192 丢失额外有效数据", 15, False, RED),
    ]
    add_rich_textbox(slide, Inches(7.2), Inches(2.2), Inches(5.4), Inches(2.5),
                     lines_1k, default_size=15)

    # Why 1B is better
    add_box(slide, Inches(0.5), Inches(4.8), Inches(12.3), Inches(0.6),
            "1B 保留原始信息 \u2192 可以做更精细的饱和判断和光变曲线重建",
            fill_color=LIGHT_GREEN, border_color=GREEN, font_size=16, bold=True, text_color=DARK)

    # 4 objectives
    add_textbox(slide, Inches(0.5), Inches(5.7), Inches(12.0), Inches(0.4),
                "本工作四个目标：", font_size=16, bold=True, color=DARK)

    objectives = [
        "\u2460 从 1B 数据精确重建事件绝对时间",
        "\u2461 检测 FIFO 复位和 Silent drop 两种饱和模式",
        "\u2462 重建饱和区光变曲线（估计丢失计数）",
        "\u2463 验证重建结果与 1K 数据的一致性",
    ]
    for i, obj in enumerate(objectives):
        x = Inches(0.5) + Inches(i * 3.1)
        add_box(slide, x, Inches(6.2), Inches(2.9), Inches(0.7),
                obj, fill_color=LIGHT_BLUE, border_color=BLUE, font_size=13,
                bold=False, text_color=DARK)


# ── Slide 5 ────────────────────────────────────────────────────────────────────
def _slide_05_data_path(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "硬件数据通路总览")

    # Define boxes for the data path
    boxes = [
        ("NaI/CsI\n探测器", Inches(0.3),  LIGHT_GREEN, GREEN),
        ("ASIC\n成形/甄别",  Inches(2.2),  LIGHT_BLUE, BLUE),
        ("FPGA\n时间标记/CRC", Inches(4.1), LIGHT_BLUE, BLUE),
        ("FIFO A\nM67204H",  Inches(6.0),  LIGHT_RED, RED),
        ("MCU\n8051",         Inches(7.9),  LIGHT_ORANGE, ORANGE),
        ("FIFO B",            Inches(9.8),  LIGHT_BLUE, BLUE),
        ("1553B\n下传",       Inches(11.4), LIGHT_PURPLE, PURPLE),
    ]

    box_y = Inches(2.2)
    box_w = Inches(1.6)
    box_h = Inches(1.2)

    for label, x, fill, border in boxes:
        add_box(slide, x, box_y, box_w, box_h, label,
                fill_color=fill, border_color=border, font_size=14, bold=True)

    # Arrows between boxes
    for i in range(len(boxes) - 1):
        x1 = boxes[i][1] + box_w
        x2 = boxes[i + 1][1]
        y_mid = box_y + box_h / 2
        add_arrow(slide, x1, y_mid, x2, y_mid, color=DARK_GRAY, width=Pt(2.5))

    # Annotations below
    annotations = [
        (Inches(0.3), "X射线光子产生\n电信号"),
        (Inches(2.2), "能量甄别\n阈值判断"),
        (Inches(4.1), "19-bit ptime\n4-bit CRC"),
        (Inches(6.0), "4096B 硬件缓冲\n\u2605 饱和瓶颈"),
        (Inches(7.9), "单线程轮询\n109 evt/包"),
        (Inches(9.8), "输出缓冲"),
        (Inches(11.4), "CCSDS 包\n下行传输"),
    ]

    for x, text in annotations:
        add_textbox(slide, x, Inches(3.7), Inches(1.6), Inches(1.0),
                    text, font_size=11, color=DARK_GRAY, alignment=PP_ALIGN.CENTER)

    # Highlight FIFO A as bottleneck
    add_textbox(slide, Inches(5.5), Inches(4.9), Inches(2.6), Inches(0.5),
                "\u2191 饱和的根本原因：FIFO A 容量有限（~455 事例）",
                font_size=13, bold=True, color=RED, alignment=PP_ALIGN.CENTER)

    # Throughput note
    add_box(slide, Inches(0.5), Inches(5.8), Inches(12.3), Inches(1.0),
            "关键约束：MCU 每 ~7ms 读取一包（109 事例）\u2192 理论最大吞吐率 \u2248 15571 evt/s\n"
            "当源率超过此值时，FIFO A 写入速率 > 读出速率 \u2192 逐渐积压 \u2192 触发饱和",
            fill_color=LIGHT_ORANGE, border_color=ORANGE, font_size=14, text_color=DARK)


# ── Slide 6 ────────────────────────────────────────────────────────────────────
def _slide_06_mcu_deep_dive(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "MCU (8051) 主循环详解")

    # Left: code-like structure
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(5.5), Inches(0.4),
                "MCU 主循环伪代码", font_size=18, bold=True)

    code_lines = [
        ("while (1) {", 15, True, DARK),
        ("    ResetWDI();           // 看门狗", 14, False, GRAY),
        ("    SearchStack();        // 搜索 0x5A", 14, False, GRAY),
        ("    FIFOAFullReset();     // \u2605 检查满标志", 14, False, RED),
        ("    HandleEngineerLVDS(); // ~2ms", 14, False, DARK_GRAY),
        ("    HandlePhysicalLVDS(); // ~7ms \u2605", 14, False, RED),
        ("}", 15, True, DARK),
        ("", 12, False, DARK),
        ("HandlePhysicalLVDS() {", 15, True, DARK),
        ("    for i in 0..109:", 14, False, DARK_GRAY),
        ("        ReadFIFOA();  // 读 8 字节", 14, False, DARK_GRAY),
        ("        CheckCRC();   // 4-bit CRC", 14, False, DARK_GRAY),
        ("        PackEvent();  // 写入包缓冲", 14, False, DARK_GRAY),
        ("    SendPacket();  // 发送 CCSDS 包", 14, False, DARK_GRAY),
        ("}", 15, True, DARK),
    ]
    add_rich_textbox(slide, Inches(0.5), Inches(1.8), Inches(5.8), Inches(5.0),
                     code_lines, default_size=14)

    # Right: explanation of problems
    add_textbox(slide, Inches(6.8), Inches(1.3), Inches(6.0), Inches(0.4),
                "为什么这会导致饱和？", font_size=18, bold=True, color=RED)

    problems = [
        ("单线程轮询架构", "MCU 不能并行处理，HandlePhysicalLVDS 执行期间\n无法响应其他事件"),
        ("读取耗时 ~7ms", "读取 109 个事例需要约 7ms，在此期间\nFPGA 持续向 FIFO A 写入新事例"),
        ("FIFO 积压", "如果源率高，7ms 内写入量 > 109 事例\nFIFO 水位持续上升"),
        ("回到主循环才检查", "HandlePhysicalLVDS 完成后才回到主循环\n此时 FIFOAFullReset() 才能检查满标志"),
    ]

    for i, (title, desc) in enumerate(problems):
        y = Inches(1.9) + Inches(i * 1.25)
        add_box(slide, Inches(6.8), y, Inches(6.0), Inches(1.05),
                "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
        add_textbox(slide, Inches(7.0), y + Inches(0.05), Inches(5.6), Inches(0.3),
                    title, font_size=15, bold=True, color=ORANGE)
        add_textbox(slide, Inches(7.0), y + Inches(0.4), Inches(5.6), Inches(0.6),
                    desc, font_size=13, color=DARK)

    # Bottom: throughput calculation
    add_box(slide, Inches(6.8), Inches(6.0), Inches(6.0), Inches(0.8),
            "理论最大处理率 = 109 evt / 7ms \u2248 15,571 evt/s\n"
            "GRB 峰值可达数万~数十万 evt/s \u2192 必然饱和",
            fill_color=LIGHT_RED, border_color=RED, font_size=14, bold=True, text_color=DARK)


# ── Slide 7 ────────────────────────────────────────────────────────────────────
def _slide_07_ccsds_format(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "CCSDS 包结构与事例格式")

    # Packet structure bar
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(5.0), Inches(0.4),
                "CCSDS 包结构 (882 字节)", font_size=18, bold=True)

    bar_y = Inches(1.9)
    bar_h = Inches(0.7)
    total_w = Inches(12.0)

    # Header 6B
    hdr_w = Inches(0.8)
    draw_bar(slide, Inches(0.5), bar_y, hdr_w, bar_h, PURPLE,
             label="包头\n6B", font_size=11, text_color=WHITE)

    # Payload 872B (109 events x 8B)
    payload_w = Inches(9.9)
    draw_bar(slide, Inches(0.5) + hdr_w, bar_y, payload_w, bar_h, BLUE,
             label="载荷 872B (109 \u00d7 8B 事例)", font_size=14, text_color=WHITE)

    # UTC tail 4B
    tail_w = Inches(1.3)
    draw_bar(slide, Inches(0.5) + hdr_w + payload_w, bar_y, tail_w, bar_h, GREEN,
             label="UTC 尾\n4B", font_size=11, text_color=WHITE)

    # Event types
    add_textbox(slide, Inches(0.5), Inches(2.9), Inches(12.0), Inches(0.4),
                "三种事例类型（各 8 字节）", font_size=18, bold=True)

    event_types = [
        ("Event（物理事例）", LIGHT_BLUE, BLUE,
         "通道号 (channel) + 19-bit ptime\n分辨率 2\u03bcs，回绕周期 2\u00b9\u2079\u00d72\u03bcs \u2248 1.0486s\n记录探测器实际探测到的 X 射线光子"),
        ("Second（秒标）", LIGHT_GREEN, GREEN,
         "stime (整秒 MET) + ptime\n硬件每秒注入一个，是时间重建的绝对锚点\n提供 ptime \u2192 MET 的映射关系"),
        ("Error（错误事例）", LIGHT_RED, RED,
         "CRC 校验失败的事例\n4-bit CRC 碰撞概率 1/16 \u2248 6.3%\n高计数率时误判比例可能增大"),
    ]

    for i, (title, fill, border, desc) in enumerate(event_types):
        x = Inches(0.5) + Inches(i * 4.2)
        y = Inches(3.5)
        add_box(slide, x, y, Inches(3.9), Inches(0.5),
                title, fill_color=fill, border_color=border, font_size=15, bold=True, text_color=DARK)
        add_textbox(slide, x + Inches(0.1), y + Inches(0.6), Inches(3.7), Inches(1.5),
                    desc, font_size=13, color=DARK)

    # ptime wrap explanation
    add_box(slide, Inches(0.5), Inches(5.8), Inches(12.3), Inches(1.2),
            "",
            fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_textbox(slide, Inches(0.7), Inches(5.85), Inches(11.9), Inches(0.35),
                "ptime 回绕机制", font_size=16, bold=True, color=ORANGE)
    add_textbox(slide, Inches(0.7), Inches(6.25), Inches(11.9), Inches(0.7),
                "19 位计数器最大值 = 524287，每 tick = 2\u03bcs \u2192 回绕周期 = 524288 \u00d7 2\u03bcs \u2248 1.0486 秒\n"
                "即 ptime 每 ~1.05 秒从 524287 回绕到 0，时间重建必须正确处理这些回绕边界",
                font_size=14, color=DARK)


# ── Slide 8 ────────────────────────────────────────────────────────────────────
def _slide_08_fifo_a(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "FIFO A (M67204H) 详细行为")

    # Left: chip specs
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(5.5), Inches(0.4),
                "M67204H 芯片特性", font_size=18, bold=True)

    specs = [
        ("硬件环形缓冲区", 16, True, DARK),
        ("容量：4096 字节 \u2248 455 个事例（9B/evt）", 15, False, DARK),
        ("Full 标志位：FIFOAFull1, FIFOAFull2", 15, False, DARK),
        ("", 8, False, DARK),
        ("\u2605 关键特性：满时不覆盖！", 16, True, RED),
        ("FIFO 满时 FPGA 写入被阻塞", 15, False, RED),
        ("新到达的事例直接丢弃", 15, False, RED),
        ("", 8, False, DARK),
        ("复位方式：ResetFIFOA = 0x00", 15, False, DARK),
        ("没有\"部分清空\"功能，只能全清", 15, False, DARK_GRAY),
        ("\u2192 这是整包丢失的根本原因", 15, True, RED),
    ]
    add_rich_textbox(slide, Inches(0.5), Inches(1.8), Inches(5.8), Inches(4.0),
                     specs, default_size=15)

    # Right: 0x5A sync + operational diagram
    add_textbox(slide, Inches(6.8), Inches(1.3), Inches(6.0), Inches(0.4),
                "操作流程", font_size=18, bold=True)

    flow_boxes = [
        ("FPGA 写入事例\n(带 0x5A 起始标记)", Inches(1.6), LIGHT_GREEN, GREEN),
        ("\u2193 FIFO A 缓冲 \u2193", Inches(2.5), LIGHT_GRAY, GRAY),
        ("MCU SearchStack()\n搜索 0x5A 定位边界", Inches(3.4), LIGHT_BLUE, BLUE),
        ("MCU HandlePhysicalLVDS()\n读取 109 事例", Inches(4.3), LIGHT_BLUE, BLUE),
        ("FIFOAFullReset()\n检查满标志 \u2192 决定是否复位", Inches(5.2), LIGHT_RED, RED),
    ]

    for label, y, fill, border in flow_boxes:
        add_box(slide, Inches(7.0), y, Inches(5.5), Inches(0.7),
                label, fill_color=fill, border_color=border, font_size=13, text_color=DARK)

    # Arrows between flow boxes
    for i in range(len(flow_boxes) - 1):
        y1 = flow_boxes[i][1] + Inches(0.7)
        y2 = flow_boxes[i + 1][1]
        x_mid = Inches(9.75)
        add_arrow(slide, x_mid, y1, x_mid, y2, color=DARK_GRAY, width=Pt(2))

    # Bottom: design rationale
    add_box(slide, Inches(0.5), Inches(6.2), Inches(12.3), Inches(0.8),
            "设计约束：硬件 FIFO 没有\"部分清空\"功能 \u2192 MCU 只能全清重置。"
            "复位后约 455/R_true 秒内 FIFO 从空到满（R=50000 时约 9ms），"
            "主循环一轮仅需微秒级 \u2192 一次复位最多清一次",
            fill_color=LIGHT_GRAY, border_color=GRAY, font_size=13, text_color=DARK)


# ── Slide 9 ────────────────────────────────────────────────────────────────────
def _slide_09_silent_drop(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "饱和模式 1：Silent Drop（静默丢包）")

    # Left: timing diagram
    add_textbox(slide, Inches(0.3), Inches(1.3), Inches(6.0), Inches(0.4),
                "时序示意", font_size=18, bold=True)

    # Timeline base
    tl_left = Inches(0.5)
    tl_y = Inches(3.0)
    tl_w = Inches(5.8)

    # Time axis
    add_line(slide, tl_left, tl_y + Inches(1.4), tl_left + tl_w, tl_y + Inches(1.4),
             color=DARK_GRAY, width=Pt(1.5))
    add_textbox(slide, tl_left + tl_w - Inches(0.5), tl_y + Inches(1.45), Inches(0.5), Inches(0.3),
                "time \u2192", font_size=10, color=DARK_GRAY)

    # FIFO level bar (rising then dropping)
    draw_bar(slide, tl_left, tl_y - Inches(0.2), Inches(1.5), Inches(0.5),
             GREEN, label="FIFO \u5145\u586b\u4e2d", font_size=10)
    draw_bar(slide, tl_left + Inches(1.5), tl_y - Inches(0.7), Inches(1.2), Inches(1.0),
             RED, label="FIFO \u6ee1\n\u4e22\u5305!", font_size=10)
    draw_bar(slide, tl_left + Inches(2.7), tl_y - Inches(0.2), Inches(1.5), Inches(0.5),
             GREEN, label="FIFO \u6062\u590d", font_size=10)
    draw_bar(slide, tl_left + Inches(4.2), tl_y + Inches(0.1), Inches(1.5), Inches(0.2),
             LIGHT_GREEN, label="MCU \u8bfb\u53d6", font_size=10, text_color=DARK)

    # Labels
    add_textbox(slide, tl_left, tl_y + Inches(0.5), Inches(1.5), Inches(0.3),
                "FPGA 写入", font_size=11, color=GREEN, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, tl_left + Inches(1.5), tl_y + Inches(0.5), Inches(1.2), Inches(0.3),
                "\u2605 丢弃区", font_size=11, color=RED, bold=True, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, tl_left + Inches(2.7), tl_y + Inches(0.5), Inches(1.5), Inches(0.3),
                "恢复写入", font_size=11, color=GREEN, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, tl_left + Inches(4.2), tl_y + Inches(0.5), Inches(1.5), Inches(0.3),
                "正常", font_size=11, color=GRAY, alignment=PP_ALIGN.CENTER)

    # Note about check
    add_textbox(slide, tl_left, tl_y + Inches(0.9), Inches(5.8), Inches(0.4),
                "\u2192 HandlePhysicalLVDS 结束后检查 FIFO：已不满 \u2192 不触发复位",
                font_size=12, bold=True, color=DARK_GRAY)

    # Right: detailed explanation
    add_textbox(slide, Inches(6.8), Inches(1.3), Inches(6.0), Inches(0.4),
                "发生机制", font_size=18, bold=True, color=RED)

    explanation_lines = [
        ("写速率略 > 读速率时：", 15, True, DARK),
        ("  7ms 处理期间 FIFO 逐渐填满", 14, False, DARK),
        ("  FIFO 满 \u2192 新事例被 FPGA 丢弃", 14, False, RED),
        ("  MCU 继续读取 \u2192 腾出空间", 14, False, DARK),
        ("  FPGA 恢复写入", 14, False, DARK),
        ("", 6, False, DARK),
        ("回到主循环检查：", 15, True, DARK),
        ("  FIFOAFullReset() 看到 FIFO 不满", 14, False, DARK),
        ("  \u2192 不触发复位！", 14, True, RED),
        ("", 6, False, DARK),
        ("结果：", 15, True, RED),
        ("  丢失数十~数百事例", 14, False, RED),
        ("  包内出现异常大的时间间隔", 14, False, RED),
        ("  包结构完整 \u2192 无法通过包间 gap 检测", 14, False, DARK_GRAY),
        ("", 6, False, DARK),
        ("检测方法：泊松统计", 15, True, GREEN),
        ("  正常间隔 ~10-60\u03bcs", 14, False, GREEN),
        ("  异常间隔 500-6000\u03bcs \u2192 标记为 silent drop", 14, False, GREEN),
    ]
    add_rich_textbox(slide, Inches(6.8), Inches(1.8), Inches(6.0), Inches(5.0),
                     explanation_lines, default_size=14)


# ── Slide 10 ───────────────────────────────────────────────────────────────────
def _slide_10_fifo_reset(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "饱和模式 2：FIFO 复位（整包丢失）")

    # Left: timing diagram
    add_textbox(slide, Inches(0.3), Inches(1.3), Inches(6.0), Inches(0.4),
                "时序示意", font_size=18, bold=True)

    tl_left = Inches(0.5)
    tl_y = Inches(3.0)
    tl_w = Inches(5.8)

    # Time axis
    add_line(slide, tl_left, tl_y + Inches(1.4), tl_left + tl_w, tl_y + Inches(1.4),
             color=DARK_GRAY, width=Pt(1.5))
    add_textbox(slide, tl_left + tl_w - Inches(0.5), tl_y + Inches(1.45), Inches(0.5), Inches(0.3),
                "time \u2192", font_size=10, color=DARK_GRAY)

    # FIFO stays full
    draw_bar(slide, tl_left, tl_y - Inches(0.7), Inches(2.5), Inches(1.0),
             RED, label="FIFO \u6301\u7eed\u6ee1\u8f7d\n\u5199\u5165 >> \u8bfb\u51fa", font_size=10)

    # Reset point
    draw_bar(slide, tl_left + Inches(2.5), tl_y - Inches(0.7), Inches(0.4), Inches(1.0),
             DARK, label="\u590d\u4f4d!", font_size=10, text_color=WHITE)

    # Empty FIFO after reset
    draw_bar(slide, tl_left + Inches(2.9), tl_y + Inches(0.1), Inches(1.2), Inches(0.2),
             LIGHT_GREEN, label="FIFO \u7a7a", font_size=10, text_color=DARK)

    # Refilling
    draw_bar(slide, tl_left + Inches(4.1), tl_y - Inches(0.2), Inches(1.6), Inches(0.5),
             GREEN, label="\u91cd\u65b0\u79ef\u7d2f", font_size=10)

    # Labels
    add_textbox(slide, tl_left, tl_y + Inches(0.5), Inches(2.5), Inches(0.3),
                "7ms \u5185\u5199\u5165\u8fdc\u8d85\u8bfb\u51fa", font_size=11, color=RED,
                alignment=PP_ALIGN.CENTER)
    add_textbox(slide, tl_left + Inches(2.5), tl_y + Inches(0.5), Inches(0.4), Inches(0.3),
                "\u2605", font_size=11, color=DARK, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, tl_left + Inches(2.9), tl_y + Inches(0.5), Inches(2.8), Inches(0.3),
                "\u6e05\u7a7a\u540e\u91cd\u65b0\u79ef\u7d2f", font_size=11, color=GREEN,
                alignment=PP_ALIGN.CENTER)

    add_textbox(slide, tl_left, tl_y + Inches(0.9), Inches(5.8), Inches(0.4),
                "\u2192 HandlePhysicalLVDS \u7ed3\u675f\u540e FIFO \u4ecd\u6ee1 \u2192 FIFOAFullReset() \u89e6\u53d1\u590d\u4f4d",
                font_size=12, bold=True, color=DARK_GRAY)

    # Right: detailed explanation
    add_textbox(slide, Inches(6.8), Inches(1.3), Inches(6.0), Inches(0.4),
                "发生机制", font_size=18, bold=True, color=RED)

    explanation_lines = [
        ("写速率 >> 读速率时：", 15, True, DARK),
        ("  7ms 内 FPGA 写入远超 MCU 读出", 14, False, DARK),
        ("  HandlePhysicalLVDS 完成后 FIFO 仍满", 14, False, RED),
        ("", 6, False, DARK),
        ("FIFOAFullReset() 检测到 Full 标志：", 15, True, DARK),
        ("  执行 ResetFIFOA = 0x00", 14, False, RED),
        ("  清空整个 FIFO 缓冲区", 14, True, RED),
        ("  ~455 事例 + 后续积压事例全部丢失", 14, False, RED),
        ("", 6, False, DARK),
        ("复位后：", 15, True, DARK),
        ("  FIFO 空 \u2192 FPGA 恢复写入", 14, False, DARK),
        ("  重新填满需 455/R_true 秒", 14, False, DARK),
        ("  即使 R=50000 也需 ~9ms", 14, False, DARK_GRAY),
        ("", 6, False, DARK),
        ("结果：", 15, True, RED),
        ("  相邻包之间出现大时间空洞", 14, False, RED),
        ("  可通过包间 gap 检测（与 silent drop 不同）", 14, False, GREEN),
        ("", 6, False, DARK),
        ("为何一次只清一次：", 15, True, DARK),
        ("  复位后 FIFO 空，主循环一轮仅需微秒级", 14, False, DARK_GRAY),
        ("  下次检查时 FIFO 尚未满 \u2192 不会连续复位", 14, False, DARK_GRAY),
    ]
    add_rich_textbox(slide, Inches(6.8), Inches(1.8), Inches(6.0), Inches(5.2),
                     explanation_lines, default_size=14)
