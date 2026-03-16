"""Slides 22-30: 多 pass 修正、检测算法与光变曲线重建。"""

from pptx_helpers import *


def add_slides(prs):
    _slide_22a_wrap_reversal_problem(prs)
    _slide_22b_wrap_reversal_fix(prs)
    _slide_23a_dip_problem(prs)
    _slide_23b_dip_fix(prs)
    _slide_24_segmented_sort(prs)
    _slide_24b_recon_results(prs)
    _slide_24c_scatter_boxA(prs)
    _slide_24d_scatter_boxC(prs)
    _slide_24e_recon_overview(prs)
    _slide_24f_zoom_boxB(prs)
    _slide_25_fifo_reset_detection(prs)
    _slide_25b_fifo_reset_examples(prs)
    _slide_26_silent_drop(prs)
    _slide_26b_silent_drop_examples(prs)
    _slide_27_r_true(prs)
    _slide_28_fifo_gap_filling(prs)
    _slide_29_coverage(prs)
    _slide_30_silent_deep_filling(prs)


# ── Slide 22a: Pass 2 — problem ──────────────────────────────────────────────

def _slide_22a_wrap_reversal_problem(prs):
    """Pass 2 问题：时间倒退是什么样子、为什么出现"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Pass 2：时间倒退修正 — 问题",
                  "Pass 1 之后仍残留的 ±1 WRAP 批次偏移")

    # Left: 具体例子
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(3.2), [
        ("Pass 1 之后的时间序列可能长这样：", 16, True, RED),
        ("", 6),
        ("  包 500: MET = 249.0s   正确（拥塞包，wrap tracking）", 12, False, GREEN),
        ("  包 501: MET = 249.1s   正确", 12, False, GREEN),
        ("  -------- FIFO 复位 gap --------", 12, True, RED),
        ("  包 502: MET = 250.1s   高了 1 WRAP！应该是 249.05s", 12, True, RED),
        ("  包 503: MET = 250.2s   也高了", 12, True, RED),
        ("  包 504: MET = 250.3s   也高了", 12, True, RED),
        ("  包 505: MET = 249.2s   又正确了（新鲜 SEC 后的包）", 12, False, GREEN),
        ("  包 506: MET = 249.3s   正确", 12, False, GREEN),
        ("", 6),
        ("时间序列：249.0 → 249.1 → [250.1, 250.2, 250.3] → 249.2", 13, True, DARK),
        ("                          高了 ~1.05s        倒退 ~1.05s!", 13, True, RED),
    ])

    # Right: 为什么出现
    add_box(slide, Inches(6.8), Inches(1.15), Inches(6.2), Inches(3.2),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(3.0), [
        ("为什么 Pass 1 没完全修好？", 16, True, ORANGE),
        ("", 6),
        ("后向 SEC 修正覆盖了大部分 FIFO 复位后的事件。", 13),
        ("但有些边缘情况它处理不了：", 13),
        ("", 6),
        ("  1. 后向修正的阶段 2（前后对照）在双重模糊时", 12),
        ("     选错了候选 → 部分包仍差 ±1 wrap", 12),
        ("", 4),
        ("  2. 连续 FIFO 复位之间的 stale batch 边界包", 12),
        ("     可能没被完全修正", 12),
        ("", 4),
        ("  3. 后向修正只处理被检测为 FIFO 复位的区域，", 12),
        ("     有些轻微的 FIFO 复位（gap 不够大）没被", 12),
        ("     检测到 → 对应事件未进入 pending 列表", 12),
        ("", 6),
        ("这些残留的 ±1 WRAP 偏移需要 Pass 2 来清理。", 13, True, RED),
    ])

    # Bottom: diagram
    ox, oy = Inches(0.4), Inches(6.8)
    pw = Inches(12.4)

    add_textbox(slide, Inches(0.4), Inches(4.5), Inches(12), Inches(0.3),
                "时间序列示意图（MET vs 包序号）：", font_size=14, bold=True, color=DARK)

    # Draw the reversal pattern with bars
    bar_y = Inches(5.0)
    bar_h = Inches(0.4)

    # Normal segment 1
    draw_bar(slide, Inches(0.5), bar_y, Inches(2.5), bar_h, GREEN,
             label="正确层级 (n=5)", font_size=10)

    # Gap
    draw_bar(slide, Inches(3.0), bar_y, Inches(0.5), bar_h, LIGHT_GRAY,
             label="gap", font_size=9, text_color=GRAY, border_color=GRAY)
    add_textbox(slide, Inches(2.8), bar_y + bar_h, Inches(0.9), Inches(0.25),
                "FIFO复位gap", font_size=8, color=RED, alignment=PP_ALIGN.CENTER)

    # High batch (wrong)
    high_y = Inches(4.3)
    draw_bar(slide, Inches(3.5), high_y, Inches(3.0), bar_h, RED,
             label="偏高批次 (n=6, 应该是 5!)", font_size=10)
    add_arrow(slide, Inches(5.0), high_y + bar_h, Inches(5.0), bar_y,
              color=BLUE, width=Pt(2))
    add_textbox(slide, Inches(5.1), Inches(4.55), Inches(1.5), Inches(0.3),
                "应下移 -1 WRAP", font_size=10, bold=True, color=BLUE)

    # Normal segment 2
    draw_bar(slide, Inches(6.5), bar_y, Inches(3.0), bar_h, GREEN,
             label="正确层级 (n=5)", font_size=10)

    # Reversal arrow
    add_arrow(slide, Inches(6.5), high_y + Inches(0.15), Inches(6.5), bar_y + Inches(0.15),
              color=RED, width=Pt(2))
    add_textbox(slide, Inches(6.6), Inches(4.45), Inches(1.0), Inches(0.3),
                "倒退!", font_size=11, bold=True, color=RED)

    # Continuing normal
    draw_bar(slide, Inches(9.5), bar_y, Inches(3.0), bar_h, GREEN,
             label="正确层级 (继续)", font_size=10)

    # Bottom note
    add_textbox(slide, Inches(0.5), Inches(5.6), Inches(12), Inches(0.3),
                "Pass 2 的任务：找到这些偏高的批次，把它们下移 1 个 WRAP_PERIOD (-1.05s)",
                font_size=13, bold=True, color=BLUE)


# ── Slide 22b: Pass 2 — gap criterion + fix ──────────────────────────────────

def _slide_22b_wrap_reversal_fix(prs):
    """Pass 2 算法：gap 判据区分真假倒退"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Pass 2：时间倒退修正 — 算法",
                  "用 gap 判据区分「真错误」和「正常乱序」")

    # Left: algorithm steps
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(2.5), [
        ("算法步骤", 16, True, GREEN),
        ("", 6),
        ("1. 构建干净包序列（span < 0.3s，排除损坏包）", 13),
        ("2. 计算相邻干净包的时间差", 13),
        ("3. 发现 delta ~ -1.05s → 可能的倒退点", 13),
        ("4. 从倒退点往前追溯，找到整批偏高的包", 13),
        ("5. 检查这批包前面有没有大 gap（> 0.3s）", 13, True, DARK),
        ("6. 已标记 backward_flushed 的包 → 跳过（已精确修正）", 13),
    ])

    # Right top: situation A
    add_box(slide, Inches(6.8), Inches(1.15), Inches(6.2), Inches(2.5),
            "", fill_color=LIGHT_RED, border_color=RED)
    add_rich_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(2.3), [
        ("情况 A：批次前有大 gap → 真错误 → 修正!", 14, True, RED),
        ("", 6),
        ("  包 500: MET=249.0", 12),
        ("  包 501: MET=249.1", 12),
        ("  ---- gap 0.8s ----    FIFO 复位造成的数据空洞", 12, True, RED),
        ("  包 502: MET=250.1     这批高了 1 WRAP", 12),
        ("  包 503: MET=250.2", 12),
        ("  包 505: MET=249.2     倒退!", 12),
        ("", 6),
        ("  gap > 0.3s → 确认是 FIFO 复位后的真错位", 12, True, DARK),
        ("  → 整批下移 -1.05s ✓", 12, True, GREEN),
    ])

    # Left bottom: situation B
    add_box(slide, Inches(0.4), Inches(3.85), Inches(6.2), Inches(2.5),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(0.6), Inches(3.9), Inches(5.8), Inches(2.3), [
        ("情况 B：批次前无 gap → 正常乱序 → 不修!", 14, True, GREEN),
        ("", 6),
        ("  包 500: MET=249.0", 12),
        ("  包 501: MET=249.05    平滑衔接，没有 gap", 12, True, GREEN),
        ("  包 502: MET=250.1     看起来高了", 12),
        ("  包 503: MET=250.15", 12),
        ("  包 504: MET=249.1     倒退?", 12),
        ("", 6),
        ("  无 gap → 这不是 n_base 错误，是 FIFO 复位造成的", 12, True, DARK),
        ("  文件中的自然包重排 → 跳过，不动 ✓", 12, True, GREEN),
    ])

    # Right bottom: why 0.3s + backward_flushed
    add_box(slide, Inches(6.8), Inches(3.85), Inches(6.2), Inches(1.2),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(3.9), Inches(5.8), Inches(1.0), [
        ("为什么 gap 阈值是 0.3s？", 14, True, ORANGE),
        ("  正常包间隔 ~7ms (109 evt / 15000 evt/s)", 12),
        ("  FIFO 复位 gap 通常几十 ms 到几秒", 12),
        ("  0.3s 远大于正常间隔，足以确认是 FIFO 复位 → 好的分界线", 12),
    ])

    add_box(slide, Inches(6.8), Inches(5.2), Inches(6.2), Inches(1.2),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(7.0), Inches(5.25), Inches(5.8), Inches(1.0), [
        ("为什么跳过 backward_flushed 包？", 14, True, PURPLE),
        ("  后向 SEC 修正用新鲜 SEC 精确计算过这些包的 n_wraps", 12),
        ("  精度比 Pass 2 的批量下移更高", 12),
        ("  如果 Pass 2 动了它们 → 反而破坏正确结果!", 12, True, RED),
    ])

    # Bottom summary
    add_box(slide, Inches(0.4), Inches(6.55), Inches(12.6), Inches(0.8),
            "", fill_color=RGBColor(0xF9, 0xF5, 0xF0), border_color=DARK_GRAY)
    add_rich_textbox(slide, Inches(0.6), Inches(6.6), Inches(12.2), Inches(0.6), [
        ("Pass 2 总结：扫描反向跳跃 ~ -1.05s → 检查 gap 判据区分真假 → 真错误整批下移 → 跳过已精确修正的包",
         13, True, DARK),
    ])


