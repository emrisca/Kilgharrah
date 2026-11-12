"""Turn the Kilgharrah ASCII art into PNG artwork without any image library.

The art uses four tone characters: `#` for the dark field, `+` for mid tone,
`.` for light tone, and space for the brightest areas. Both the app icon and
the README banner draw those characters as real glyphs so the artwork stays
recognisably made of text.
"""

import math
import struct
import zlib

GLYPHS = "#+. "


def load_rows(path):
    with open(path, "r", encoding="utf-8") as handle:
        rows = handle.read().split("\n")
    rows = [row for row in rows if row.strip()]
    width = max(len(row) for row in rows)
    return [row.ljust(width) for row in rows]


class Canvas:
    """A straight-alpha RGBA float buffer with glyph drawing."""

    def __init__(self, width, height, background=(0, 0, 0, 0)):
        self.width = width
        self.height = height
        self.pixels = [list(background) for _ in range(width * height)]

    def blend(self, x, y, color, alpha):
        if alpha <= 0.0 or x < 0 or y < 0 or x >= self.width or y >= self.height:
            return
        target = self.pixels[y * self.width + x]
        a = min(1.0, alpha)
        for channel in range(3):
            target[channel] = target[channel] * (1.0 - a) + color[channel] * a
        target[3] = target[3] + a * (1.0 - target[3])

    def fill_rect(self, x0, y0, x1, y1, shade):
        for y in range(max(0, int(y0)), min(self.height, int(y1))):
            for x in range(max(0, int(x0)), min(self.width, int(x1))):
                color, alpha = shade(x, y)
                self.blend(x, y, color, alpha)

    def paint(self, x, y, color, alpha):
        self.blend(int(x), int(y), color, alpha)

    def write_png(self, path, scale=1.0):
        def chunk(tag, data):
            body = tag + data
            return (
                struct.pack(">I", len(data))
                + body
                + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
            )

        raw = bytearray()
        for y in range(self.height):
            raw.append(0)
            for x in range(self.width):
                r, g, b, a = self.pixels[y * self.width + x]
                raw.extend(
                    (
                        int(round(max(0.0, min(1.0, r)) * 255)),
                        int(round(max(0.0, min(1.0, g)) * 255)),
                        int(round(max(0.0, min(1.0, b)) * 255)),
                        int(round(max(0.0, min(1.0, a)) * 255)),
                    )
                )
        header = struct.pack(">IIBBBBB", self.width, self.height, 8, 6, 0, 0, 0)
        png = (
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + chunk(b"IEND", b"")
        )
        with open(path, "wb") as handle:
            handle.write(png)


def glyph_points(cell_x, cell_y, cell_w, cell_h, char):
    """Return the strokes for one glyph as axis-aligned thick segments."""

    thickness = max(1.0, min(cell_w, cell_h) * 0.11)
    mid_x = cell_x + cell_w * 0.5
    mid_y = cell_y + cell_h * 0.5
    if char == "#":
        return [
            (cell_x + cell_w * 0.34, cell_y + cell_h * 0.1, cell_x + cell_w * 0.34, cell_y + cell_h * 0.9),
            (cell_x + cell_w * 0.66, cell_y + cell_h * 0.1, cell_x + cell_w * 0.66, cell_y + cell_h * 0.9),
            (cell_x + cell_w * 0.1, cell_y + cell_h * 0.34, cell_x + cell_w * 0.9, cell_y + cell_h * 0.34),
            (cell_x + cell_w * 0.1, cell_y + cell_h * 0.66, cell_x + cell_w * 0.9, cell_y + cell_h * 0.66),
        ], thickness
    if char == "+":
        return [
            (mid_x, cell_y + cell_h * 0.16, mid_x, cell_y + cell_h * 0.84),
            (cell_x + cell_w * 0.16, mid_y, cell_x + cell_w * 0.84, mid_y),
        ], thickness
    if char == ".":
        return [(mid_x, mid_y, mid_x, mid_y)], max(1.0, min(cell_w, cell_h) * 0.11)
    return [], thickness


_GLYPH_CACHE = {}


