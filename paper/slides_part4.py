"""Slides 31-38: Validation results and conclusions."""

from pptx_helpers import *


def add_slides(prs):
    _slide_31_grb200415a(prs)
    _slide_32_grb260226a(prs)
    _slide_33_grb221009a_overview(prs)
    _slide_34_grb221009a_matching(prs)
    _slide_35_validation_summary(prs)
    _slide_35b_recon_200415a(prs)
    _slide_35c_recon_260226a(prs)
    _slide_35d_recon_221009a(prs)
    _slide_35e_recon_onset(prs)
    _slide_35f_recon_peak(prs)
    _slide_36_limitations(prs)
    _slide_37_conclusions(prs)
    _slide_38_thank_you(prs)


# -- Slide 31 -----------------------------------------------------------------
def _slide_31_grb200415a(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "验证 1：GRB 200415A（轻度饱和）",
                  subtitle="仅峰值处有轻微饱和，作为验证基准")

    # Description
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(12.0), Inches(0.5),
                "仅峰值处有轻微饱和，无 FIFO 复位，无 Silent Drop — 验证基准场景",
                font_size=16, bold=True, color=DARK)

    # Table: Box / 1B events / 1K events / Delta / Delta%
    col_widths = [Inches(1.5), Inches(2.5), Inches(2.5), Inches(2.0), Inches(2.0)]
    rows = [
        ["机箱", "1B 事件数", "1K 事件数", "差值 (Δ)", "差值 (Δ%)"],
        ["Box A", "—", "—", "0", "0.0%"],
        ["Box B", "—", "—", "0", "0.0%"],
        ["Box C", "—", "—", "0", "0.0%"],
    ]
    draw_table(slide, Inches(1.5), Inches(2.1), col_widths, rows,
               row_height=Inches(0.5), font_size=14)

    # Key points
    add_textbox(slide, Inches(0.5), Inches(4.5), Inches(12.0), Inches(0.4),
                "关键结论", font_size=18, bold=True, color=GREEN)

    points = [
        ("所有 1s bin 完美匹配",
         "1B 重建结果与 1K 标准管线逐 bin 对比，差值均为 0，无任何异常 bin"),
        ("Pass 2/3 几乎未触发",
         "轻度饱和下无 FIFO 复位和 ptime 回绕边界问题，算法直接通过 Pass 1 完成"),
        ("证明算法正确性",
         "在无饱和干扰的基准场景下，时间重建精度与标准管线完全一致"),
    ]

    for i, (title, desc) in enumerate(points):
        y = Inches(5.0) + Inches(i * 0.7)
        add_textbox(slide, Inches(0.7), y, Inches(0.4), Inches(0.35),
                    "\u2713", font_size=16, bold=True, color=GREEN, alignment=PP_ALIGN.CENTER)
        add_textbox(slide, Inches(1.1), y, Inches(3.5), Inches(0.35),
                    title, font_size=15, bold=True, color=DARK)
        add_textbox(slide, Inches(4.8), y, Inches(7.5), Inches(0.35),
                    desc, font_size=13, color=DARK_GRAY)


