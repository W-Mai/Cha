#!/usr/bin/env python3
"""
Cha Logo Generator
Character 察 formed by code quality keywords with a magnifying glass overlay.
All text converted to outlines. Severity prefixes color-coded.
"""

import math
import os
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from PIL import Image, ImageDraw, ImageFont

# --- Config ---
CANVAS = 512
PADDING = 28
CORNER_RADIUS = 115  # ~22.37% of 512, matching Apple iOS icon spec
BG_COLOR = "#1c1917"

GRID_SIZE = 36
CHA_FONT = os.path.expanduser("~/Library/Fonts/AlibabaPuHuiTi-2-95-ExtraBold.ttf")
CHA_FONT_SCALE = 0.88
CHA_THRESHOLD = 100

ENG_FONT = os.path.expanduser("~/Library/Fonts/AGENCYB.ttf")

# Magnifying glass config (in canvas coords)
LENS_CX, LENS_CY, LENS_R = 340, 345, 65
HANDLE_LEN = 50
LENS_STROKE = 12

# Colors
COLOR_DEFAULT = "#f97316"
COLOR_ERROR = "#ef4444"
COLOR_WARNING = "#fb923c"
COLOR_HINT = "#60a5fa"
COLOR_SUGGEST = "#4ade80"
COLOR_LENS = "#f97316"
COLOR_CHECK = "#4ade80"

KEYWORDS = [
    "W:LONG_METHOD(32)", "E:COMPLEXITY=22", "W:LARGE_CLASS(11)",
    "H:DEAD_CODE:FN", "W:DUPLICATE_AST", "E:LAYER_VIOLATION",
    "W:COUPLING=18", "H:NAMING<2", "W:API_SURFACE=80%",
    "→EXTRACT_METHOD", "→EXTRACT_CLASS", "→MOVE_METHOD",
    "W:LONG_FN(45)", "E:COMPLEXITY=31", "H:UNUSED_EXPORT",
    "W:DUPLICATE(2)", "E:IMPORT_CYCLE", "H:NAMING>50",
    "→REPLACE_COND", "→HIDE_DELEGATE", "→INLINE_CLASS",
    "W:PARAMS=8", "E:DEPTH=6", "H:DEAD_CLASS",
    "W:FILE(520)", "E:BRANCH=25", "W:METHODS=15",
    "→FORM_TEMPLATE", "→PULL_UP_FN", "→PUSH_DOWN",
    "W:SMELL:BLOAT", "E:SMELL:COUPL", "H:SMELL:DISP",
    "→REFACTOR:NOW", "W:THRESHOLD!", "E:SEVERITY:ERR",
]


def word_color(word):
    if word.startswith("E:"):
        return COLOR_ERROR
    if word.startswith("W:"):
        return COLOR_WARNING
    if word.startswith("H:"):
        return COLOR_HINT
    if word.startswith("\u2192"):
        return COLOR_SUGGEST
    return COLOR_DEFAULT


def rasterize_cha():
    img = Image.new('L', (GRID_SIZE, GRID_SIZE), 0)
    draw = ImageDraw.Draw(img)
    font = ImageFont.truetype(CHA_FONT, size=int(GRID_SIZE * CHA_FONT_SCALE))
    bbox = font.getbbox('察')
    w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
    x = (GRID_SIZE - w) // 2 - bbox[0]
    y = (GRID_SIZE - h) // 2 - bbox[1]
    draw.text((x, y), '察', fill=255, font=font)

    # Save step 1
    img.resize((512, 512), Image.NEAREST).save("/tmp/cha_debug_step1.png")

    # Punch a black circle on the bitmap directly
    usable = CANVAS - 2 * PADDING
    cell = usable / GRID_SIZE
    lcx = (LENS_CX - PADDING) / cell
    lcy = (LENS_CY - PADDING) / cell
    lr = (LENS_R + 16) / cell  # generous padding around lens
    draw.ellipse(
        [lcx - lr, lcy - lr, lcx + lr, lcy + lr],
        fill=0
    )

    # Save step 2
    img.resize((512, 512), Image.NEAREST).save("/tmp/cha_debug_step2.png")

    # Now extract: outside = what remains, lens = re-render 察 clipped to circle only
    outside_rows = {}
    for row in range(GRID_SIZE):
        for col in range(GRID_SIZE):
            if img.getpixel((col, row)) > CHA_THRESHOLD:
                outside_rows.setdefault(row, set()).add(col)

    # Re-render for lens area: original 察 intersected with circle
    img2 = Image.new('L', (GRID_SIZE, GRID_SIZE), 0)
    draw2 = ImageDraw.Draw(img2)
    draw2.text((x, y), '察', fill=255, font=font)
    # Keep only pixels inside the circle
    lens_rows = {}
    for row in range(GRID_SIZE):
        for col in range(GRID_SIZE):
            if img2.getpixel((col, row)) <= CHA_THRESHOLD:
                continue
            dist = math.sqrt((col + 0.5 - lcx) ** 2 + (row + 0.5 - lcy) ** 2)
            if dist < lr - 0.5:
                lens_rows.setdefault(row, set()).add(col)

    return outside_rows, lens_rows


