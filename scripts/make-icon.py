#!/usr/bin/env python3
"""Render the Kilgharrah icon set without any image library.

Draws a dark rounded tile with the green monogram, then hands the 1024px
master to sips and iconutil for the sizes Tauri bundles.
"""

import math
import os
import shutil
import struct
import subprocess
import tempfile
import zlib

SIZE = 1024
ACCENT = (61, 220, 132)
TOP = (21, 27, 36)
BOTTOM = (9, 12, 16)


def smoothstep(edge0, edge1, value):
    t = max(0.0, min(1.0, (value - edge0) / (edge1 - edge0)))
    return t * t * (3.0 - 2.0 * t)


def rounded_distance(px, py, half, radius):
    dx = abs(px - SIZE / 2) - (half - radius)
    dy = abs(py - SIZE / 2) - (half - radius)
    outside = math.hypot(max(dx, 0.0), max(dy, 0.0))
    inside = min(max(dx, dy), 0.0)
    return outside + inside - radius


def segment_distance(px, py, ax, ay, bx, by):
    vx, vy = bx - ax, by - ay
    wx, wy = px - ax, py - ay
    t = max(0.0, min(1.0, (wx * vx + wy * vy) / (vx * vx + vy * vy)))
    return math.hypot(wx - t * vx, wy - t * vy)


def pixel(x, y):
    inset = 0.055 * SIZE
    half = SIZE / 2 - inset
    distance = rounded_distance(x, y, half, 0.235 * SIZE)
    alpha = 1.0 - smoothstep(-1.0, 1.0, distance)
    if alpha <= 0.0:
        return (0, 0, 0, 0)

    blend = y / SIZE
    red = int(TOP[0] + (BOTTOM[0] - TOP[0]) * blend)
    green = int(TOP[1] + (BOTTOM[1] - TOP[1]) * blend)
    blue = int(TOP[2] + (BOTTOM[2] - TOP[2]) * blend)

    scale = SIZE
    strokes = [
        (0.34, 0.265, 0.34, 0.755),
        (0.34, 0.527, 0.665, 0.265),
        (0.447, 0.44, 0.68, 0.745),
    ]
    mark = min(
        segment_distance(x, y, a * scale, b * scale, c * scale, d * scale)
        for a, b, c, d in strokes
    )
    ink = 1.0 - smoothstep(0.036 * SIZE - 1.2, 0.036 * SIZE + 1.2, mark)
    if ink > 0.0:
        red = int(red + (ACCENT[0] - red) * ink)
        green = int(green + (ACCENT[1] - green) * ink)
        blue = int(blue + (ACCENT[2] - blue) * ink)
    return (red, green, blue, int(round(alpha * 255)))


def write_png(path, size):
    raw = bytearray()
    for y in range(size):
        raw.append(0)
        for x in range(size):
            raw.extend(pixel(x, y))
    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as handle:
        handle.write(png)


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    icons = os.path.join(root, "src-tauri", "icons")
    os.makedirs(icons, exist_ok=True)
    master = os.path.join(icons, "icon.png")
    write_png(master, SIZE)

    sizes = [(16, "16x16"), (32, "32x32"), (64, "64x64"), (128, "128x128"), (256, "256x256"), (512, "512x512"), (1024, "1024x1024")]
    for size, _ in sizes:
        subprocess.run(["sips", "-z", str(size), str(size), master, "--out", os.path.join(icons, f"{size}.png")], check=True, stdout=subprocess.DEVNULL)

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