# -- Slide 32 -----------------------------------------------------------------
def _slide_32_grb260226a(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "验证 2：GRB 260226A（中度饱和）",
                  subtitle="T\u2080 \u2248 MET 446726273，有明确 FIFO 复位间隙，无 silent drop")

    # Description
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(12.0), Inches(0.5),
                "有明确 FIFO 复位间隙，无 Silent Drop — 验证回绕边界处理",
                font_size=16, bold=True, color=DARK)

    # Table
    col_widths = [Inches(1.5), Inches(2.5), Inches(2.5), Inches(2.0), Inches(2.0)]
    rows = [
        ["机箱", "1B 事件数", "1K 事件数", "差值 (Δ)", "差值 (Δ%)"],
        ["Box A", "—", "—", "\u22480", "0.0%"],
        ["Box B", "—", "—", "\u22480", "0.0%"],
        ["Box C", "—", "—", "\u22482", "+0.0%"],
    ]
    draw_table(slide, Inches(1.5), Inches(2.1), col_widths, rows,
               row_height=Inches(0.5), font_size=14)

    # Pass 3 trigger info
    add_textbox(slide, Inches(0.5), Inches(4.5), Inches(12.0), Inches(0.4),
                "Pass 3（ptime 回绕边界修正）触发情况", font_size=18, bold=True, color=ORANGE)

    pass3_cols = [Inches(2.0), Inches(3.0), Inches(3.0)]
    pass3_rows = [
        ["机箱", "Pass 3 修正包数", "说明"],
        ["Box A", "7 packets", "回绕边界修正"],
        ["Box B", "14 packets", "回绕边界修正"],
        ["Box C", "13 packets", "回绕边界修正"],
    ]
    draw_table(slide, Inches(2.5), Inches(5.0), pass3_cols, pass3_rows,
               row_height=Inches(0.45), font_size=13, header_bg=ORANGE)

    # Why this matters
    add_box(slide, Inches(0.5), Inches(6.5), Inches(12.3), Inches(0.6),
            "证明 ptime 回绕边界修正有效：中度饱和下部分包跨越回绕边界，Pass 3 精确修正后与 1K 完美匹配",
            fill_color=LIGHT_GREEN, border_color=GREEN, font_size=14, bold=True, text_color=DARK)


# -- Slide 33 -----------------------------------------------------------------
def _slide_33_grb221009a_overview(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "验证 3：GRB 221009A 概览（极端饱和）",
                  subtitle="2022-10-09，史上最亮 GRB，HE 经历严重持续饱和")

    # Event count comparison table
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(6.0), Inches(0.4),
                "1B vs 1K 事件总数对比", font_size=18, bold=True)

    col_widths = [Inches(1.2), Inches(2.0), Inches(2.0), Inches(1.8), Inches(1.5)]
    rows = [
        ["机箱", "1B 事件数", "1K 事件数", "差值 (Δ)", "Δ%"],
        ["Box A", "3,857,418", "3,857,310", "+108", "+0.003%"],
        ["Box B", "3,800,510", "3,797,574", "+2,936", "+0.08%"],
        ["Box C", "3,745,229", "3,619,371", "+125,858", "+3.5%"],
    ]
    draw_table(slide, Inches(0.5), Inches(1.9), col_widths, rows,
               row_height=Inches(0.5), font_size=13)

    # Box C explanation
    add_box(slide, Inches(0.5), Inches(4.3), Inches(8.0), Inches(0.7),
            "Box C 3.5% 差异集中在 T+510s — 1K 管线数据丢失（1B 多 ~12 万事件），非 1B 重建错误",
            fill_color=LIGHT_RED, border_color=RED, font_size=13, text_color=DARK)

    # Saturation statistics
    add_textbox(slide, Inches(0.5), Inches(5.3), Inches(12.0), Inches(0.4),
                "饱和统计", font_size=18, bold=True, color=RED)

    stats = [
        ("FIFO 复位", "每机箱数百个复位间隙\n持续时间覆盖整个 GRB 峰值区域", LIGHT_RED, RED),
        ("Silent Drop", "138 个候选\nA:42  B:42  C:39\n分布在约 90 个包中", LIGHT_ORANGE, ORANGE),
        ("饱和持续时间", "约 1000 秒\n远超 200415A 和 260226A", LIGHT_PURPLE, PURPLE),
    ]

    for i, (title, desc, fill, border) in enumerate(stats):
        x = Inches(0.5) + Inches(i * 4.2)
        add_box(slide, x, Inches(5.8), Inches(3.9), Inches(1.3),
                "", fill_color=fill, border_color=border)
        add_textbox(slide, x + Inches(0.15), Inches(5.85), Inches(3.6), Inches(0.3),
                    title, font_size=15, bold=True, color=border)
        add_textbox(slide, x + Inches(0.15), Inches(6.2), Inches(3.6), Inches(0.8),
                    desc, font_size=12, color=DARK)


