"""Generate the transparent, three-times-scale PNGs for the physical notch wings."""

import math
import struct
import zlib
from pathlib import Path

WIDTH = 38
HEIGHT = 33
SCALE = 3
SAMPLES = 4


def chunk(name: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + name + data + struct.pack(
        ">I", zlib.crc32(name + data) & 0xFFFFFFFF
    )


def inside_left(x: float, y: float) -> bool:
    if x < 10 and y < 10:
        return math.hypot(x - 10, y - 10) <= 10
    if x < 17 and y > HEIGHT - 17:
        return math.hypot(x - 17, y - (HEIGHT - 17)) <= 17
    return True


def make(side: str) -> None:
    rows = bytearray()
    pixel_width = WIDTH * SCALE
    pixel_height = HEIGHT * SCALE
    for py in range(pixel_height):
        rows.append(0)
        for px in range(pixel_width):
            covered = 0
            for sy in range(SAMPLES):
                y = (py + (sy + 0.5) / SAMPLES) / SCALE
                for sx in range(SAMPLES):
                    x = (px + (sx + 0.5) / SAMPLES) / SCALE
                    if side == "right":
                        x = WIDTH - x
                    covered += inside_left(x, y)
            rows.extend((0, 0, 0, round(255 * covered / SAMPLES**2)))
    payload = b"\x89PNG\r\n\x1a\n"
    payload += chunk(b"IHDR", struct.pack(">IIBBBBB", pixel_width, pixel_height, 8, 6, 0, 0, 0))
    payload += chunk(b"IDAT", zlib.compress(bytes(rows), 9))
    payload += chunk(b"IEND", b"")
    Path(__file__).with_name(f"notch-shoulder-{side}.png").write_bytes(payload)


if __name__ == "__main__":
    make("left")
    make("right")
