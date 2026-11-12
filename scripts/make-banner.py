#!/usr/bin/env python3
"""Render `ascii-art.txt` into the README banner.

Writes a dark and a light variant so the README can pick one per theme with a
<picture> element. No image library, no fonts: every mark comes from the art's
own characters. Characters are drawn in grey on the bare panel, the same way
the app icon draws them, so the banner and the icon stay one family. The art
sits at roughly two thirds of the canvas width so the dragon has room to
breathe inside a wide banner.
"""

import os

from ascii_render import Canvas, draw_cells, load_rows, rounded_rect_alpha

CANVAS = (1280, 520)
ART_WIDTH = 0.66
RADIUS = 24
CELL_ASPECT = 0.56

# (fill, glyph, glyph alpha). No fills: like the app icon, the dragon is
# made of grey characters on the bare panel rather than solid tiles.
DARK = {
    "#": (None, (0.16, 0.165, 0.175), 1.0),
    "+": (None, (0.55, 0.56, 0.58), 1.0),
    ".": (None, (0.86, 0.87, 0.89), 1.0),
    " ": (None, (0.98, 0.985, 0.99), 1.0),
}

LIGHT = {
    "#": (None, (0.925, 0.930, 0.935), 1.0),
    "+": (None, (0.62, 0.63, 0.64), 1.0),
    ".": (None, (0.34, 0.35, 0.36), 1.0),
    " ": (None, (0.10, 0.105, 0.11), 1.0),
}


def layout(rows):
    width, height = CANVAS
    columns, lines = len(rows[0]), len(rows)
    cell_w = ART_WIDTH * width / columns
    cell_h = cell_w / CELL_ASPECT
    if lines * cell_h > height * 0.8:
        cell_h = height * 0.8 / lines
        cell_w = cell_h * CELL_ASPECT
    origin_x = (width - columns * cell_w) / 2
    origin_y = (height - lines * cell_h) / 2
    return cell_w, cell_h, origin_x, origin_y


def field_tones(base, panel, delta):
    """Make the art's dark field melt into the panel instead of boxing it."""

    tones = dict(base)
    tones["#"] = (panel, tuple(min(1.0, max(0.0, c + delta)) for c in panel), 0.6)
    return tones


def render(rows, base, panel, destination):
    width, height = CANVAS
    canvas = Canvas(width, height, (0.0, 0.0, 0.0, 0.0))
    for y in range(height):
        for x in range(width):
            alpha = rounded_rect_alpha(x, y, 0.0, 0.0, width - 1, height - 1, RADIUS)
            if alpha > 0:
                canvas.blend(x, y, panel, alpha)

    cell_w, cell_h, origin_x, origin_y = layout(rows)
    draw_cells(canvas, rows, origin_x, origin_y, cell_w, cell_h, base, glyph_char="#")
    canvas.write_png(destination)
    print("wrote", destination, f"{width}x{height}", f"art {len(rows[0]) * cell_w:.0f}px wide")


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    assets = os.path.join(root, "assets")
    os.makedirs(assets, exist_ok=True)
    rows = load_rows(os.path.join(root, "ascii-art.txt"))
    render(rows, DARK, (0.031, 0.035, 0.043), os.path.join(assets, "banner-dark.png"))
    render(rows, LIGHT, (0.98, 0.98, 0.99), os.path.join(assets, "banner-light.png"))


if __name__ == "__main__":
    main()