# ── Slide 23a: Pass 3 — dip problem with concrete example ─────────────────────

def _slide_23a_dip_problem(prs):
    """Pass 3 问题：凹坑为什么出现，具体数字推演"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Pass 3：凹坑修正 — 问题",
                  "Pass 2 的反向操作：Pass 2 修偏高，Pass 3 修偏低")

    # Top: Pass 2 vs Pass 3 对比
    add_box(slide, Inches(0.4), Inches(1.15), Inches(12.4), Inches(1.0),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(0.6), Inches(1.2), Inches(12.0), Inches(0.8), [
        ("Pass 2 与 Pass 3 是互补的对称操作", 16, True, PURPLE),
        ("  Pass 2：找偏高批次 (+1 WRAP) → 检测反向跳跃 -1.05s → 整批下移    |    "
         "Pass 3：找偏低批次 (-1 WRAP) → 检测正向跳跃 +1.05s → 整批上移", 12),
    ])

    # Left: 凹坑长什么样
    add_rich_textbox(slide, Inches(0.4), Inches(2.35), Inches(6.2), Inches(2.0), [
        ("凹坑长什么样", 16, True, RED),
        ("", 6),
        ("  包 200: MET=5.00s  正确 (n=5)", 12, False, GREEN),
        ("  包 201: MET=5.10s  正确", 12, False, GREEN),
        ("  包 202: MET=5.20s  正确", 12, False, GREEN),
        ("  包 203: MET=4.05s  突然低了 ~1.05s! (n=4, 应该是 5)", 12, True, RED),
        ("  包 204: MET=4.15s  也低了", 12, True, RED),
        ("  包 205: MET=4.25s  也低了", 12, True, RED),
        ("  包 206: MET=5.30s  又正确了 (n=5)", 12, False, GREEN),
    ])

    # Right: 为什么出现 — 具体数字
    add_box(slide, Inches(6.8), Inches(2.35), Inches(6.2), Inches(2.0),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(2.4), Inches(5.8), Inches(1.8), [
        ("根因：round() 取整在 median 经过 anchor 时翻转", 14, True, ORANGE),
        ("", 6),
        ("n_est = round((utc - anchor_MET", 12, True, DARK),
        ("       - (p_median - p_anchor)*2us) / WRAP_PERIOD)", 12, True, DARK),
        ("", 4),
        ("当 median 经过 anchor_ptime 附近时，", 12),
        ("(p_median - p_anchor) 突然跳变 ~524288", 12),
        ("→ 分子变化 ~1.05s → round() 参数跨过 x.5 → 翻转!", 12, True, RED),
    ])

    # Bottom: 具体计算例子
    add_box(slide, Inches(0.4), Inches(4.55), Inches(6.2), Inches(2.8),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, Inches(0.6), Inches(4.6), Inches(5.8), Inches(2.6), [
        ("具体计算例子", 14, True, BLUE),
        ("anchor_ptime = 100000, anchor_MET = 500.0s", 12, False, DARK_GRAY),
        ("", 6),
        ("包 B: median=523000 (快绕回了)", 12),
        ("  (523000-100000)*2us = 0.846s", 11, False, DARK_GRAY),
        ("  n = round((505.85 - 500.0 - 0.846) / 1.0486)", 11),
        ("    = round(5.004 / 1.0486) = round(4.77) = 5 ✓", 11, True, GREEN),
        ("", 4),
        ("包 C: median=90000 (刚绕过 anchor!)", 12, True, RED),
        ("  (90000-100000)*2us = -0.02s", 11, False, DARK_GRAY),
        ("  n = round((505.9 - 500.0 - (-0.02)) / 1.0486)", 11),
        ("    = round(5.92 / 1.0486) = round(5.65) = 6", 11),
        ("  但如果真实值应该是 5... → 多了 1! 不是凹坑而是凸起", 11, True, RED),
        ("  或者包 B 应该是 5 但实际 round(4.49)=4 → 少了 1! 凹坑", 11, True, RED),
    ])

    add_box(slide, Inches(6.8), Inches(4.55), Inches(6.2), Inches(2.8),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.0), Inches(4.6), Inches(5.8), Inches(2.6), [
        ("什么时候触发？", 14, True, GREEN),
        ("", 6),
        ("两个条件同时满足：", 13),
        ("", 4),
        ("1. median ptime 经过 anchor_ptime 附近", 13),
        ("   ptime 每绕一圈 (~1.05s) 就经过一次", 12, False, DARK_GRAY),
        ("   → 每 ~1.05s 有一次触发机会", 12, False, DARK_GRAY),
        ("", 4),
        ("2. 恰好 elapsed 让 round() 参数在 x.5 附近", 13),
        ("   跳变把它推过了边界 → 翻转", 12, False, DARK_GRAY),
        ("   → 概率取决于具体时间，不是每次都触发", 12, False, DARK_GRAY),
        ("", 6),
        ("260226A 验证：Pass 3 修正了", 13, True, DARK),
        ("  Box A: 7 包, Box B: 14 包, Box C: 13 包", 12, True, DARK),
        ("  (说明中度饱和时也有边界翻转)", 12, False, DARK_GRAY),
    ])


# ── Slide 23b: Pass 3 — fix algorithm ────────────────────────────────────────

def _slide_23b_dip_fix(prs):
    """Pass 3 算法：两步修正 + 为什么不需要 gap 判据"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Pass 3：凹坑修正 — 算法",
                  "Step 1 修整批凹坑，Step 2 修混合包")

    # Left: Step 1
    add_box(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(3.0),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, Inches(0.6), Inches(1.2), Inches(5.8), Inches(2.8), [
        ("Step 1：检测凹坑批次 → 整批上移", 16, True, BLUE),
        ("", 6),
        ("扫描时间序列，找正向跳跃 ~ +1.05s：", 13),
        ("", 4),
        ("  ... 4.05, 4.15, 4.25, [5.30] ...", 12, True, DARK),
        ("                         正向跳跃 +1.05s = 凹坑结束点", 12, False, DARK_GRAY),
        ("", 4),
        ("从跳跃点往前回溯，找到 [4.05, 4.15, 4.25] 这批包。", 12),
        ("确认前后邻居都在 5.x 层级 → 这批是凹坑。", 12),
        ("→ 整批上移 +1.05s (WRAP_PERIOD)", 12, True, GREEN),
        ("", 6),
        ("为什么不需要 gap 判据？（和 Pass 2 不同）", 13, True, ORANGE),
        ("  Pass 2 的偏高可能是文件重排（假阳性）→ 需要 gap 区分", 12),
        ("  Pass 3 的凹坑不可能是文件重排产生的 → 没有假阳性", 12),
    ])

    # Right: Step 2
    add_box(slide, Inches(6.8), Inches(1.15), Inches(6.2), Inches(3.0),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(7.0), Inches(1.2), Inches(5.8), Inches(2.8), [
        ("Step 2：修复混合包", 16, True, PURPLE),
        ("", 6),
        ("有时候一个包内同时有两个层级的事件：", 13),
        ("", 4),
        ("  包 203 的 109 个事件：", 12, True, DARK),
        ("    事件 1-80:  MET = 5.15~5.18s  高层级（正确）", 12, False, GREEN),
        ("    事件 81-109: MET = 4.10~4.13s 低层级（凹坑）", 12, False, RED),
        ("", 4),
        ("  包跨时 span = 5.18 - 4.10 = 1.08s ~ WRAP_PERIOD", 12, True, DARK),
        ("  ← 这是混合包的标志！", 12, True, PURPLE),
        ("", 6),
        ("修复：看邻居包是哪个层级", 12),
        ("  多数簇 = 跟邻居一致的那组 → 保持不动", 12, False, GREEN),
        ("  少数簇 = 另一组 → 对齐 +WRAP_PERIOD", 12, False, BLUE),
    ])

    # Bottom: 时间序列示意图
    add_textbox(slide, Inches(0.4), Inches(4.35), Inches(12), Inches(0.3),
                "时间序列示意图 — Step 1 修正过程：", font_size=14, bold=True, color=DARK)

    bar_y = Inches(4.8)
    bar_h = Inches(0.4)

    # Before fix
    add_textbox(slide, Inches(0.4), bar_y - Inches(0.05), Inches(0.8), Inches(0.4),
                "修正前：", font_size=11, bold=True, color=RED)
    draw_bar(slide, Inches(1.3), bar_y, Inches(2.5), bar_h, GREEN,
             label="正确层级 (n=5)", font_size=10)
    dip_y = bar_y + Inches(0.6)
    draw_bar(slide, Inches(3.8), dip_y, Inches(2.0), bar_h, RED,
             label="凹坑 (n=4!)", font_size=10)
    draw_bar(slide, Inches(5.8), bar_y, Inches(2.5), bar_h, GREEN,
             label="正确层级 (n=5)", font_size=10)

    add_arrow(slide, Inches(3.8), bar_y + bar_h, Inches(3.8), dip_y,
              color=RED, width=Pt(1.5))
    add_textbox(slide, Inches(3.3), bar_y + Inches(0.15), Inches(0.5), Inches(0.2),
                "-1.05s", font_size=8, color=RED)

    add_arrow(slide, Inches(5.8), dip_y, Inches(5.8), bar_y,
              color=RED, width=Pt(1.5))
    add_textbox(slide, Inches(5.9), bar_y + Inches(0.15), Inches(0.5), Inches(0.2),
                "+1.05s", font_size=8, color=RED)

    # After fix
    bar_y2 = Inches(5.9)
    add_textbox(slide, Inches(0.4), bar_y2 - Inches(0.05), Inches(0.8), Inches(0.4),
                "修正后：", font_size=11, bold=True, color=GREEN)
    draw_bar(slide, Inches(1.3), bar_y2, Inches(7.0), bar_h, GREEN,
             label="全部在正确层级 (n=5) ✓", font_size=10)

    add_arrow(slide, Inches(4.8), dip_y + bar_h, Inches(4.8), bar_y2,
              color=BLUE, width=Pt(2))
    add_textbox(slide, Inches(4.9), Inches(5.5), Inches(1.0), Inches(0.25),
                "+WRAP", font_size=10, bold=True, color=BLUE)

    # Summary box
    add_box(slide, Inches(8.5), Inches(4.8), Inches(4.3), Inches(2.3),
            "", fill_color=RGBColor(0xF9, 0xF5, 0xF0), border_color=DARK_GRAY)
    add_rich_textbox(slide, Inches(8.7), Inches(4.85), Inches(3.9), Inches(2.1), [
        ("Pass 2 vs Pass 3 对比", 14, True, DARK),
        ("", 6),
        ("         Pass 2    Pass 3", 11, True, DARK_GRAY),
        ("偏移方向  偏高+1    偏低-1", 11),
        ("检测方式  反向跳跃  正向跳跃", 11),
        ("修正方向  下移      上移", 11),
        ("gap判据   需要      不需要", 11),
        ("混合包    不处理    Step 2处理", 11),
        ("", 6),
        ("互补操作：一起保证所有", 11, True, DARK),
        ("±1 WRAP 批次偏移被清理", 11, True, DARK),
    ])


