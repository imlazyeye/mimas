#!/usr/bin/env python3
"""Converts between a cart's `sprites.txt` and a png, either way.

    ./sprites.py sheet.png                writes sheet.txt
    ./sprites.py carts/pong/sprites.txt   writes carts/pong/sprites.png
    ./sprites.py sheet.png out.txt        names the output

A png's colors are matched to the nearest of the sixteen, and anything mostly transparent
becomes color 0. The png written back out is indexed with the palette, so a round trip keeps
every pixel, though the text is trimmed to whole 8x8 sprites the way the editor saves it.

This is python because mimas can't do it yet. What it would need:

- pictures as values, with the host doing the png: `Picture::load(path)`, `pic.width()`,
  `pic.pixel(pos) -> (int, int, int, int)`, `pic.set_pixel(pos, rgba)`, `pic.save(path)`. a cart
  can already read and edit sheet pixels (`gfx::sheet_pixel` / `gfx::set_sheet_pixel`), so the
  missing half is the picture on the other side of the conversion.
- the palette's rgb reachable from a script. `Color` only converts to and from its index, so
  nothing in mimas can measure which of the sixteen a png's pixel is closest to.
- binary files. `std::fs::read` is `read_to_string`, so a png can't be loaded as `[int]` either.
- somewhere to run it. carts have no fs, so this would run under `mimas run` rather than as a
  cart.
"""

import struct
import sys
import zlib
from pathlib import Path

SIZE = 128
SPRITE = 8

# Sweetie 16, the same order as core/src/screen.rs
PALETTE = [
    (0x1A, 0x1C, 0x2C), (0x5D, 0x27, 0x5D), (0xB1, 0x3E, 0x53), (0xEF, 0x7D, 0x57),
    (0xFF, 0xCD, 0x75), (0xA7, 0xF0, 0x70), (0x38, 0xB7, 0x64), (0x25, 0x71, 0x79),
    (0x29, 0x36, 0x6F), (0x3B, 0x5D, 0xC9), (0x41, 0xA6, 0xF6), (0x73, 0xEF, 0xF7),
    (0xF4, 0xF4, 0xF4), (0x94, 0xB0, 0xC2), (0x56, 0x6C, 0x86), (0x33, 0x3C, 0x57),
]

CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


def fail(message):
    print(f"sprites.py: {message}", file=sys.stderr)
    sys.exit(1)


def chunks(data):
    at = 8
    while at + 8 <= len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        kind = data[at + 4 : at + 8]
        yield kind, data[at + 8 : at + 8 + length]
        at += 12 + length


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def scanlines(raw, height, stride, bpp):
    lines, previous, at = [], bytearray(stride), 0
    for _ in range(height):
        kind, line = raw[at], bytearray(raw[at + 1 : at + 1 + stride])
        at += 1 + stride
        for i in range(stride):
            left = line[i - bpp] if i >= bpp else 0
            up = previous[i]
            corner = previous[i - bpp] if i >= bpp else 0
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 0xFF
            elif kind == 4:
                line[i] = (line[i] + paeth(left, up, corner)) & 0xFF
            elif kind != 0:
                fail(f"filter {kind} is not one a png may use")
        lines.append(line)
        previous = line
    return lines


