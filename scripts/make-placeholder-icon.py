#!/usr/bin/env python3
"""Generate a NEUTRAL 1024x1024 placeholder app icon (stdlib only, no PIL).

This intentionally does NOT use Whop's trademarked logo or colors. It draws a
simple dark rounded tile with a neutral ring glyph so the temporary icon is
obviously a placeholder. Replace it later with your own 1024x1024 PNG and run
scripts/generate-icons.sh.

Usage: python3 scripts/make-placeholder-icon.py <output.png>
"""
import math
import struct
import sys
import zlib

SIZE = 1024


def lerp(a, b, t):
    return a + (b - a) * t


def main(out_path: str) -> None:
    w = h = SIZE
    # Flat RGBA buffer, fully transparent to start.
    buf = bytearray(w * h * 4)

    margin = 96
    radius = 190  # tile corner radius
    x0, y0, x1, y1 = margin, margin, w - margin, h - margin

    cx, cy = w / 2.0, h / 2.0
    ring_outer = 280.0
    ring_inner = 215.0
    dot_r = 78.0

    def in_rounded_tile(px, py):
        if px < x0 or px > x1 or py < y0 or py > y1:
            return False
        # corner rounding
        rx = None
        ry = None
        if px < x0 + radius and py < y0 + radius:
            rx, ry = x0 + radius, y0 + radius
        elif px > x1 - radius and py < y0 + radius:
            rx, ry = x1 - radius, y0 + radius
        elif px < x0 + radius and py > y1 - radius:
            rx, ry = x0 + radius, y1 - radius
        elif px > x1 - radius and py > y1 - radius:
            rx, ry = x1 - radius, y1 - radius
        if rx is not None:
            return (px - rx) ** 2 + (py - ry) ** 2 <= radius * radius
        return True

    for y in range(h):
        t = (y - y0) / float(y1 - y0) if y1 != y0 else 0.0
        t = min(1.0, max(0.0, t))
        # vertical gradient for the tile
        tr = int(lerp(58, 28, t))
        tg = int(lerp(58, 30, t))
        tb = int(lerp(74, 42, t))
        row = y * w * 4
        for x in range(w):
            if not in_rounded_tile(x, y):
                continue
            i = row + x * 4
            # default: tile color
            r, g, b, a = tr, tg, tb, 255
            d = math.hypot(x - cx, y - cy)
            if ring_inner <= d <= ring_outer:
                r, g, b = 232, 233, 238
            elif d <= dot_r:
                r, g, b = 232, 233, 238
            buf[i] = r
            buf[i + 1] = g
            buf[i + 2] = b
            buf[i + 3] = a

    def chunk(typ, data):
        return (
            struct.pack(">I", len(data))
            + typ
            + data
            + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)
        )

    raw = bytearray()
    for y in range(h):
        raw.append(0)
        raw += buf[y * w * 4 : (y + 1) * w * 4]

    with open(out_path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        f.write(chunk(b"IEND", b""))

    print(f"wrote {out_path} ({w}x{h})")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "app-icon-source.png")