# ── Slide 24: Pass 4 — Segmented sort ────────────────────────────────────────

def _slide_24_segmented_sort(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Pass 4：分段排序",
                  "四遍管线的最后一步 — 解决后向修正交界处的局部时间重叠")

    # Top: 澄清误区
    add_box(slide, Inches(0.4), Inches(1.15), Inches(12.4), Inches(1.2),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, Inches(0.6), Inches(1.2), Inches(12.0), Inches(1.0), [
        ("MCU 写入顺序本来就是时间顺序 — 不需要对整个文件排序！", 16, True, BLUE),
        ("  FIFO 严格先进先出 → MCU 读出顺序 = 事件进入顺序 = ptime 递增顺序", 13),
        ("  Pass 4 排的不是「文件乱序」，而是后向修正改变 MET 后在交界处产生的局部时间重叠", 13, True, DARK),
    ])

    # Left: 什么场景需要排序
    add_rich_textbox(slide, Inches(0.4), Inches(2.55), Inches(6.2), Inches(3.0), [
        ("什么时候会出现时间重叠？", 16, True, RED),
        ("", 6),
        ("后向 SEC 修正重新计算了一批事件的 MET。", 13),
        ("修正后的值可能和相邻的 wrap tracking 区域有重叠：", 13),
        ("", 6),
        ("  Wrap tracking 区域（未修正）：", 12, True, DARK),
        ("    包 995: MET = 277.50", 12),
        ("    包 996: MET = 277.60", 12),
        ("    包 997: MET = 277.80", 12),
        ("", 4),
        ("  后向修正区域（MET 被重新计算了）：", 12, True, ORANGE),
        ("    包 998: MET = 277.55  ← 可能比包 997 早！", 12, True, RED),
        ("    包 999: MET = 277.65", 12),
        ("    包 1000: MET = 277.75", 12),
        ("", 4),
        ("  文件顺序：... 277.80, 277.55, 277.65 ...", 12, True, RED),
        ("                       ↑ 局部时间倒退", 12, False, RED),
    ])

    # Right: 为什么不能全局排序 + 分段方案
    add_box(slide, Inches(6.8), Inches(2.55), Inches(6.2), Inches(1.5),
            "", fill_color=LIGHT_RED, border_color=RED)
    add_rich_textbox(slide, Inches(7.0), Inches(2.6), Inches(5.8), Inches(1.3), [
        ("为什么不能直接全局排序？", 14, True, RED),
        ("", 6),
        ("后向修正用新鲜 SEC 为每个 pending 事件精确计算了 MET", 12),
        ("这些事件的内部顺序已经是最优的", 12),
        ("全局排序会把它们和相邻 wrap tracking 事件混合排列", 12),
        ("→ 破坏了后向修正的精确分组 → 反而变差！", 12, True, RED),
    ])

    add_box(slide, Inches(6.8), Inches(4.25), Inches(6.2), Inches(1.3),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(7.0), Inches(4.3), Inches(5.8), Inches(1.1), [
        ("解决方案：分段排序", 14, True, GREEN),
        ("", 6),
        ("1. 找到所有 backward_flushed 区域（Pass 1 补充时标记的）", 12),
        ("2. 这些区域冻结不动", 12, True, DARK),
        ("3. 只对非 backward_flushed 的连续段排序", 12, True, DARK),
    ])

    # Bottom: 示意图
    add_textbox(slide, Inches(0.4), Inches(5.7), Inches(12), Inches(0.3),
                "分段排序示意：", font_size=14, bold=True, color=DARK)

    bar_y = Inches(6.1)
    bar_h = Inches(0.45)

    draw_bar(slide, Inches(0.5), bar_y, Inches(2.5), bar_h, GREEN,
             "排序", font_size=10)
    draw_bar(slide, Inches(3.0), bar_y, Inches(2.5), bar_h, PURPLE,
             "后向修正区域（冻结）", font_size=10)
    draw_bar(slide, Inches(5.5), bar_y, Inches(2.5), bar_h, GREEN,
             "排序", font_size=10)
    draw_bar(slide, Inches(8.0), bar_y, Inches(1.5), bar_h, PURPLE,
             "冻结", font_size=10)
    draw_bar(slide, Inches(9.5), bar_y, Inches(3.0), bar_h, GREEN,
             "排序", font_size=10)

    add_textbox(slide, Inches(0.5), bar_y + bar_h + Inches(0.05), Inches(12), Inches(0.3),
                "Pass 4 是最简单的一步：只在交界处做局部排序，不修改任何 MET 值，只重新排列顺序。",
                font_size=12, bold=True, color=DARK_GRAY)


# ── Slide 24b: Reconstruction results — algorithm works ─────────────────────

