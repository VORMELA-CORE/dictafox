#!/usr/bin/env python3
"""Generate DictaFox application icons without external graphics tooling.

The mark is an original geometric fox head: two ears, a tapered muzzle and a
lime accent on charcoal. Rendering uses supersampling for clean edges.
"""

import os
import struct
import zlib

BG = (14, 16, 15, 255)
ACCENT = (183, 240, 106, 255)
ACCENT_DARK = (126, 176, 63, 255)
FACE = (10, 12, 11, 255)
CREAM = (236, 250, 219, 255)


def polygon(points):
    return [(x, y) for x, y in points]


EARS = [
    polygon([(0.24, 0.36), (0.19, 0.10), (0.43, 0.25)]),
    polygon([(0.76, 0.36), (0.81, 0.10), (0.57, 0.25)]),
]
HEAD = polygon([
    (0.50, 0.17), (0.73, 0.30), (0.72, 0.55), (0.50, 0.86),
    (0.28, 0.55), (0.27, 0.30),
])
MUZZLE = polygon([
    (0.50, 0.52), (0.61, 0.62), (0.50, 0.80), (0.39, 0.62),
])


def inside(x, y, points):
    inside_flag = False
    j = len(points) - 1
    for i in range(len(points)):
        xi, yi = points[i]
        xj, yj = points[j]
        if ((yi > y) != (yj > y)) and (
            x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi
        ):
            inside_flag = not inside_flag
        j = i
    return inside_flag


def rounded_rect(x, y, radius):
    if x < radius and y < radius:
        return (x - radius) ** 2 + (y - radius) ** 2 <= radius * radius
    if x > 1 - radius and y < radius:
        return (x - (1 - radius)) ** 2 + (y - radius) ** 2 <= radius * radius
    if x < radius and y > 1 - radius:
        return (x - radius) ** 2 + (y - (1 - radius)) ** 2 <= radius * radius
    if x > 1 - radius and y > 1 - radius:
        return (x - (1 - radius)) ** 2 + (y - (1 - radius)) ** 2 <= radius * radius
    return True


def sample(x, y):
    if not rounded_rect(x, y, 0.22):
        return (0, 0, 0, 0)
    if any(inside(x, y, ear) for ear in EARS):
        if inside(x, y, MUZZLE):
            return FACE
        return ACCENT_DARK
    if inside(x, y, HEAD):
        if inside(x, y, MUZZLE):
            return CREAM
        # eyes
        for eye_x in (0.415, 0.585):
            if (x - eye_x) ** 2 + (y - 0.44) ** 2 <= 0.036 ** 2:
                return FACE
        return ACCENT
    return BG


def render(size):
    samples = 3
    pixels = bytearray()
    for py in range(size):
        for px in range(size):
            r = g = b = a = 0
            for sy in range(samples):
                for sx in range(samples):
                    x = (px + (sx + 0.5) / samples) / size
                    y = (py + (sy + 0.5) / samples) / size
                    cr, cg, cb, ca = sample(x, y)
                    r += cr * ca
                    g += cg * ca
                    b += cb * ca
                    a += ca
            if a == 0:
                pixels.extend((0, 0, 0, 0))
            else:
                pixels.extend((r // a, g // a, b // a, a // (samples * samples)))
    return bytes(pixels)


def png(size, pixels):
    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    raw = b"".join(
        b"\x00" + pixels[row * size * 4 : (row + 1) * size * 4] for row in range(size)
    )
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def main():
    root = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src-tauri", "icons")
    os.makedirs(root, exist_ok=True)
    for size, name in [
        (32, "32x32.png"),
        (128, "128x128.png"),
        (256, "128x128@2x.png"),
        (512, "icon.png"),
    ]:
        data = png(size, render(size))
        with open(os.path.join(root, name), "wb") as handle:
            handle.write(data)
        print(f"wrote {name} ({size}x{size}, {len(data)} bytes)")


if __name__ == "__main__":
    main()
