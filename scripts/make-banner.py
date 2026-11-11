#!/usr/bin/env python3
"""Render `ascii-art.txt` into the README banner.

Writes a dark and a light variant so the README can pick one per theme with a
<picture> element. No image library, no fonts: every mark comes from the art's
own characters.
"""

import os

from ascii_render import Canvas, draw_cells, load_rows, rounded_rect_alpha

CELL_WIDTH = 9
CELL_HEIGHT = 16
PADDING = 48
RADIUS = 26

# (fill, glyph, glyph alpha). `#` is the dark field, ` ` the lit subject.
# The fill sits far below the glyph so the characters still read as text.
DARK = {
    "#": ((0.030, 0.036, 0.046), (0.10, 0.13, 0.16), 1.0),
    "+": ((0.030, 0.12, 0.07), (0.10, 0.42, 0.25), 1.0),
    ".": ((0.045, 0.19, 0.11), (0.17, 0.68, 0.40), 1.0),
    " ": ((0.10, 0.42, 0.26), (0.52, 1.0, 0.74), 1.0),
}

LIGHT = {
    "#": ((0.972, 0.976, 0.980), (0.87, 0.88, 0.90), 1.0),
    "+": ((0.90, 0.94, 0.91), (0.56, 0.75, 0.65), 1.0),
    ".": ((0.68, 0.86, 0.76), (0.18, 0.50, 0.34), 1.0),
    " ": ((0.26, 0.58, 0.41), (0.02, 0.24, 0.14), 1.0),
}


def render(rows, tones, panel, destination):
    width = len(rows[0]) * CELL_WIDTH + PADDING * 2
    height = len(rows) * CELL_HEIGHT + PADDING * 2
    canvas = Canvas(width, height, (0.0, 0.0, 0.0, 0.0))

    inset = PADDING * 0.4
    for y in range(height):
        for x in range(width):
            alpha = rounded_rect_alpha(x, y, inset, inset, width - inset, height - inset, RADIUS)
            if alpha > 0:
                canvas.blend(x, y, panel, alpha)

    draw_cells(canvas, rows, PADDING, PADDING, CELL_WIDTH, CELL_HEIGHT, tones)
    canvas.write_png(destination)
    print("wrote", destination, f"{width}x{height}")


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    assets = os.path.join(root, "assets")
    os.makedirs(assets, exist_ok=True)
    rows = load_rows(os.path.join(root, "ascii-art.txt"))
    render(rows, DARK, (0.031, 0.035, 0.043), os.path.join(assets, "banner-dark.png"))
    render(rows, LIGHT, (0.98, 0.98, 0.99), os.path.join(assets, "banner-light.png"))


if __name__ == "__main__":
    main()