def _slide_24b_recon_results(prs):
    """展示时间重建效果：1B 和 1K 的对比，证明算法正确"""
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "时间重建效果验证",
                  "四遍管线的重建结果 vs 1K 标准管线")

    # Top: key message
    add_box(slide, Inches(0.4), Inches(1.15), Inches(12.4), Inches(0.8),
            "", fill_color=LIGHT_GREEN, border_color=GREEN)
    add_rich_textbox(slide, Inches(0.6), Inches(1.2), Inches(12.0), Inches(0.6), [
        ("核心问题：我们从 1B 原始数据重建的事件时间，和 1K 标准管线相比如何？", 18, True, GREEN),
    ])

    # Left: non-saturated validation
    add_box(slide, Inches(0.4), Inches(2.15), Inches(6.2), Inches(2.5),
            "", fill_color=LIGHT_BLUE, border_color=BLUE)
    add_rich_textbox(slide, Inches(0.6), Inches(2.2), Inches(5.8), Inches(2.3), [
        ("非饱和/轻度饱和区：完美一致", 16, True, BLUE),
        ("", 6),
        ("GRB 200415A（轻度饱和）：", 14, True, DARK),
        ("  三个 Box 全部 Delta = 0，所有 1s bin 完美匹配", 13, True, GREEN),
        ("", 4),
        ("GRB 260226A（中度饱和）：", 14, True, DARK),
        ("  事件数差异仅 2 个（+0.0%），所有 bin 误差 0.0%", 13, True, GREEN),
        ("", 6),
        ("证明：四遍管线在正常区间完全正确", 13, True, GREEN),
        ("  → 不比 1K 差，而且不做激进过滤", 13),
    ])

    # Right: saturated validation
    add_box(slide, Inches(6.8), Inches(2.15), Inches(6.2), Inches(2.5),
            "", fill_color=LIGHT_ORANGE, border_color=ORANGE)
    add_rich_textbox(slide, Inches(7.0), Inches(2.2), Inches(5.8), Inches(2.3), [
        ("极端饱和区：97.9-100% 精确匹配", 16, True, ORANGE),
        ("", 6),
        ("GRB 221009A（史上最亮 GRB）：", 14, True, DARK),
        ("  在 FIFO 复位恢复区域逐事件匹配（<0.5ms 容差）", 13),
        ("  精确匹配率：97.9% - 100%", 14, True, GREEN),
        ("  时间偏移中位数：仅 +-15ms（毫秒级，非 wrap 级）", 13),
        ("", 6),
        ("证明：即使在极端饱和区，绝大多数事件", 13, True, GREEN),
        ("  的重建时间与 1K 一致", 13, True, GREEN),
    ])

    # Bottom: 后向修正效果
    add_box(slide, Inches(0.4), Inches(4.85), Inches(6.2), Inches(2.3),
            "", fill_color=LIGHT_PURPLE, border_color=PURPLE)
    add_rich_textbox(slide, Inches(0.6), Inches(4.9), Inches(5.8), Inches(2.1), [
        ("后向 SEC 修正的效果（Box B T+249s）", 14, True, PURPLE),
        ("", 6),
        ("修正前（用过期锚点）：", 13, True, RED),
        ("  找到 402 / 9960 个事件（-96%）", 13),
        ("  9558 个事件被错放到 T+248s", 13),
        ("", 4),
        ("修正后（用新鲜 SEC 反向重算）：", 13, True, GREEN),
        ("  找到 10065 / 9960 个事件（+1.1%）✓", 13),
    ])

    # Bottom right: summary table
    col_w = [Inches(2.0), Inches(1.5), Inches(1.5), Inches(1.3)]
    rows = [
        ["GRB", "饱和程度", "1B vs 1K Delta", "结论"],
        ["200415A", "轻度", "0", "完美一致"],
        ["260226A", "中度", "~0", "完美一致"],
        ["221009A Box A", "极端", "+108 (+0.003%)", "1B 更完整"],
        ["221009A Box B", "极端", "+2936 (+0.08%)", "1B 更完整"],
        ["221009A Box C", "极端", "+125858 (+3.5%)", "1K 丢数据"],
    ]
    draw_table(slide, Inches(6.8), Inches(4.85), col_w, rows,
               row_height=Inches(0.35), header_bg=BLUE, font_size=11)


# ── Slide 24c: Box A scatter — 1B/1K near-perfect alignment ──────────────────

def _slide_24c_scatter_boxA(prs):
    """Box A scatter plot — 1B/1K 几乎完美对齐"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Box A：1B/1K 逐事件 scatter 对比",
                  "GRB 221009A Box A — Delta = +108 (+0.003%)")

    img_path = os.path.join(os.path.dirname(__file__), "images", "scatter_221009A_boxA.png")
    slide.shapes.add_picture(img_path, Inches(0.3), Inches(1.1), Inches(9.0), Inches(6.2))

    add_rich_textbox(slide, Inches(9.5), Inches(1.3), Inches(3.5), Inches(5.5), [
        ("Box A 解读", 18, True, BLUE),
        ("", 8),
        ("上方：1B/1K 光变曲线叠加", 13, True, DARK),
        ("  蓝色=1B, 灰色=1K", 12),
        ("  非饱和区完全重合", 12, True, GREEN),
        ("", 6),
        ("中间：1K 事件散点（黑色）", 13, True, DARK),
        ("  n = 3,857,310", 12),
        ("", 6),
        ("下方：1B 事件散点（蓝色）", 13, True, DARK),
        ("  n = 3,857,418", 12),
        ("  Delta = +108 (+0.003%)", 12, True, GREEN),
        ("", 8),
        ("底部：放大 T+200~350s", 13, True, DARK),
        ("  1K(黑) 和 1B(蓝) 叠加", 12),
        ("  FIFO 复位区域可见差异", 12),
        ("  但整体几乎完美对齐", 12, True, GREEN),
    ])


# ── Slide 24d: Box C scatter — 1B has MORE data ─────────────────────────────

def _slide_24d_scatter_boxC(prs):
    """Box C scatter plot — 1B 比 1K 多了 12 万事件"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Box C：1B 比 1K 多出 12 万事件！",
                  "GRB 221009A Box C — Delta = +125,858 (+3.5%) — 1K 管线数据丢失")

    img_path = os.path.join(os.path.dirname(__file__), "images", "scatter_221009A_boxC.png")
    slide.shapes.add_picture(img_path, Inches(0.3), Inches(1.1), Inches(9.0), Inches(6.2))

    add_rich_textbox(slide, Inches(9.5), Inches(1.3), Inches(3.5), Inches(5.5), [
        ("Box C 解读", 18, True, RED),
        ("", 8),
        ("上方：光变曲线对比", 13, True, DARK),
        ("  1K(灰) n=3,619,371", 12),
        ("  1B(绿) n=3,745,229", 12),
        ("  差异 +125,858 (+3.5%)", 12, True, RED),
        ("", 6),
        ("关键区域：T+510s 附近", 13, True, RED),
        ("  1B/1K 比值 = 2.04", 12, True, RED),
        ("  1B 是 1K 的两倍！", 12, True, RED),
        ("  1K 管线在此丢失了数据", 12),
        ("", 6),
        ("底部放大图：", 13, True, DARK),
        ("  1K(黑) vs 1B(绿) 叠加", 12),
        ("  绿色区域 >> 黑色区域", 12),
        ("  = 1B 保留了更完整的数据", 12, True, GREEN),
        ("", 8),
        ("→ 这不是 1B 多算了事件", 13, True, DARK),
        ("→ 是 1K 管线丢了数据", 13, True, RED),
    ])


# ── Slide 24e: Reconstructed light curve overview ────────────────────────────

def _slide_24e_recon_overview(prs):
    """重建光变曲线总览 — observed + filled"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "GRB 221009A 光变曲线重建总览",
                  "三个 Box 的观测(彩色) + FIFO gap 补全(红色) — 交叉参考填充效果")

    img_path = os.path.join(os.path.dirname(__file__), "images",
                            "recon_step_221009a.png")
    slide.shapes.add_picture(img_path, Inches(0.2), Inches(1.1), Inches(12.9), Inches(5.5))

    add_rich_textbox(slide, Inches(0.5), Inches(6.7), Inches(12), Inches(0.6), [
        ("蓝/橙/绿 = 三个 Box 的观测事件 (observed)    "
         "红色 = FIFO reset gap 补全 (filled, 用交叉参考的形状函数)    "
         "补全后光变曲线的 gap 被填充，轮廓更完整",
         13, True, DARK),
    ])


# ── Slide 24f: Zoom Box B FIFO reset recovery ───────────────────────────────

def _slide_24f_zoom_boxB(prs):
    """Box B FIFO 复位恢复区域放大 — 后向修正效果"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "Box B T+246~262s：FIFO 复位恢复区细节",
                  "后向 SEC 修正前后的逐事件对比 — 修正效果的直接证据")

    img_path = os.path.join(os.path.dirname(__file__), "images",
                            "zoom_mismatch_B_246_262.png")
    slide.shapes.add_picture(img_path, Inches(0.3), Inches(1.1), Inches(8.5), Inches(5.8))

    add_rich_textbox(slide, Inches(9.0), Inches(1.3), Inches(4.0), Inches(5.5), [
        ("三面板解读", 18, True, ORANGE),
        ("", 8),
        ("上方：1B/1K 逐秒对比", 13, True, DARK),
        ("  红色柱 = 1B/1K 比值", 12),
        ("  比值 ~ 1.0 = 一致", 12, True, GREEN),
        ("  T+258 有轻微 spike (1.27x)", 12, False, ORANGE),
        ("", 6),
        ("中间：1K 事件散点", 13, True, DARK),
        ("  FIFO 复位导致数据空洞", 12),
        ("  空洞后事件恢复", 12),
        ("", 6),
        ("下方：1B 事件散点", 13, True, DARK),
        ("  经后向 SEC 修正后", 12),
        ("  事件正确归位", 12, True, GREEN),
        ("  与 1K 结构高度一致", 12),
        ("", 8),
        ("T+249 修正效果：", 14, True, GREEN),
        ("  修正前：402/9960 (-96%)", 12, True, RED),
        ("  修正后：10065/9960 (+1.1%)", 12, True, GREEN),
    ])