# -- Slide 34 -----------------------------------------------------------------
def _slide_34_grb221009a_matching(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "GRB 221009A：逐事件精确匹配",
                  subtitle="FIFO 复位恢复区域，按 channel + time (<0.5ms) 逐事件匹配 1B 与 1K")

    # Matching results
    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(6.0), Inches(0.4),
                "匹配结果", font_size=18, bold=True)

    col_widths = [Inches(2.5), Inches(3.0), Inches(3.0)]
    rows = [
        ["指标", "结果", "说明"],
        ["精确匹配率", "97.9% - 100%", "逐事件 channel + time 匹配"],
        ["时间偏移", "中位数 \u00b115ms", "毫秒级，非 WRAP 级错误"],
        ["1B 多出事件", "100 - 2800/区域", "1B 保留更完整事件集"],
        ["仅 1K 事件", "0 - 37/区域", "几乎没有 1K 独有事件"],
    ]
    draw_table(slide, Inches(0.5), Inches(1.9), col_widths, rows,
               row_height=Inches(0.5), font_size=13)

    # Backward SEC correction example
    add_textbox(slide, Inches(0.5), Inches(4.5), Inches(12.0), Inches(0.4),
                "Backward SEC 修正效果示例", font_size=18, bold=True, color=BLUE)

    add_box(slide, Inches(0.5), Inches(5.0), Inches(5.8), Inches(1.0),
            "Box B T+249 修正前\n402/9960 匹配 (-96%)\n事件被错放至相邻 WRAP",
            fill_color=LIGHT_RED, border_color=RED, font_size=14, text_color=DARK)

    add_arrow(slide, Inches(6.5), Inches(5.5), Inches(7.0), Inches(5.5),
              color=DARK_GRAY, width=Pt(3))

    add_box(slide, Inches(7.2), Inches(5.0), Inches(5.8), Inches(1.0),
            "Box B T+249 修正后\n10065/9960 匹配 (+1.1%)\nBackward flush 精确修正",
            fill_color=LIGHT_GREEN, border_color=GREEN, font_size=14, text_color=DARK)

    # Conclusion
    add_box(slide, Inches(0.5), Inches(6.3), Inches(12.3), Inches(0.7),
            "结论：1B 重建保留了更完整的事件集。差异来自 1K 管线在 FIFO 复位边界的激进过滤，"
            "而非 1B 重建错误。",
            fill_color=LIGHT_BLUE, border_color=BLUE, font_size=15, bold=True, text_color=DARK)


# -- Slide 35 -----------------------------------------------------------------
def _slide_35_validation_summary(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "验证总结")

    # Summary table with all GRBs
    col_widths = [Inches(2.0), Inches(1.5), Inches(1.8), Inches(2.0), Inches(2.0), Inches(2.0)]
    rows = [
        ["GRB", "饱和程度", "总差值 Δ", "Δ%", "逐事件匹配", "备注"],
        ["200415A Box A", "轻度", "0", "0.0%", "N/A", "完美匹配"],
        ["200415A Box B", "轻度", "0", "0.0%", "N/A", "完美匹配"],
        ["200415A Box C", "轻度", "0", "0.0%", "N/A", "完美匹配"],
        ["260226A", "中度", "\u22482", "+0.0%", "N/A", "Pass 3 修正"],
        ["221009A Box A", "极端", "+108", "+0.003%", "97.9-100%", "FIFO 复位恢复"],
        ["221009A Box B", "极端", "+2,936", "+0.08%", "97.9-100%", "Backward SEC"],
        ["221009A Box C", "极端", "+125,858", "+3.5%", "97.9-100%", "1K 丢数据"],
    ]
    draw_table(slide, Inches(0.3), Inches(1.3), col_widths, rows,
               row_height=Inches(0.42), font_size=12)

    # 4 key conclusions
    add_textbox(slide, Inches(0.5), Inches(5.0), Inches(12.0), Inches(0.4),
                "四项关键结论", font_size=18, bold=True, color=GREEN)

    conclusions = [
        ("\u2713  轻度/中度饱和：Δ=0，完美匹配",
         "算法在无严重饱和时与标准管线完全一致"),
        ("\u2713  极端饱和：97.9-100% 逐事件匹配",
         "即使在最恶劣条件下仍保持高精度"),
        ("\u2713  1B 保留更完整数据",
         "1K 在 FIFO 复位边界过滤导致 0.003%-3.5% 事件丢失"),
        ("\u2713  三机箱交叉参考覆盖率 92%",
         "独立机箱互补填充饱和间隙"),
    ]

    for i, (title, desc) in enumerate(conclusions):
        y = Inches(5.5) + Inches(i * 0.45)
        add_textbox(slide, Inches(0.7), y, Inches(5.0), Inches(0.4),
                    title, font_size=14, bold=True, color=GREEN)
        add_textbox(slide, Inches(5.8), y, Inches(6.5), Inches(0.4),
                    desc, font_size=13, color=DARK_GRAY)