def find_runs(cols):
    cols = sorted(cols)
    runs, start, prev = [], cols[0], cols[0]
    for c in cols[1:]:
        if c == prev + 1:
            prev = c
        else:
            runs.append((start, prev))
            start = prev = c
    runs.append((start, prev))
    return runs


def clip_run_to_circle(run_x, run_end_x, row_y, cx, cy, r):
    """Clip a horizontal run against a circle. Returns list of (x_start, x_end) segments outside the circle."""
    # Find intersection of horizontal line y=row_y with circle
    dy = row_y - cy
    if abs(dy) >= r:
        return [(run_x, run_end_x)]  # entirely outside

    dx = math.sqrt(r * r - dy * dy)
    circle_left = cx - dx
    circle_right = cx + dx

    segments = []
    if run_x < circle_left:
        segments.append((run_x, min(run_end_x, circle_left)))
    if run_end_x > circle_right:
        segments.append((max(run_x, circle_right), run_end_x))

    return segments


def run_inside_circle(run_x, run_end_x, row_y, cx, cy, r):
    """Get the portion of a run that's inside the circle."""
    dy = row_y - cy
    if abs(dy) >= r:
        return None

    dx = math.sqrt(r * r - dy * dy)
    circle_left = cx - dx
    circle_right = cx + dx

    inside_left = max(run_x, circle_left)
    inside_right = min(run_end_x, circle_right)

    if inside_left < inside_right:
        return (inside_left, inside_right)
    return None


_font_cache = {}

def get_font_data():
    if "data" not in _font_cache:
        font = TTFont(ENG_FONT)
        _font_cache["data"] = {
            "font": font,
            "glyph_set": font.getGlyphSet(),
            "cmap": font.getBestCmap(),
            "upm": font['head'].unitsPerEm,
            "hmtx": font['hmtx'],
        }
    return _font_cache["data"]


def render_text_run(words, font_size, x, y, target_width):
    fd = get_font_data()
    scale = font_size / fd["upm"]
    space_advance = fd["upm"] * 0.25

    segments = []
    cursor = 0
    for i, word in enumerate(words):
        word_start = cursor
        for ch in word:
            glyph_name = fd["cmap"].get(ord(ch))
            if glyph_name:
                advance, _ = fd["hmtx"][glyph_name]
                cursor += advance
            else:
                cursor += fd["upm"] * 0.3
        segments.append((word, word_color(word), word_start))
        if i < len(words) - 1:
            cursor += space_advance

    total_upm_width = cursor
    if total_upm_width <= 0:
        return ""

    # Scale x to fill target_width, keep y at font_size scale
    h_scale = target_width / (total_upm_width * scale)
    # Skip if text would be too compressed to read
    if h_scale < 0.5:
        return ""
    result = [f'<g transform="translate({x:.2f},{y:.2f}) scale({scale * h_scale:.6f},{-scale:.6f})">']

    for word, color, word_start_upm in segments:
        word_paths = []
        ch_cursor = word_start_upm
        for ch in word:
            glyph_name = fd["cmap"].get(ord(ch))
            if not glyph_name:
                ch_cursor += fd["upm"] * 0.3
                continue
            pen = SVGPathPen(fd["glyph_set"])
            fd["glyph_set"][glyph_name].draw(pen)
            path_data = pen.getCommands()
            if path_data:
                word_paths.append(f'<path d="{path_data}" transform="translate({ch_cursor},0)"/>')
            advance, _ = fd["hmtx"][glyph_name]
            ch_cursor += advance

        if word_paths:
            result.append(f'<g fill="{color}">{"".join(word_paths)}</g>')

    result.append('</g>')
    return "".join(result)



def word_to_paths_colored(text, color, font_size, x, y):
    """Render text at natural width, no stretching, single color."""
    fd = get_font_data()
    scale = font_size / fd["upm"]
    paths = []
    cursor = 0
    for ch in text:
        glyph_name = fd["cmap"].get(ord(ch))
        if not glyph_name:
            cursor += fd["upm"] * 0.3
            continue
        pen = SVGPathPen(fd["glyph_set"])
        fd["glyph_set"][glyph_name].draw(pen)
        path_data = pen.getCommands()
        if path_data:
            paths.append(f'<path d="{path_data}" transform="translate({cursor},0)"/>')
        advance, _ = fd["hmtx"][glyph_name]
        cursor += advance
    actual_width = cursor * scale
    inner = "".join(paths)
    return f'<g fill="{color}" transform="translate({x:.2f},{y:.2f}) scale({scale:.6f},{-scale:.6f})">{inner}</g>', actual_width

def fill_words(max_chars, ki):
    words = []
    total_len = 0
    while total_len < max_chars:
        word = KEYWORDS[ki % len(KEYWORDS)]
        ki += 1
        if total_len > 0:
            total_len += 1
        total_len += len(word)
        words.append(word)
    return words, ki