def _slide_25_fifo_reset_detection(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "FIFO 复位检测",
                  "自适应阈值识别 FIFO 溢出事件")

    # Left: core algorithm
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.0), Inches(3.0), [
        ("自适应阈值", 16, True, BLUE),
        ("", 4),
        ("gap > T_baseline \u00d7 GAP_FACTOR (100)", 14, True),
        ("", 6),
        ("T_baseline = min(avg_interval_prev, avg_interval_next)", 13),
        ("  \u2192 取事件率更高的邻近数据包", 12, False, DARK_GRAY),
        ("", 6),
        ("为何自适应？", 14, True, RED),
        ("固定阈值（6.9ms）在不同计数率下失效：", 13),
        ("  \u2022 在 10000 evt/s 时：正常间隔 = 10.9ms > 6.9ms", 13),
        ("    \u2192 每个间隔都是误报！", 13, False, RED),
        ("  \u2022 GAP_FACTOR = 100\u00d7 确保只有真正的 FIFO 复位", 13),
        ("    能通过阈值，不受源计数率影响", 13),
    ])

    # Left: requirements
    add_rich_textbox(slide, Inches(0.4), Inches(4.1), Inches(6.0), Inches(2.0), [
        ("计数率要求", 16, True, GREEN),
        ("max(event_rate) > 15000 evt/s（MCU 读取速率下限）", 13),
        ("  \u2192 低于此速率，FIFO 不可能溢出", 13, False, DARK_GRAY),
        ("  \u2192 FIFO 以 R_true 填充，以 R_read \u2248 15797 evt/s 排空", 13, False, DARK_GRAY),
        ("  \u2192 仅当 R_true > R_read 时才会溢出", 13, False, DARK_GRAY),
    ])

    # Right: deep saturation extension
    add_rich_textbox(slide, Inches(6.7), Inches(1.15), Inches(6.0), Inches(2.5), [
        ("深度饱和扩展", 16, True, PURPLE),
        ("", 4),
        ("问题：在深度饱和中，相邻数据包全部拥塞", 13),
        ("\u2192 表观计数率偏低（~13000 evt/s）", 13),
        ("", 6),
        ("修正：将搜索窗口扩展到 \u00b15 个数据包，找到", 13, True),
        ("真正的高计数率数据包用于 T_baseline 估算", 13, True),
        ("", 6),
        ("防止遗漏被拥塞数据包包围的", 12, False, DARK_GRAY),
        ("FIFO 复位", 12, False, DARK_GRAY),
    ])

    # Right: one reset per gap
    add_box(slide, Inches(6.7), Inches(3.8), Inches(6.0), Inches(1.2),
            "每个检测到的间隔 = 恰好一次 FIFO 复位\n\n"
            "为什么？复位后 FIFO 为空，重新填满到溢出\n"
            "至少需要 ~9ms（FIFO 容量 / 写入速率）。\n"
            "一个包间间隔内发生两次复位在物理上不可能。",
            fill_color=LIGHT_GREEN, border_color=GREEN, font_size=12,
            text_color=DARK)

    # Bottom: diagram — timeline showing gap
    ox = Inches(6.7)
    oy = Inches(6.8)
    pw = Inches(6.0)

    add_line(slide, ox, oy, ox + pw, oy, color=DARK_GRAY, width=Pt(1.5))

    # Normal packets
    for i in range(4):
        x = ox + Inches(0.1) + Inches(i * 0.4)
        draw_bar(slide, x, oy - Inches(0.2), Inches(0.3), Inches(0.2),
                 GREEN, "", border_color=GREEN)

    # Gap
    add_textbox(slide, ox + Inches(1.8), oy - Inches(0.55), Inches(1.5), Inches(0.3),
                "FIFO 复位间隔", font_size=10, bold=True, color=RED,
                alignment=PP_ALIGN.CENTER)
    add_line(slide, ox + Inches(1.75), oy - Inches(0.05),
             ox + Inches(3.3), oy - Inches(0.05), color=RED, width=Pt(2), dashed=True)

    # Post-reset packets
    for i in range(4):
        x = ox + Inches(3.4) + Inches(i * 0.4)
        draw_bar(slide, x, oy - Inches(0.2), Inches(0.3), Inches(0.2),
                 BLUE, "", border_color=BLUE)

    add_textbox(slide, ox + Inches(0.1), oy + Inches(0.05), Inches(1.5), Inches(0.25),
                "复位前", font_size=9, color=GREEN)
    add_textbox(slide, ox + Inches(3.4), oy + Inches(0.05), Inches(1.5), Inches(0.25),
                "复位后", font_size=9, color=BLUE)


# ── Slide 25b: FIFO reset detection examples ─────────────────────────────────