# -- Slide 35b: 200415A reconstructed light curve ----------------------------

def _slide_35b_recon_200415a(prs):
    """200415A 补全后光变曲线 — 轻度饱和"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "GRB 200415A 补全后光变曲线（轻度饱和）",
                  "仅峰值处有极小的补全 — 整体几乎无饱和影响")

    img = os.path.join(os.path.dirname(__file__), "images",
                       "recon_step_200415a.png")
    slide.shapes.add_picture(img, Inches(0.2), Inches(1.1), Inches(9.5), Inches(6.0))

    add_rich_textbox(slide, Inches(9.9), Inches(1.3), Inches(3.2), Inches(5.5), [
        ("GRB 200415A", 18, True, BLUE),
        ("", 8),
        ("饱和程度：轻度", 14, True, GREEN),
        ("  仅在峰值 (T~0s) 有极小的饱和", 12),
        ("", 6),
        ("补全情况：", 14, True, DARK),
        ("  红色区域极小", 12),
        ("  几乎看不到补全", 12),
        ("  = 几乎不需要补全!", 12, True, GREEN),
        ("", 8),
        ("验证意义：", 14, True, PURPLE),
        ("  光变曲线平稳，无 gap", 12),
        ("  1B/1K Delta = 0", 12),
        ("  证明算法在正常区间", 12),
        ("  完全不引入伪影", 12, True, GREEN),
        ("", 8),
        ("这是基准：", 14, True, DARK),
        ("  非饱和区 → 无补全 → 无失真", 12),
        ("  证明我们「不会把好的搞坏」", 12, True, GREEN),
    ])


# -- Slide 35c: 260226A reconstructed light curve ----------------------------

def _slide_35c_recon_260226a(prs):
    """260226A 补全后光变曲线 — 中度饱和"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "GRB 260226A 补全后光变曲线（中度饱和）",
                  "峰值附近的 FIFO reset gap 被交叉参考填充")

    img = os.path.join(os.path.dirname(__file__), "images",
                       "recon_step_260226a.png")
    slide.shapes.add_picture(img, Inches(0.2), Inches(1.1), Inches(9.5), Inches(6.0))

    add_rich_textbox(slide, Inches(9.9), Inches(1.3), Inches(3.2), Inches(5.5), [
        ("GRB 260226A", 18, True, ORANGE),
        ("", 8),
        ("饱和程度：中度", 14, True, ORANGE),
        ("  峰值区 (T+15~30s) 有明显的", 12),
        ("  FIFO reset gap", 12),
        ("", 6),
        ("补全情况：", 14, True, DARK),
        ("  红色区域集中在峰值", 12),
        ("  gap 被交叉参考形状填充", 12),
        ("  补全后峰值轮廓更完整", 12, True, GREEN),
        ("", 8),
        ("关键观察：", 14, True, PURPLE),
        ("  三个 Box 的饱和时刻略有差异", 12),
        ("  → 交叉参考可以互补", 12),
        ("  红色峰值高于观测值", 12),
        ("  = 丢失的事件被估算并补回", 12),
        ("", 8),
        ("验证：1B/1K Delta ~ 0", 14, True, GREEN),
        ("  补全不影响非饱和区精度", 12),
    ])