def glyph_mask(char, cell_w, cell_h, samples=4):
    """Coverage values for one glyph, rendered once and reused per cell."""

    key = (char, round(cell_w, 3), round(cell_h, 3), samples)
    if key in _GLYPH_CACHE:
        return _GLYPH_CACHE[key]
    strokes, thickness = glyph_points(0.0, 0.0, cell_w, cell_h, char)
    half = thickness / 2.0
    left = int(math.floor(-half)) - 1
    top = int(math.floor(-half)) - 1
    width = int(math.ceil(cell_w + half)) + 2
    height = int(math.ceil(cell_h + half)) + 2
    rows = []
    if strokes:
        step = 1.0 / samples
        for y in range(top, top + height):
            row = []
            for x in range(left, left + width):
                hits = 0
                for sy in range(samples):
                    py = y + (sy + 0.5) * step
                    for sx in range(samples):
                        px = x + (sx + 0.5) * step
                        for ax, ay, bx, by in strokes:
                            if ax == bx and ay == by:
                                if math.hypot(px - ax, py - ay) <= half:
                                    hits += 1
                                    break
                            else:
                                vx, vy = bx - ax, by - ay
                                wx, wy = px - ax, py - ay
                                length = vx * vx + vy * vy
                                t = (
                                    0.0
                                    if length == 0
                                    else max(0.0, min(1.0, (wx * vx + wy * vy) / length))
                                )
                                if math.hypot(wx - t * vx, wy - t * vy) <= half:
                                    hits += 1
                                    break
                row.append(hits / float(samples * samples))
            rows.append(row)
    mask = (left, top, width, height, rows)
    _GLYPH_CACHE[key] = mask
    return mask


def draw_glyph(canvas, cell_x, cell_y, cell_w, cell_h, char, color, alpha, samples=4):
    if alpha <= 0.0:
        return
    left, top, width, height, rows = glyph_mask(char, cell_w, cell_h, samples)
    if not rows:
        return
    for row_index, row in enumerate(rows):
        y = int(round(cell_y)) + top + row_index
        for column_index, coverage in enumerate(row):
            if coverage <= 0.0:
                continue
            x = int(round(cell_x)) + left + column_index
            canvas.blend(x, y, color, alpha * coverage)


def draw_cells(canvas, rows, origin_x, origin_y, cell_w, cell_h, tones, tint=None, glyph_char=None):
    """Paint each character as a filled tone cell with its glyph on top.

    `tones` maps a character to (fill_rgb, glyph_rgb, glyph_alpha). The fill
    carries the picture's light and the glyph keeps the text origin visible.
    Pass `glyph_char` to draw the same dense glyph in every cell, so the tone
    colour alone carries the picture the way the app icon does.
    """

    for row_index, row in enumerate(rows):
        cell_y = origin_y + row_index * cell_h
        for column_index, char in enumerate(row):
            fill, glyph, glyph_alpha = tones.get(char, tones[" "])
            if tint is not None and fill is not None:
                fill = tint(fill, column_index, row_index)
            cell_x = origin_x + column_index * cell_w
            if fill is not None:
                for y in range(int(cell_y), int(cell_y + cell_h)):
                    for x in range(int(cell_x), int(cell_x + cell_w)):
                        canvas.blend(x, y, fill, 1.0)
            if glyph is not None:
                draw_glyph(canvas, cell_x, cell_y, cell_w, cell_h, glyph_char or char, glyph, glyph_alpha)



def smoothstep(edge0, edge1, value):
    if edge1 == edge0:
        return 1.0 if value >= edge1 else 0.0
    t = max(0.0, min(1.0, (value - edge0) / (edge1 - edge0)))
    return t * t * (3.0 - 2.0 * t)


def rounded_rect_alpha(x, y, x0, y0, x1, y1, radius, softness=1.2):
    cx = min(max(x, x0 + radius), x1 - radius)
    cy = min(max(y, y0 + radius), y1 - radius)
    distance = math.hypot(x - cx, y - cy) - radius
    if x < x0 or x > x1 or y < y0 or y > y1:
        return 0.0
    return 1.0 - smoothstep(-softness, softness, distance)


def density_mask(rows, start_row=0, end_row=None, start_col=0, end_col=None):
    """Return the art's lit area as a boolean grid, with `#` as dark field."""

    end_row = len(rows) if end_row is None else end_row
    end_col = len(rows[0]) if end_col is None else end_col
    mask = []
    for row in rows[start_row:end_row]:
        line = []
        for char in row[start_col:end_col]:
            line.append(char in "+. " and char != " ")
        mask.append(line)
    return mask