def _slide_25b_fifo_reset_examples(prs):
    """FIFO 复位检测实例图"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "FIFO 复位检测实例",
                  "红色竖条 = 检测到的 FIFO Reset 区间")

    # 221009A overview (top)
    img1 = os.path.join(os.path.dirname(__file__), "images", "saturation_overview_221009A.png")
    slide.shapes.add_picture(img1, Inches(0.2), Inches(1.1), Inches(8.5), Inches(3.0))

    add_rich_textbox(slide, Inches(8.9), Inches(1.2), Inches(4.2), Inches(2.8), [
        ("GRB 221009A（极端饱和）", 16, True, BLUE),
        ("", 6),
        ("三个 Box 光变曲线 + FIFO Reset 区间", 13),
        ("", 4),
        ("Box A: 1,799 个 FIFO Reset", 13, True, DARK),
        ("Box B: 1,621 个 FIFO Reset", 13, True, DARK),
        ("Box C: 1,673 个 FIFO Reset", 13, True, DARK),
        ("", 6),
        ("红色竖条密集区 = 饱和严重时段", 13, True, RED),
        ("T+200~350s 尤为集中", 12, False, DARK_GRAY),
    ])

    # 260226A overview (bottom)
    img2 = os.path.join(os.path.dirname(__file__), "images", "saturation_overview_260226A.png")
    slide.shapes.add_picture(img2, Inches(0.2), Inches(4.2), Inches(8.5), Inches(3.0))

    add_rich_textbox(slide, Inches(8.9), Inches(4.3), Inches(4.2), Inches(2.8), [
        ("GRB 260226A（中度饱和）", 16, True, GREEN),
        ("", 6),
        ("FIFO Reset 集中在峰值附近", 13),
        ("", 4),
        ("Box A: 77 个 FIFO Reset", 13, True, DARK),
        ("Box B: 66 个 FIFO Reset", 13, True, DARK),
        ("Box C: 58 个 FIFO Reset", 13, True, DARK),
        ("", 6),
        ("峰值前后的红色竖条清晰可见", 13, True, RED),
        ("远少于 221009A → 饱和程度低", 12, False, DARK_GRAY),
    ])


# ── Slide 26: Silent drop detection ──────────────────────────────────────────

def _slide_26_silent_drop(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "静默丢数检测（泊松方法）",
                  "包内事件丢失的统计检测")

    # Left: statistical derivation
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.0), Inches(3.5), [
        ("物理模型", 16, True, BLUE),
        ("在拥塞数据包中，MCU 以 ~15797 evt/s 读取", 13),
        ("\u2192 事件间隔近似均匀（~63\u03bcs）", 13),
        ("", 6),
        ("统计检验", 14, True),
        ("正常间隔分布：参数为 \u03bb 的指数分布", 13),
        ("间隔 \u0394t 的概率：p = exp(-\u03bb\u0394t)", 13),
        ("log\u2081\u2080(p) = -\u03bb\u0394t / ln(10)", 13),
        ("", 6),
        ("阈值：log\u2081\u2080(p) < -10", 14, True, RED),
        ("\u2192 P < 10\u207b\u00b9\u2070 = 随机出现几乎不可能", 13, False, RED),
        ("", 6),
        ("\u03bb 估算：使用过滤后的间隔（<1ms）以避免", 13),
        ("异常间隔本身偏置计数率估计", 13),
    ])

    # Left: special cases
    add_rich_textbox(slide, Inches(0.4), Inches(4.6), Inches(6.0), Inches(2.5), [
        ("特殊情况", 16, True, ORANGE),
        ("", 4),
        ("拥塞宽数据包（span > 3\u00d7 邻近中位数 span）：", 13, True),
        ("  \u2192 使用邻近数据包的计数率作为 \u03bb", 12),
        ("  原因：宽包自身的计数率反映拥塞延迟，", 12, False, DARK_GRAY),
        ("  而非真实源计数率", 12, False, DARK_GRAY),
        ("", 6),
        ("深度饱和（所有邻近包均拥塞）：", 13, True),
        ("  \u2192 跳过检测（\u03bb 不可靠）", 12),
        ("  原因：无可靠的计数率参考", 12, False, DARK_GRAY),
        ("", 6),
        ("丢失事件数：N_lost = \u03bb\u0394t \u2212 1", 13, True),
    ])

    # Right: detection results
    add_rich_textbox(slide, Inches(6.7), Inches(1.15), Inches(6.0), Inches(2.5), [
        ("检测结果：GRB 221009A", 16, True, GREEN),
        ("", 4),
        ("识别出 138 个候选静默丢数", 14, True),
        ("", 6),
        ("集中在数据包边界：", 13),
        ("  \u2022 事件索引 0：前一包最后事件与当前包首事件间的间隔", 12),
        ("", 12),
        ("  \u2022 事件索引 106：109 事件数据包末尾附近", 12),
        ("", 6),
        ("这种边界集中现象符合物理预期：", 12, False, DARK_GRAY),
        ("MCU 在包间切换 = 短暂的读取中断", 12, False, DARK_GRAY),
    ])

    # Right: visual — Poisson probability scale
    add_textbox(slide, Inches(6.7), Inches(3.8), Inches(5.5), Inches(0.35),
                "概率标尺 (log\u2081\u2080)：", font_size=13, bold=True, color=DARK)

    scale_y = Inches(4.25)
    scale_x = Inches(6.7)
    total_w = Inches(5.5)

    # Draw probability bar
    # Normal region (green)
    draw_bar(slide, scale_x, scale_y, Inches(2.5), Inches(0.4),
             LIGHT_GREEN, "正常 (p > 10\u207b\u00b3)", font_size=10,
             text_color=GREEN, border_color=GREEN)
    # Suspicious
    draw_bar(slide, scale_x + Inches(2.5), scale_y, Inches(1.5), Inches(0.4),
             LIGHT_ORANGE, "可疑", font_size=10,
             text_color=ORANGE, border_color=ORANGE)
    # Anomalous
    draw_bar(slide, scale_x + Inches(4.0), scale_y, Inches(1.5), Inches(0.4),
             LIGHT_RED, "异常 (<10\u207b\u00b9\u2070)", font_size=10,
             text_color=RED, border_color=RED)

    # Scale labels
    add_textbox(slide, scale_x, scale_y + Inches(0.4), Inches(0.5), Inches(0.25),
                "0", font_size=9, color=GRAY, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, scale_x + Inches(2.35), scale_y + Inches(0.4), Inches(0.5), Inches(0.25),
                "-3", font_size=9, color=GRAY, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, scale_x + Inches(3.85), scale_y + Inches(0.4), Inches(0.5), Inches(0.25),
                "-10", font_size=9, color=RED, alignment=PP_ALIGN.CENTER)

    # Formula box
    add_box(slide, Inches(6.7), Inches(5.2), Inches(5.8), Inches(1.8),
            "检测公式\n\n"
            "1. 从过滤后的间隔计算 \u03bb（排除 > 1ms 的间隔）\n"
            "2. 对每个间隔 \u0394t：score = -\u03bb\u0394t / ln(10)\n"
            "3. 若 score < -10：标记为静默丢数\n"
            "4. N_lost = \u03bb\u0394t \u2212 1",
            fill_color=LIGHT_BLUE, border_color=BLUE, font_size=12,
            text_color=DARK)


# ── Slide 26b: Silent drop detection examples ────────────────────────────────

def _slide_26b_silent_drop_examples(prs):
    """静默丢数检测实例图"""
    import os
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "静默丢数检测实例",
                  "包内事件间隔分布 + 单包三面板诊断")

    # Top: interval distribution
    img1 = os.path.join(os.path.dirname(__file__), "images",
                        "silent_drop_intervals_221009a.png")
    slide.shapes.add_picture(img1, Inches(0.2), Inches(1.1), Inches(12.9), Inches(2.8))

    add_rich_textbox(slide, Inches(0.5), Inches(3.95), Inches(12.5), Inches(0.5), [
        ("上图：221009A 三个 Box 高率包的事件间隔分布（对数坐标）。"
         "红线 = 指数分布拟合。"
         "超出指数分布的长尾 (>500us) = 静默丢数候选。",
         12, True, DARK),
    ])

    # Bottom: single packet detail
    img2 = os.path.join(os.path.dirname(__file__), "images",
                        "sd_01_boxA_pkt36731.png")
    slide.shapes.add_picture(img2, Inches(0.2), Inches(4.5), Inches(8.5), Inches(2.8))

    add_rich_textbox(slide, Inches(8.9), Inches(4.5), Inches(4.2), Inches(2.8), [
        ("单包诊断：Box A #36731", 14, True, RED),
        ("", 6),
        ("上面板：+-100ms 概览", 12, True, DARK),
        ("  蓝条=包，红色虚线=异常间隔位置", 11),
        ("", 4),
        ("中面板：放大到 +-5.8ms", 12, True, DARK),
        ("  事件 tick 可见（红色竖线）", 11),
        ("  2454us 和 1508us 两个异常间隔", 11, True, RED),
        ("", 4),
        ("下面板：包内间隔柱状图", 12, True, DARK),
        ("  正常间隔 ~16us（lambda=63253 evt/s）", 11),
        ("  异常间隔 2454us → log10(p)=-67.4", 11, True, RED),
        ("  异常间隔 1508us → log10(p)=-41.4", 11, True, RED),
        ("  远超 -10 阈值 → 确认静默丢数", 11),
    ])


# ── Slide 27: R_true estimation ──────────────────────────────────────────────

def _slide_27_r_true(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "R_true 估算：关键物理约束",
                  "为何拥塞数据包的 span 不反映真实源计数率")

    # Left: FIFO-full physics
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(6.2), Inches(3.2), [
        ("FIFO 满载动力学", 16, True, BLUE),
        ("", 4),
        ("FIFO 满载期间：", 14, True),
        ("  1. MCU 读取 1 个事件 \u2192 FIFO 空出 1 个位置", 13),
        ("  2. FPGA 立即写入 1 个新事件", 13),
        ("  3. 新事件的 ptime = 当前时间（NOW）", 13),
        ("", 6),
        ("结果：拥塞数据包中连续事件的", 13, True, RED),
        ("ptime 间距 \u2248 1/R_read（MCU 速度），而非 1/R_true", 13, True, RED),
        ("", 6),
        ("数据包 span T_span \u2248 109/R_read \u2248 109/15797 \u2248 6.9ms", 13),
        ("\u2192 此值为常数，与真实源计数率无关！", 13, False, RED),
        ("", 6),
        ("不能用拥塞数据包 span 来估算 R_true", 13, True, RED),
    ])

    # Left: post-reset solution
    add_rich_textbox(slide, Inches(0.4), Inches(4.3), Inches(6.2), Inches(2.8), [
        ("解决方案：复位后数据包", 16, True, GREEN),
        ("", 4),
        ("FIFO 复位后：FIFO 为空", 13),
        ("\u2192 事件直接进入，无阻塞", 13),
        ("\u2192 span 反映真实源计数率 R_true", 13),
        ("", 6),
        ("R_true \u2248 109 / T_span(post-reset)", 14, True),
        ("", 6),
        ("验证：复位前/复位后 span 比值 \u2248 0.93", 13),
        ("\u2192 R_true 仅比 R_read 高 ~10-20%", 13, False, DARK_GRAY),
        ("\u2192 中度饱和期间，大部分事件被捕获", 13, False, DARK_GRAY),
    ])

    # Right: diagram — read-write dynamics
    rx = Inches(7.0)
    ry = Inches(1.15)
    add_textbox(slide, rx, ry, Inches(5.5), Inches(0.35),
                "FIFO 读写循环（饱和期间）", font_size=14, bold=True)

    # Cycle diagram: boxes showing FIFO states
    cy = ry + Inches(0.6)
    step_h = Inches(0.6)
    box_w = Inches(4.5)

    states = [
        ("FIFO 满载：2048 事件等待中", LIGHT_RED, RED),
        ("MCU 读取 1 事件 \u2192 剩余 2047 事件", LIGHT_GREEN, GREEN),
        ("FPGA 写入 1 事件 \u2192 2048 事件 (ptime=NOW)", LIGHT_BLUE, BLUE),
        ("MCU 读取 1 事件 \u2192 剩余 2047 事件", LIGHT_GREEN, GREEN),
        ("FPGA 写入 1 事件 \u2192 2048 事件 (ptime=NOW)", LIGHT_BLUE, BLUE),
    ]
    for i, (txt, fill, border) in enumerate(states):
        y = cy + i * (step_h + Inches(0.05))
        add_box(slide, rx, y, box_w, step_h - Inches(0.05), txt,
                fill_color=fill, border_color=border, font_size=11, text_color=DARK)
        if i < len(states) - 1:
            add_arrow(slide, rx + box_w / 2, y + step_h - Inches(0.05),
                      rx + box_w / 2, y + step_h + Inches(0.05),
                      color=DARK_GRAY, width=Pt(1))

    add_textbox(slide, rx + box_w + Inches(0.2), cy + Inches(0.2),
                Inches(1.5), Inches(0.5),
                "\u0394t \u2248 1/R_read\n\u2248 63\u03bcs", font_size=11, color=RED)

    # Deep saturation box
    add_box(slide, Inches(7.0), Inches(5.2), Inches(5.5), Inches(1.1),
            "深度饱和扩展\n\n"
            "复位后数据包同样拥塞 \u2192 span 反映 R_read\n"
            "回退方案：burst rate（<1ms 间隔的事件）作为 R_true 估算",
            fill_color=LIGHT_PURPLE, border_color=PURPLE, font_size=12,
            text_color=DARK)

    # Key numbers
    add_box(slide, Inches(7.0), Inches(6.5), Inches(5.5), Inches(0.6),
            "R_read \u2248 15797 evt/s  |  T_span(拥塞) \u2248 6.9ms  |  "
            "FIFO 容量 = 2048 事件",
            fill_color=LIGHT_GRAY, border_color=GRAY, font_size=12, bold=True,
            text_color=DARK)


# ── Slide 28: FIFO gap filling + cross-box reference ─────────────────────────

def _slide_28_fifo_gap_filling(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "FIFO 复位 gap 补全与三机箱交叉参考",
                  "利用姊妹探测器机箱重建丢失事件")

    # Left: algorithm
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(5.6), Inches(2.8), [
        ("丢失事件数", 16, True, BLUE),
        ("N_lost = R_true \u00d7 T_gap", 14, True),
        ("", 6),
        ("交叉参考形状函数", 16, True, GREEN),
        ("3 个机箱（A、B、C）观测同一源", 13),
        ("\u2192 各时刻计数率成正比", 13),
        ("", 6),
        ("当机箱 A 有 gap 时，机箱 B 和 C 可能有好数据：", 13),
        ("\u2022 从参考机箱在 gap 时间范围内的事件构建 1ms 分辨率形状", 12),
        ("", 12),
        ("\u2022 将形状归一化到 N_lost 个事件", 12),
        ("\u2022 按形状比例分配事件", 12),
    ])

    # Left: unreliable interval filtering
    add_rich_textbox(slide, Inches(0.4), Inches(3.9), Inches(5.6), Inches(2.8), [
        ("不可靠区间过滤（关键步骤）", 16, True, RED),
        ("参考机箱数据需过滤 3 类不可靠区间：", 13),
        ("", 4),
        ("\u2460 FIFO 复位间隔（无事件 \u2192 零计数）", 12, True, ORANGE),
        ("    会在形状函数中产生人为凹陷", 11, False, DARK_GRAY),
        ("", 4),
        ("\u2461 拥塞宽数据包（span > 3\u00d7 邻近中位数）", 12, True, ORANGE),
        ("    反映 MCU 读取模式，而非源变化", 11, False, DARK_GRAY),
        ("", 4),
        ("\u2462 含泊松异常间隔的数据包", 12, True, ORANGE),
        ("    静默丢数扭曲局部计数率", 11, False, DARK_GRAY),
        ("", 6),
        ("多参考机箱：取平均形状，用目标机箱的", 12),
        ("gap 端点事件计数进行校准", 12),
    ])

    # Right: cross-reference diagram
    rx = Inches(6.5)
    ry = Inches(1.3)
    add_textbox(slide, rx, ry, Inches(6.0), Inches(0.35),
                "三机箱交叉参考示意图", font_size=14, bold=True)

    box_w = Inches(5.5)
    lane_h = Inches(1.0)
    lane_gap = Inches(0.15)

    for i, (label, lane_color) in enumerate([("机箱 A（目标）", RED),
                                              ("机箱 B（参考）", GREEN),
                                              ("机箱 C（参考）", BLUE)]):
        y = ry + Inches(0.5) + i * (lane_h + lane_gap)
        add_textbox(slide, rx, y, Inches(1.3), Inches(0.3),
                    label, font_size=11, bold=True, color=lane_color)

        # Timeline
        tx = rx + Inches(1.4)
        add_line(slide, tx, y + Inches(0.5), tx + Inches(4.2), y + Inches(0.5),
                 color=GRAY, width=Pt(1))

        if i == 0:
            # Box A: gap in the middle
            draw_bar(slide, tx, y + Inches(0.25), Inches(1.2), Inches(0.5),
                     LIGHT_GREEN, "数据", font_size=9, text_color=GREEN, border_color=GREEN)
            draw_bar(slide, tx + Inches(1.2), y + Inches(0.25), Inches(1.8), Inches(0.5),
                     LIGHT_RED, "FIFO 复位 GAP", font_size=9, text_color=RED,
                     border_color=RED)
            draw_bar(slide, tx + Inches(3.0), y + Inches(0.25), Inches(1.2), Inches(0.5),
                     LIGHT_GREEN, "数据", font_size=9, text_color=GREEN, border_color=GREEN)
        else:
            # Box B/C: continuous data (with some unreliable marks)
            draw_bar(slide, tx, y + Inches(0.25), Inches(1.5), Inches(0.5),
                     LIGHT_GREEN, "良好", font_size=9, text_color=GREEN, border_color=GREEN)
            draw_bar(slide, tx + Inches(1.5), y + Inches(0.25), Inches(0.5), Inches(0.5),
                     LIGHT_ORANGE, "跳过", font_size=8, text_color=ORANGE,
                     border_color=ORANGE)
            draw_bar(slide, tx + Inches(2.0), y + Inches(0.25), Inches(2.2), Inches(0.5),
                     LIGHT_GREEN, "良好", font_size=9, text_color=GREEN, border_color=GREEN)

    # Arrows from B and C to A's gap
    gap_center_x = rx + Inches(1.4) + Inches(2.1)
    a_y = ry + Inches(0.5) + Inches(0.25)
    b_y = ry + Inches(0.5) + (lane_h + lane_gap) + Inches(0.25)
    c_y = ry + Inches(0.5) + 2 * (lane_h + lane_gap) + Inches(0.25)
    add_arrow(slide, gap_center_x - Inches(0.2), b_y, gap_center_x - Inches(0.2), a_y + Inches(0.55),
              color=GREEN, width=Pt(1.5))
    add_arrow(slide, gap_center_x + Inches(0.2), c_y, gap_center_x + Inches(0.2), a_y + Inches(0.55),
              color=BLUE, width=Pt(1.5))

    # Coverage fallback
    add_box(slide, Inches(6.5), Inches(5.3), Inches(6.2), Inches(0.5),
            "覆盖率 < 30% 的 gap 时间段有参考数据 \u2192 退化为均匀分布",
            fill_color=LIGHT_ORANGE, border_color=ORANGE, font_size=12, text_color=DARK)

    # Shape normalization
    add_box(slide, Inches(6.5), Inches(5.95), Inches(6.2), Inches(1.2),
            "形状归一化流程\n\n"
            "1. 从机箱 B + C 收集 [gap_start, gap_end] 范围内的参考事件\n"
            "2. 过滤不可靠区间 \u2192 形状 bin（1ms 分辨率）\n"
            "3. 对多个参考机箱的形状取平均\n"
            "4. 将总数归一化到 N_lost \u2192 按比例分配事件",
            fill_color=LIGHT_BLUE, border_color=BLUE, font_size=11, text_color=DARK)


# ── Slide 29: Coverage statistics ────────────────────────────────────────────

def _slide_29_coverage(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "交叉参考覆盖率统计",
                  "GRB 221009A：三个独立 FIFO 提供高互补覆盖率")

    # Coverage table
    col_w = [Inches(2.5), Inches(1.5), Inches(1.5), Inches(1.5)]
    rows = [
        ["参考可用性", "机箱 A", "机箱 B", "机箱 C"],
        ["双参考可用", "49.2%", "38.3%", "46.8%"],
        ["至少 1 个参考", "92.0%", "92.0%", "92.6%"],
        ["无参考（三箱同时饱和）", "8.0%", "8.0%", "7.4%"],
    ]
    draw_table(slide, Inches(0.5), Inches(1.3), col_w, rows,
               row_height=Inches(0.5), font_size=13)

    # Explanation
    add_rich_textbox(slide, Inches(0.5), Inches(3.7), Inches(6.5), Inches(3.5), [
        ("为何 ~92% 覆盖率？", 18, True, GREEN),
        ("", 6),
        ("三个独立 FIFO 具有独立的溢出时序：", 14),
        ("", 4),
        ("\u2022 每个机箱有自己的 FIFO \u2192 复位发生在不同", 13),
        ("  时刻，取决于各机箱特定的事件率", 13),
        ("\u2022 当机箱 A 处于 FIFO 复位间隔时，机箱 B 和 C", 13),
        ("  很可能仍在正常读取（或处于不同的间隔）", 13),
        ("\u2022 仅在最强 GRB 峰值期间，三个机箱才", 13),
        ("  同时饱和", 13),
        ("", 8),
        ("为何 ~8% 同时饱和？", 18, True, RED),
        ("", 6),
        ("在 GRB 峰值流量期间，源计数率同时超过所有", 14),
        ("3 个机箱的 MCU 读取速率 \u2192 3 个 FIFO 同时", 14),
        ("溢出 \u2192 无交叉参考可用。", 14),
        ("", 6),
        ("这些区间使用均匀分布作为回退方案。", 13, False, DARK_GRAY),
    ])

    # Right: visual diagram of 3-box independence
    rx = Inches(7.5)
    ry = Inches(3.7)

    add_textbox(slide, rx, ry, Inches(5.0), Inches(0.35),
                "时间线：三机箱独立性", font_size=14, bold=True)

    for i, (label, c, lc) in enumerate([("A", RED, LIGHT_RED),
                                         ("B", GREEN, LIGHT_GREEN),
                                         ("C", BLUE, LIGHT_BLUE)]):
        y = ry + Inches(0.5) + i * Inches(0.7)
        add_textbox(slide, rx, y, Inches(0.4), Inches(0.3),
                    label, font_size=12, bold=True, color=c)

        # Timeline with gaps at different positions
        tx = rx + Inches(0.5)
        total = Inches(4.5)

        if i == 0:  # Box A: gap in position 1
            draw_bar(slide, tx, y, Inches(1.0), Inches(0.35), lc, "",
                     border_color=c)
            draw_bar(slide, tx + Inches(1.0), y, Inches(0.8), Inches(0.35), c, "gap",
                     font_size=8, text_color=WHITE, border_color=c)
            draw_bar(slide, tx + Inches(1.8), y, Inches(2.7), Inches(0.35), lc, "",
                     border_color=c)
        elif i == 1:  # Box B: gap in position 2
            draw_bar(slide, tx, y, Inches(2.0), Inches(0.35), lc, "",
                     border_color=c)
            draw_bar(slide, tx + Inches(2.0), y, Inches(0.7), Inches(0.35), c, "gap",
                     font_size=8, text_color=WHITE, border_color=c)
            draw_bar(slide, tx + Inches(2.7), y, Inches(1.8), Inches(0.35), lc, "",
                     border_color=c)
        else:  # Box C: gap in position 3
            draw_bar(slide, tx, y, Inches(3.2), Inches(0.35), lc, "",
                     border_color=c)
            draw_bar(slide, tx + Inches(3.2), y, Inches(0.6), Inches(0.35), c, "gap",
                     font_size=8, text_color=WHITE, border_color=c)
            draw_bar(slide, tx + Inches(3.8), y, Inches(0.7), Inches(0.35), lc, "",
                     border_color=c)

    add_textbox(slide, rx, ry + Inches(2.8), Inches(5.0), Inches(0.3),
                "gap 很少重叠 \u2192 互补参考可用性高",
                font_size=11, color=DARK_GRAY)

    # Simultaneous saturation zone
    add_box(slide, rx, ry + Inches(3.2), Inches(5.0), Inches(0.5),
            "同时饱和（8%）：均匀填充作为最终回退方案",
            fill_color=LIGHT_RED, border_color=RED, font_size=11, text_color=DARK)


# ── Slide 30: Silent drop + deep saturation filling ─────────────────────────

def _slide_30_silent_deep_filling(prs):
    slide = add_blank_slide(prs)
    add_title_bar(prs, slide, "静默丢数与深度饱和补全",
                  "针对不同丢失模式的三种独立重建策略")

    # Left column: Silent drop filling
    add_rich_textbox(slide, Inches(0.4), Inches(1.15), Inches(4.0), Inches(3.0), [
        ("策略 1：静默丢数补全", 15, True, BLUE),
        ("", 4),
        ("对每个泊松异常间隔：", 13),
        ("  N_lost = \u03bb\u0394t \u2212 1", 13, True),
        ("", 6),
        ("时间分布：", 13, True),
        ("  \u2022 与 FIFO gap 补全相同的交叉参考方法", 12),
        ("  \u2022 参考机箱提供 gap 时间范围内的形状", 12),
        ("  \u2022 过滤参考中的不可靠区间", 12),
        ("  \u2022 覆盖率 < 30% \u2192 均匀分布", 12),
        ("", 6),
        ("适用范围：仅轻度饱和", 12, False, DARK_GRAY),
        ("  (neighbor_rate \u2265 15000 evt/s)", 12, False, DARK_GRAY),
    ])

    # Middle column: Deep saturation filling
    add_rich_textbox(slide, Inches(4.6), Inches(1.15), Inches(4.2), Inches(3.0), [
        ("策略 2：深度饱和补全", 15, True, PURPLE),
        ("", 4),
        ("对每个深度饱和数据包：", 13),
        ("  R_true = burst rate（<1ms 间隔）", 13, True),
        ("", 6),
        ("包内 gap 识别：", 13, True),
        ("  \u2022 找出包内所有 > 1ms 的间隔", 12),
        ("  \u2022 N_fill = R_true \u00d7 \u03a3(gap 时长)", 12),
        ("  \u2022 事件在 gap 内均匀分布", 12),
        ("", 6),
        ("无需交叉参考：", 12, False, DARK_GRAY),
        ("  深度饱和 = 所有机箱很可能都饱和", 12, False, DARK_GRAY),
        ("  \u2192 均匀分布是唯一可行策略", 12, False, DARK_GRAY),
    ])

    # Right column: FIFO gap (already covered, summary)
    add_rich_textbox(slide, Inches(9.0), Inches(1.15), Inches(4.0), Inches(3.0), [
        ("策略 3：FIFO gap 补全", 15, True, GREEN),
        ("", 4),
        ("对每个 FIFO 复位 gap：", 13),
        ("  N_lost = R_true \u00d7 T_gap", 13, True),
        ("", 6),
        ("R_true 来源：", 13, True),
        ("  \u2022 正常：复位后数据包 span", 12),
        ("  \u2022 深度饱和：burst rate 回退", 12),
        ("", 6),
        ("形状来自交叉参考机箱", 12),
        ("  （见幻灯片 27-29）", 12, False, DARK_GRAY),
    ])

    # Independence box
    add_box(slide, Inches(0.4), Inches(4.3), Inches(12.5), Inches(0.6),
            "三种策略完全独立：每种仅使用原始观测事件作为输入。"
            "任何策略的输出不会馈入其他策略。",
            fill_color=LIGHT_ORANGE, border_color=ORANGE, font_size=13, bold=True,
            text_color=DARK)

    # Bottom: comparison diagram
    add_textbox(slide, Inches(0.4), Inches(5.1), Inches(12.0), Inches(0.35),
                "丢失模式对比", font_size=16, bold=True)

    col_w = [Inches(2.5), Inches(2.5), Inches(2.5), Inches(2.5), Inches(2.5)]
    rows = [
        ["丢失模式", "原因", "检测方法", "N_lost", "时间形状"],
        ["FIFO 复位 gap", "FIFO 溢出 \u2192 复位",
         "自适应 gap 阈值", "R_true \u00d7 T_gap", "交叉参考"],
        ["静默丢数", "短暂 FIFO 满载，无复位",
         "泊松异常 (p<10\u207b\u00b9\u2070)", "\u03bb\u0394t \u2212 1", "交叉参考"],
        ["深度饱和", "持续 FIFO 满载",
         "所有邻近包拥塞", "R_burst \u00d7 \u03a3gaps", "均匀分布"],
    ]
    draw_table(slide, Inches(0.4), Inches(5.5), col_w, rows,
               row_height=Inches(0.45), font_size=11)