# -- Slide 35d: 221009A reconstructed light curve ----------------------------

def _slide_35d_recon_221009a(prs):
    """221009A 补全后光变曲线 — 极端饱和"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "GRB 221009A 补全后光变曲线（极端饱和）",
                  "蓝/橙/绿 = 观测事件，红色 = 三种策略补全")

    img = os.path.join(os.path.dirname(__file__), "images",
                       "recon_step_221009a.png")
    slide.shapes.add_picture(img, Inches(0.2), Inches(1.1), Inches(9.5), Inches(6.0))

    add_rich_textbox(slide, Inches(9.9), Inches(1.3), Inches(3.2), Inches(5.5), [
        ("GRB 221009A", 18, True, RED),
        ("（史上最亮 GRB）", 13, False, DARK_GRAY),
        ("", 8),
        ("饱和程度：极端", 14, True, RED),
        ("  T+200~350s 持续严重饱和", 12),
        ("  大量 FIFO reset + 深度饱和", 12),
        ("", 6),
        ("补全情况：", 14, True, DARK),
        ("  红色区域大面积覆盖峰值", 12),
        ("  补全量远超观测量", 12),
        ("  = 丢失了大量事件", 12, True, RED),
        ("", 6),
        ("三种补全策略并用：", 14, True, DARK),
        ("  FIFO gap: 交叉参考填充", 12),
        ("  静默丢数: 泊松检测+补全", 12),
        ("  深度饱和: burst rate 修正", 12),
        ("", 8),
        ("T+400~600s 尾部", 14, True, DARK),
        ("  零星 gap 被填充", 12),
        ("  补全量较小", 12),
    ])


# -- Slide 35e: Onset detail — 1ms bins with three strategies ----------------

def _slide_35e_recon_onset(prs):
    """221009A 饱和起始段细节 — 1ms bin"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "221009A 饱和起始段 T+186~188s（1ms bin 细节）",
                  "三种补全策略在 1ms 时间分辨率下的效果")

    img = os.path.join(os.path.dirname(__file__), "images",
                       "reconstructed_221009a_onset_fixed.png")
    slide.shapes.add_picture(img, Inches(0.2), Inches(1.1), Inches(9.5), Inches(6.0))

    add_rich_textbox(slide, Inches(9.9), Inches(1.3), Inches(3.2), Inches(5.5), [
        ("1ms bin 细节解读", 16, True, ORANGE),
        ("", 8),
        ("蓝/橙/绿 = 观测事件", 13, True, DARK),
        ("  MCU 的 ~8ms 读取周期清晰可见", 12),
        ("  （每 ~8ms 有 109 事件的 burst）", 12),
        ("", 6),
        ("红色 = FIFO reset gap 补全", 13, True, RED),
        ("  用交叉参考的形状函数", 12),
        ("  较大的 gap 被填充", 12),
        ("", 6),
        ("紫色 = 静默丢数补全", 13, True, PURPLE),
        ("  包内异常间隔处填入事件", 12),
        ("  补全量较小但分布精确", 12),
        ("", 8),
        ("可以看到：", 14, True, GREEN),
        ("  饱和起始时 gap 逐渐增多", 12),
        ("  三个 Box 的饱和时刻不同", 12),
        ("  交叉参考利用了这个差异", 12),
    ])


# -- Slide 35f: Peak deep saturation — 0.1ms bins ----------------------------

