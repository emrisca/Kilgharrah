#!/usr/bin/env python3
"""Render the Kilgharrah icon set from `ascii-art.txt`, with no image library.

The dragon from the ASCII art is rebuilt as a green mosaic: every art cell
becomes a lit tile, so the silhouette survives at small sizes while the
character grid still shows up close. The 1024px master is then handed to sips
and iconutil for the sizes Tauri bundles.
"""

import os
import shutil
import struct
import subprocess
import tempfile

from ascii_render import Canvas, draw_glyph, load_rows, rounded_rect_alpha, smoothstep

SIZE = 1024
INSET = 0.055 * SIZE
RADIUS = 0.235 * SIZE
TOP = (0.082, 0.106, 0.141)
BOTTOM = (0.035, 0.047, 0.063)
GLOW = (0.02, 0.10, 0.055)

# Crop around the dragon and its ledge so the mark stays centred and square.
CROP_ROWS = (0, 28)
CROP_COLS = (20, 82)
ART_BOX = 0.84 * SIZE
CELL_ASPECT = 0.56  # character cell width over height
CELL_GAP = 0.9
GLYPH_ALPHA = 0.25


def tile_color(x, y):
    blend = y / SIZE
    color = tuple(TOP[i] + (BOTTOM[i] - TOP[i]) * blend for i in range(3))
    glow = smoothstep(0.62 * SIZE, 0.05 * SIZE, abs(x - SIZE / 2) + abs(y - SIZE / 2))
    return tuple(color[i] + GLOW[i] * glow for i in range(3))


def tone(char):
    """Lit brightness for one art character, with `#` left as empty space."""

    if char == "+":
        return 0.0
    if char == ".":
        return 0.5
    return 1.0


def build_master(path):
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rows = load_rows(os.path.join(root, "ascii-art.txt"))
    grid = [row[CROP_COLS[0]:CROP_COLS[1]] for row in rows[CROP_ROWS[0]:CROP_ROWS[1]]]
    columns, lines = len(grid[0]), len(grid)

    cell_w = ART_BOX / columns
    cell_h = cell_w / CELL_ASPECT
    if lines * cell_h > ART_BOX:
        cell_h = ART_BOX / lines
        cell_w = cell_h * CELL_ASPECT
    origin_x = (SIZE - columns * cell_w) / 2
    origin_y = (SIZE - lines * cell_h) / 2

    canvas = Canvas(SIZE, SIZE, (0.0, 0.0, 0.0, 0.0))
    for y in range(SIZE):
        for x in range(SIZE):
            alpha = rounded_rect_alpha(x, y, INSET, INSET, SIZE - INSET, SIZE - INSET, RADIUS)
            if alpha > 0.0:
                canvas.blend(x, y, tile_color(x, y), alpha)

    for line_index, line in enumerate(grid):
        for column_index, char in enumerate(line):
            if char == "#":
                continue
            level = tone(char)
            fill = (0.07 + 0.11 * level, 0.36 + 0.36 * level, 0.22 + 0.22 * level)
            left = origin_x + column_index * cell_w + CELL_GAP
            top = origin_y + line_index * cell_h + CELL_GAP
            right = left + cell_w - CELL_GAP * 2
            bottom = top + cell_h - CELL_GAP * 2
            for y in range(int(top), int(bottom)):
                for x in range(int(left), int(right)):
                    canvas.blend(x, y, fill, 1.0)
            draw_glyph(
                canvas,
                origin_x + column_index * cell_w,
                origin_y + line_index * cell_h,
                cell_w,
                cell_h,
                char,
                (0.02, 0.14, 0.08),
                GLYPH_ALPHA,
            )

    canvas.write_png(path)


def build_mark(light, path):
    """A small solid-silhouette mark for the app's top bar.

    The mosaic is dropped here: at 20 CSS px the gaps and glyphs turn to noise,
    so the dragon is a single tone on a tile that matches the theme.
    """

    size = 128
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rows = load_rows(os.path.join(root, "ascii-art.txt"))
    grid = [row[CROP_COLS[0]:CROP_COLS[1]] for row in rows[CROP_ROWS[0]:CROP_ROWS[1]]]
    columns, lines = len(grid[0]), len(grid)

    cell_w = 0.86 * size / columns
    cell_h = cell_w / CELL_ASPECT
    if lines * cell_h > 0.86 * size:
        cell_h = 0.86 * size / lines
        cell_w = cell_h * CELL_ASPECT
    origin_x = (size - columns * cell_w) / 2
    origin_y = (size - lines * cell_h) / 2

    tile = (0.945, 0.952, 0.963) if light else (0.063, 0.079, 0.102)
    ink = (0.055, 0.475, 0.290) if light else (0.240, 0.860, 0.520)
    radius = 0.235 * size

    canvas = Canvas(size, size, (0.0, 0.0, 0.0, 0.0))
    for y in range(size):
        for x in range(size):
            alpha = rounded_rect_alpha(x, y, 0.5, 0.5, size - 0.5, size - 0.5, radius)
            if alpha > 0.0:
                canvas.blend(x, y, tile, alpha)

    for line_index, line in enumerate(grid):
        for column_index, char in enumerate(line):
            if char == "#":
                continue
            left = origin_x + column_index * cell_w
            top = origin_y + line_index * cell_h
            for y in range(int(top), int(top + cell_h + 1)):
                for x in range(int(left), int(left + cell_w + 1)):
                    canvas.blend(x, y, ink, 1.0)

    canvas.write_png(path)


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    icons = os.path.join(root, "src-tauri", "icons")
    marks = os.path.join(root, "src", "assets")
    os.makedirs(marks, exist_ok=True)
    build_mark(False, os.path.join(marks, "mark-dark.png"))
    build_mark(True, os.path.join(marks, "mark-light.png"))
    os.makedirs(icons, exist_ok=True)
    master = os.path.join(icons, "icon.png")
    build_master(master)

    sizes = [(16, "16x16"), (32, "32x32"), (64, "64x64"), (128, "128x128"), (256, "256x256"), (512, "512x512"), (1024, "1024x1024")]
    for size, _ in sizes:
        subprocess.run(
            ["sips", "-z", str(size), str(size), master, "--out", os.path.join(icons, f"{size}.png")],
            check=True,
            stdout=subprocess.DEVNULL,
        )

    # Names Tauri's bundle config expects.
    shutil.copyfile(os.path.join(icons, "32.png"), os.path.join(icons, "32x32.png"))
    shutil.copyfile(os.path.join(icons, "128.png"), os.path.join(icons, "128x128.png"))
    shutil.copyfile(os.path.join(icons, "256.png"), os.path.join(icons, "128x128@2x.png"))
    shutil.copyfile(os.path.join(icons, "512.png"), os.path.join(icons, "512x512.png"))

    iconset = tempfile.mkdtemp(suffix=".iconset")
    mapping = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }
    for name, size in mapping.items():
        shutil.copyfile(os.path.join(icons, f"{size}.png"), os.path.join(iconset, name))
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", os.path.join(icons, "icon.icns")], check=True)
    shutil.rmtree(iconset)

    # A single-image ICO is enough for the Windows bundle.
    png = open(os.path.join(icons, "256.png"), "rb").read()
    ico = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22) + png
    with open(os.path.join(icons, "icon.ico"), "wb") as handle:
        handle.write(ico)
    print("icons written to", icons)


if __name__ == "__main__":
    main()