def unpack(line, x, depth, channels):
    if depth == 8:
        return line[x * channels : x * channels + channels]
    if depth == 16:
        return line[x * channels * 2 : (x + 1) * channels * 2 : 2]
    per_byte = 8 // depth
    mask = (1 << depth) - 1
    out = []
    for channel in range(channels):
        index = x * channels + channel
        shift = (per_byte - 1 - index % per_byte) * depth
        out.append((line[index // per_byte] >> shift) & mask)
    return out


def nearest(r, g, b):
    best, score = 0, None
    for index, (pr, pg, pb) in enumerate(PALETTE):
        distance = (r - pr) ** 2 + (g - pg) ** 2 + (b - pb) ** 2
        if score is None or distance < score:
            best, score = index, distance
    return best


def read_png(path):
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        fail(f"{path} is not a png")
    header, palette, alphas, compressed = None, b"", b"", bytearray()
    for kind, body in chunks(data):
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"PLTE":
            palette = body
        elif kind == b"tRNS":
            alphas = body
        elif kind == b"IDAT":
            compressed += body
        elif kind == b"IEND":
            break
    if header is None:
        fail(f"{path} has no header chunk")
    width, height, depth, color, _, _, interlace = header
    if interlace:
        fail("interlaced pngs aren't read, save it without adam7")
    if color not in CHANNELS:
        fail(f"color type {color} is not one a png may use")
    if width > SIZE or height > SIZE:
        fail(f"{path} is {width}x{height}, and a sheet is {SIZE}x{SIZE} at most")
    channels = CHANNELS[color]
    stride = (width * channels * depth + 7) // 8
    raw = zlib.decompress(bytes(compressed))
    lines = scanlines(raw, height, stride, max(1, channels * depth // 8))

    sheet = [bytearray(SIZE) for _ in range(SIZE)]
    # `unpack` keeps the high byte of a 16-bit sample, so every channel is already 0..255 there
    full = 255 if depth == 16 else (1 << depth) - 1
    for y, line in enumerate(lines):
        for x in range(width):
            values = unpack(line, x, depth, channels)
            if color == 3:
                index = values[0]
                rgb = tuple(palette[index * 3 : index * 3 + 3]) or (0, 0, 0)
                alpha = alphas[index] if index < len(alphas) else 255
            else:
                scale = 255 // full
                values = [value * scale for value in values]
                rgb = tuple(values[:3]) if channels >= 3 else (values[0],) * 3
                alpha = values[-1] if color in (4, 6) else 255
            sheet[y][x] = 0 if alpha < 128 else nearest(*rgb)
    return sheet


def read_text(path):
    sheet = [bytearray(SIZE) for _ in range(SIZE)]
    for y, line in enumerate(path.read_text().splitlines()):
        if not line:
            continue
        if y >= SIZE:
            fail(f"{path} has more than {SIZE} lines")
        if len(line) > SIZE:
            fail(f"{path} line {y + 1} has more than {SIZE} digits")
        for x, digit in enumerate(line):
            if digit not in "0123456789abcdefABCDEF":
                fail(f"{path} line {y + 1}: `{digit}` is not a hex digit")
            sheet[y][x] = int(digit, 16)
    return sheet


def write_png(path, sheet):
    def chunk(kind, body):
        crc = struct.pack(">I", zlib.crc32(kind + body))
        return struct.pack(">I", len(body)) + kind + body + crc

    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 3, 0, 0, 0)
    palette = bytes(channel for rgb in PALETTE for channel in rgb)
    raw = b"".join(b"\0" + bytes(row) for row in sheet)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"PLTE", palette)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def write_text(path, sheet):
    # the sprite editor saves whole 8x8 tiles and drops the black ones past the last used sprite
    used = [(x, y) for y in range(SIZE) for x in range(SIZE) if sheet[y][x]]
    if not used:
        path.write_text("")
        return

    def tiles(values):
        return min(SIZE, (max(values) // SPRITE + 1) * SPRITE)
    width = tiles(x for x, _ in used)
    height = tiles(y for _, y in used)
    rows = ("".join(f"{color:x}" for color in sheet[y][:width]) for y in range(height))
    path.write_text("".join(f"{row}\n" for row in rows))


def main(argv):
    if not 1 <= len(argv) <= 2:
        fail("usage: sprites.py <sheet.png | sprites.txt> [output]")
    source = Path(argv[0])
    if not source.is_file():
        fail(f"{source} is not a file")
    to_text = source.suffix.lower() == ".png"
    out = Path(argv[1]) if len(argv) == 2 else source.with_suffix(".txt" if to_text else ".png")
    if to_text:
        write_text(out, read_png(source))
    else:
        write_png(out, read_text(source))
    print(out)


if __name__ == "__main__":
    main(sys.argv[1:])