def generate_logo():
    outside_rows, lens_rows = rasterize_cha()
    usable = CANVAS - 2 * PADDING
    cell_size = usable / GRID_SIZE
    font_size = cell_size * 0.78
    lens_font_size = font_size * 1.15
    char_width = font_size * 0.48

    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CANVAS} {CANVAS}" width="{CANVAS}" height="{CANVAS}">')
    # Apple iOS continuous rounded rect (exact bezier constants from UIBezierPath reverse engineering)
    s = CANVAS
    r = CORNER_RADIUS

    def tl(u, v):
        return (u * r, v * r)
    def tr(u, v):
        return (s - u * r, v * r)
    def br(u, v):
        return (s - u * r, s - v * r)
    def bl(u, v):
        return (u * r, s - v * r)

    def pt(p):
        return f'{p[0]:.2f},{p[1]:.2f}'

    squircle_path = (
        f'M{pt(tl(1.52866,0))} '
        f'L{pt(tr(1.52866,0))} '
        f'C{pt(tr(1.08849,0))} {pt(tr(0.86841,0))} {pt(tr(0.63149,0.07491))} '
        f'C{pt(tr(0.37282,0.16906))} {pt(tr(0.16906,0.37282))} {pt(tr(0.07491,0.63149))} '
        f'C{pt(tr(0,0.86841))} {pt(tr(0,1.08849))} {pt(tr(0,1.52866))} '
        f'L{pt(br(0,1.52866))} '
        f'C{pt(br(0,1.08849))} {pt(br(0,0.86841))} {pt(br(0.07491,0.63149))} '
        f'C{pt(br(0.16906,0.37282))} {pt(br(0.37282,0.16906))} {pt(br(0.63149,0.07491))} '
        f'C{pt(br(0.86841,0))} {pt(br(1.08849,0))} {pt(br(1.52866,0))} '
        f'L{pt(bl(1.52866,0))} '
        f'C{pt(bl(1.08849,0))} {pt(bl(0.86841,0))} {pt(bl(0.63149,0.07491))} '
        f'C{pt(bl(0.37282,0.16906))} {pt(bl(0.16906,0.37282))} {pt(bl(0.07491,0.63149))} '
        f'C{pt(bl(0,0.86841))} {pt(bl(0,1.08849))} {pt(bl(0,1.52866))} '
        f'L{pt(tl(0,1.52866))} '
        f'C{pt(tl(0,1.08849))} {pt(tl(0,0.86841))} {pt(tl(0.07491,0.63149))} '
        f'C{pt(tl(0.16906,0.37282))} {pt(tl(0.37282,0.16906))} {pt(tl(0.63149,0.07491))} '
        f'C{pt(tl(0.86841,0))} {pt(tl(1.08849,0))} {pt(tl(1.52866,0))} '
        f'Z'
    )
    svg.append(f'  <path d="{squircle_path}" fill="{BG_COLOR}"/>')

    # Outside text (察 with circle punched out at bitmap level)
    svg.append('  <g opacity="0.9">')
    ki = 0
    for row in sorted(outside_rows.keys()):
        for run_start, run_end in find_runs(outside_rows[row]):
            x = PADDING + run_start * cell_size
            run_width = (run_end - run_start + 1) * cell_size
            y = PADDING + row * cell_size + cell_size * 0.78
            max_chars = int(run_width / char_width)
            if max_chars < 1:
                continue
            words, ki = fill_words(max_chars, ki)
            svg.append(f'    {render_text_run(words, font_size, x, y, run_width)}')
    svg.append('  </g>')

    # Magnifying glass ring + handle
    inner_r = LENS_R - LENS_STROKE // 2
    svg.append(f'  <circle cx="{LENS_CX}" cy="{LENS_CY}" r="{inner_r}" fill="{BG_COLOR}"/>')
    svg.append(f'  <circle cx="{LENS_CX}" cy="{LENS_CY}" r="{LENS_R}" fill="none" stroke="{COLOR_LENS}" stroke-width="{LENS_STROKE}"/>')
    hx = LENS_CX + (LENS_R + HANDLE_LEN) * 0.707
    hy = LENS_CY + (LENS_R + HANDLE_LEN) * 0.707
    sx = LENS_CX + LENS_R * 0.707
    sy = LENS_CY + LENS_R * 0.707
    svg.append(f'  <line x1="{sx:.1f}" y1="{sy:.1f}" x2="{hx:.1f}" y2="{hy:.1f}" stroke="{COLOR_LENS}" stroke-width="{LENS_STROKE + 2}" stroke-linecap="round"/>')

    # Report summary inside lens
    summary_lines = [
        ("0 ERRORS", COLOR_SUGGEST),
        ("2 WARNINGS", COLOR_WARNING),
        ("3 HINTS", COLOR_HINT),
    ]
    line_h = 18
    start_y = LENS_CY - line_h
    for i, (text, color) in enumerate(summary_lines):
        ty = start_y + i * line_h
        words_svg, _ = word_to_paths_colored(text, color, 13, LENS_CX - 38, ty)
        svg.append(f'  {words_svg}')

    svg.append('</svg>')
    return "\n".join(svg)


if __name__ == "__main__":
    import sys
    output = sys.argv[1] if len(sys.argv) > 1 else "logo.svg"
    content = generate_logo()
    with open(output, "w") as f:
        f.write(content)
    print(f"Logo written to {output}")