def _slide_35f_recon_peak(prs):
    """221009A 峰值深度饱和区 — 0.1ms bin 包级修正"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "221009A 深度饱和峰值 T+225~225.1s（0.1ms bin）",
                  "包级 burst rate 修正：用包内 burst 计数率填充 MCU 空闲期的 gap")

    img = os.path.join(os.path.dirname(__file__), "images",
                       "reconstructed_221009a_peak_zoom.png")
    slide.shapes.add_picture(img, Inches(0.2), Inches(1.1), Inches(9.5), Inches(6.0))

    add_rich_textbox(slide, Inches(9.9), Inches(1.3), Inches(3.2), Inches(5.5), [
        ("深度饱和区细节", 16, True, PURPLE),
        ("", 8),
        ("紫色填充 = 深度饱和包级修正", 13, True, PURPLE),
        ("  MCU 每 ~8ms 读一次 FIFO", 12),
        ("  burst ~0.5ms 内读出 109 事件", 12),
        ("  其余 ~7.5ms 空闲期无数据", 12),
        ("  → 用 burst rate 填充这些 gap", 12),
        ("", 6),
        ("红色 = FIFO reset gap 补全", 13, True, RED),
        ("  偶尔出现的 FIFO 复位空洞", 12),
        ("", 6),
        ("绿色/橙色 = 交叉参考", 13, True, GREEN),
        ("  来自其他 Box 的参考数据", 12),
        ("", 8),
        ("这是最困难的区域：", 14, True, RED),
        ("  所有 Box 都在深度饱和", 12),
        ("  邻居率 < 15000 evt/s", 12),
        ("  只能用 burst rate 近似", 12),
        ("  补全后光变更平滑连续", 12, True, GREEN),
    ])


# -- Slide 36 -----------------------------------------------------------------
def _slide_36_limitations(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "已知局限性")

    # 4 limitation boxes side by side
    limitations = [
        ("4-bit CRC 碰撞\n(\u224806.3%)",
         "随机字节有 1/16 概率通过 CRC\n"
         "\u2192 可能产生 ghost events\n\n"
         "保护：MIN_EVENTS=50\n"
         "221009A: 仅 2/320,000 严重\n"
         "损坏包 (0.001%)",
         LIGHT_RED, RED),
        ("Double Ambiguity\n(双重歧义)",
         "Stale + fresh anchor 回绕\n"
         "边界重合 \u2192 无法解析\n\n"
         "Box B T+277/278 约 3000\n"
         "events 受影响\n"
         "影响：0.075%",
         LIGHT_ORANGE, ORANGE),
        ("三机箱同时饱和\n(\u22488% 间隙)",
         "三个机箱同时进入深度\n"
         "饱和 \u2192 无交叉参考\n\n"
         "采用均匀分布填充\n"
         "主要在 GRB 峰值区\n"
         "影响光变曲线精度",
         LIGHT_PURPLE, PURPLE),
        ("中等 gap 阈值\n(GAP_FACTOR=100)",
         "10-50\u00d7 正常率的 gap\n"
         "不被检测\n\n"
         "光变曲线中表现为窄间隙\n"
         "降低阈值会增加假阳性\n"
         "需要平衡灵敏度和误报",
         LIGHT_BLUE, BLUE),
    ]

    for i, (title, desc, fill, border) in enumerate(limitations):
        x = Inches(0.3) + Inches(i * 3.25)
        # Title box
        add_box(slide, x, Inches(1.3), Inches(3.0), Inches(0.8),
                title, fill_color=border, border_color=border,
                font_size=13, bold=True, text_color=WHITE)
        # Description box
        add_box(slide, x, Inches(2.1), Inches(3.0), Inches(3.0),
                desc, fill_color=fill, border_color=border,
                font_size=11, text_color=DARK)

    # Applicability summary at bottom
    add_textbox(slide, Inches(0.5), Inches(5.5), Inches(12.0), Inches(0.4),
                "适用性总结", font_size=18, bold=True, color=DARK)

    levels = [
        ("非饱和", "\u2713 完美重建", GREEN, LIGHT_GREEN),
        ("轻度饱和", "\u2713 Δ=0", GREEN, LIGHT_GREEN),
        ("中度饱和", "\u2713 Δ\u22480, Pass 3 修正", ORANGE, LIGHT_ORANGE),
        ("极端饱和", "\u2248 97.9-100% 匹配", RED, LIGHT_RED),
    ]

    for i, (level, result, border, fill) in enumerate(levels):
        x = Inches(0.5) + Inches(i * 3.1)
        add_box(slide, x, Inches(6.0), Inches(2.9), Inches(0.5),
                level, fill_color=border, border_color=border,
                font_size=14, bold=True, text_color=WHITE)
        add_box(slide, x, Inches(6.5), Inches(2.9), Inches(0.5),
                result, fill_color=fill, border_color=border,
                font_size=13, text_color=DARK)


# -- Slide 37 -----------------------------------------------------------------
def _slide_37_conclusions(prs):
    layout = prs.slide_layouts[6]
    slide = prs.slides.add_slide(layout)
    bg = slide.background.fill
    bg.solid()
    bg.fore_color.rgb = TITLE_BG

    add_textbox(slide, Inches(0.5), Inches(0.4), Inches(12.3), Inches(0.8),
                "结论", font_size=36, bold=True, color=WHITE, alignment=PP_ALIGN.CENTER)

    conclusions = [
        ("1", "四遍时间重建算法",
         "克服 FIFO 拥塞 + 复位时间戳畸变。非饱和 Δ=0，极端饱和 97.9-100% 匹配"),
        ("2", "三种饱和模式",
         "识别并开发了 Silent Drop + FIFO Reset + 深度饱和 三种模式的检测与重建方法"),
        ("3", "三机箱交叉参考",
         "利用独立机箱互补性。覆盖率 92%，有效填充单机箱饱和间隙"),
        ("4", "1B 保留更完整数据",
         "1K 管线在 FIFO 复位边界激进过滤导致 0.003%-3.5% 事件丢失"),
        ("5", "Rust CLI 工具",
         "可处理任意 HXMT HE 1B 数据。子命令：detect / reconstruct / compare / dump"),
    ]

    for i, (num, title, desc) in enumerate(conclusions):
        y = Inches(1.5) + Inches(i * 1.1)
        # Number circle
        add_box(slide, Inches(0.8), y, Inches(0.6), Inches(0.6), num,
                fill_color=BLUE, border_color=BLUE, font_size=22, bold=True, text_color=WHITE)
        # Title
        add_textbox(slide, Inches(1.6), y, Inches(4.0), Inches(0.5),
                    title, font_size=22, bold=True, color=WHITE)
        # Description
        add_textbox(slide, Inches(1.6), y + Inches(0.45), Inches(10.5), Inches(0.5),
                    desc, font_size=15, color=WARM_DESC)


# -- Slide 38 -----------------------------------------------------------------
def _slide_38_thank_you(prs):
    layout = prs.slide_layouts[6]
    slide = prs.slides.add_slide(layout)
    bg = slide.background.fill
    bg.solid()
    bg.fore_color.rgb = TITLE_BG

    add_textbox(slide, Inches(1.0), Inches(2.2), Inches(11.33), Inches(1.5),
                "\u8c22\u8c22\uff01",
                font_size=60, bold=True, color=WHITE, alignment=PP_ALIGN.CENTER)

    add_textbox(slide, Inches(1.0), Inches(3.8), Inches(11.33), Inches(0.8),
                "Questions?",
                font_size=32, color=WARM_LIGHT, alignment=PP_ALIGN.CENTER)

    # References
    add_textbox(slide, Inches(1.0), Inches(5.2), Inches(11.33), Inches(0.5),
                "Paper: paper/main.tex",
                font_size=16, color=WARM_DESC, alignment=PP_ALIGN.CENTER)

    add_textbox(slide, Inches(1.0), Inches(5.7), Inches(11.33), Inches(0.5),
                "Tool: blink_cli (Rust) \u2014 detect / reconstruct / compare / dump",
                font_size=16, color=WARM_DESC, alignment=PP_ALIGN.CENTER)
