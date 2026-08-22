#!/usr/bin/env python3
"""Generate the Isekaiyo application icon set.

Outputs (into apps/launcher/src-tauri/icons/ by default):

    icon.png          1024x1024 master (source of truth for future `tauri icon`)
    32x32.png         small taskbar/window sizes
    128x128.png
    128x128@2x.png    256x256
    icon.ico          Windows: 16/24/32/48/64 BMP entries + 256 PNG entry
    icon.icns         macOS: ic10 (1024) entry

Design: AMOLED-black rounded square, five-petal sakura bloom with a pink
gradient — the launcher's visual identity at every size.

Usage:
    python3 scripts/generate-icons.py [output-dir]

The script is intentionally dependency-free (pure stdlib) so any contributor
can regenerate the assets with nothing but Python 3.8+. Re-run it after
changing the drawing code and commit the results.
"""

from __future__ import annotations

import math
import struct
import sys
import zlib
from pathlib import Path

DEFAULT_OUT = Path(__file__).resolve().parent.parent / "apps" / "launcher" / "src-tauri" / "icons"

# Render at this resolution first, then box-downsample to every target size.
MASTER = 1024
SUPERSAMPLE = 2  # rendered internally at MASTER*SUPERSAMPLE for anti-aliasing


def _smoothstep(edge0: float, edge1: float, x: float) -> float:
    if x <= edge0:
        return 0.0
    if x >= edge1:
        return 1.0
    t = (x - edge0) / (edge1 - edge0)
    return t * t * (3.0 - 2.0 * t)


def _mix(a: tuple[int, int, int], b: tuple[int, int, int], t: float) -> tuple[float, float, float]:
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))  # type: ignore[return-value]


def _rounded_square_sd(u: float, v: float, half: float, radius: float) -> float:
    """Signed distance to a centered rounded square in unit UV space."""
    dx = abs(u - 0.5) - (half - radius)
    dy = abs(v - 0.5) - (half - radius)
    ox, oy = max(dx, 0.0), max(dy, 0.0)
    return math.hypot(ox, oy) + min(max(dx, dy), 0.0) - radius


def sample(u: float, v: float) -> tuple[float, float, float, float]:
    """RGBA (0..255 floats) for the master icon at normalized coords."""
    # --- rounded-square plate -------------------------------------------
    sd = _rounded_square_sd(u, v, half=0.47, radius=0.21)
    aa = 1.0 / MASTER  # ~1px feather in UV space
    alpha = _smoothstep(-aa, aa, -sd)
    if alpha <= 0.0:
        return (0.0, 0.0, 0.0, 0.0)

    # Near-black base with a very subtle top-left lift (AMOLED-oriented).
    bg_top = (18, 18, 24)
    bg_bot = (4, 4, 7)
    r, g, b = _mix(bg_bot, bg_top, (1.0 - v) * 0.9)

    # --- sakura bloom ----------------------------------------------------
    cx, cy = 0.5, 0.52
    petal_dist = 0.165
    petal_r = 0.150
    dxp = u - cx
    dyp = v - cy
    ang = math.atan2(dyp, dxp)

    d_min = 1e9
    for i in range(5):
        theta = -math.pi / 2.0 + i * (2.0 * math.pi / 5.0)
        px = cx + petal_dist * math.cos(theta)
        py = cy + petal_dist * math.sin(theta)
        d = math.hypot(u - px, v - py) - petal_r
        d_min = min(d_min, d)
    petal_a = _smoothstep(aa, -aa, d_min)

    petal_hi = (255, 133, 176)  # #FF85B0
    petal_lo = (232, 61, 134)   # #E83D86
    pr, pg, pb = _mix(petal_lo, petal_hi, _smoothstep(0.15, 0.95, cy - v + 0.35))

    r = r * (1.0 - petal_a) + pr * petal_a
    g = g * (1.0 - petal_a) + pg * petal_a
    b = b * (1.0 - petal_a) + pb * petal_a

    # --- center disc ------------------------------------------------------
    dc = math.hypot(dxp, dyp) - 0.082
    core_a = _smoothstep(aa, -aa, dc)
    core = (92, 16, 48)  # deep plum
    r = r * (1.0 - core_a) + core[0] * core_a
    g = g * (1.0 - core_a) + core[1] * core_a
    b = b * (1.0 - core_a) + core[2] * core_a

    return (r, g, b, 255.0 * alpha)


def render_master() -> list[list[tuple[float, float, float, float]]]:
    n = MASTER * SUPERSAMPLE
    step = 1.0 / n
    s2 = SUPERSAMPLE * SUPERSAMPLE
    out: list[list[tuple[float, float, float, float]]] = []
    for my in range(MASTER):
        new_row = []
        for mx in range(MASTER):
            ar = ag = ab = aa_ = 0.0
            for sy in range(SUPERSAMPLE):
                fy = (my * SUPERSAMPLE + sy + 0.5) * step
                for sx in range(SUPERSAMPLE):
                    fx = (mx * SUPERSAMPLE + sx + 0.5) * step
                    c = sample(fx, fy)
                    ar += c[0]
                    ag += c[1]
                    ab += c[2]
                    aa_ += c[3]
            new_row.append((ar / s2, ag / s2, ab / s2, aa_ / s2))
        out.append(new_row)
    return out


def downsample(
    src: list[list[tuple[float, float, float, float]]], size: int
) -> bytes:
    """Area-average the MASTER image down to `size` -> raw RGBA bytes."""
    scale = MASTER / size
    rows: list[bytes] = []
    for ty in range(size):
        sy0 = int(ty * scale)
        sy1 = max(sy0 + 1, min(MASTER, int((ty + 1) * scale)))
        row = bytearray()
        for tx in range(size):
            sx0 = int(tx * scale)
            sx1 = max(sx0 + 1, min(MASTER, int((tx + 1) * scale)))
            n = 0
            ar = ag = ab = aa = 0.0
            for yy in range(sy0, sy1):
                for xx in range(sx0, sx1):
                    c = src[yy][xx]
                    ar += c[0]
                    ag += c[1]
                    ab += c[2]
                    aa += c[3]
                    n += 1
            row += bytes(
                (round(ar / n), round(ag / n), round(ab / n), round(aa / n))
            )
        rows.append(bytes(row))
    return b"".join(rows)


# --------------------------------------------------------------------------
# Encoders (PNG / ICO / ICNS) — minimal, standard-compliant, stdlib only.
# --------------------------------------------------------------------------

def _png_chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def encode_png(w: int, h: int, rgba: bytes) -> bytes:
    raw = b"".join(b"\x00" + rgba[y * w * 4 : (y + 1) * w * 4] for y in range(h))
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
        + _png_chunk(b"IDAT", zlib.compress(raw, 9))
        + _png_chunk(b"IEND", b"")
    )


def _bmp_entry(size: int, rgba: bytes) -> bytes:
    """32bpp BMP (DIB) entry for ICO files, size <= 64."""
    w = h = size
    header = struct.pack(
        "<IiiHHIIiiII",
        40,           # biSize
        w,
        h * 2,        # XOR + AND masks share biHeight
        1,            # planes
        32,           # bpp
        0,            # BI_RGB
        w * h * 4 + ((w + 7) // 8 + 3) // 4 * h,
        0, 0, 0, 0,
    )
    xor = bytearray(w * h * 4)
    for y in range(h):
        src_row = rgba[(h - 1 - y) * w * 4 : (h - y) * w * 4]  # bottom-up
        for x in range(w):
            o = x * 4
            xor[(y * w + x) * 4 : (y * w + x) * 4 + 4] = bytes(
                (src_row[o + 2], src_row[o + 1], src_row[o], src_row[o + 3])
            )  # BGRA
    stride = ((w + 7) // 8 + 3) // 4 * 4
    and_mask = bytearray(stride * h)  # fully opaque: all zero bits
    return header + bytes(xor) + bytes(and_mask)


def encode_ico(images: dict[int, bytes]) -> bytes:
    """images: size -> raw RGBA. Sizes <=64 stored as BMP, 256 as PNG."""
    entries: list[tuple[int, bytes, bool]] = []  # (size, blob, is_png)
    for size in sorted(images):
        if size <= 64:
            entries.append((size, _bmp_entry(size, images[size]), False))
        elif size == 256:
            entries.append((size, encode_png(size, size, images[size]), True))
        else:
            raise ValueError(f"unsupported ICO size {size}")

    count = len(entries)
    offset = 6 + 16 * count
    directory = b""
    body = b""
    for size, blob, is_png in entries:
        dim = 0 if size == 256 else size  # 256 is encoded as 0
        directory += struct.pack(
            "<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset
        )
        body += blob
        offset += len(blob)
    return struct.pack("<HHH", 0, 1, count) + directory + body


def encode_icns(png_master: bytes) -> bytes:
    chunk = b"ic10" + struct.pack(">I", len(png_master) + 4) + png_master
    return b"icns" + struct.pack(">I", len(chunk) + 8) + chunk


def main() -> None:
    out_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Rendering {MASTER}x{MASTER} master (supersample {SUPERSAMPLE}x)...", flush=True)
    master = render_master()

    targets = {
        "icon.png": MASTER,
        "128x128@2x.png": 256,
        "128x128.png": 128,
        "32x32.png": 32,
    }
    rendered: dict[int, bytes] = {}
    for name, size in sorted(targets.items(), key=lambda kv: kv[1]):
        print(f"  downsampling {size}px...", flush=True)
        rendered[size] = downsample(master, size)

    for name, size in targets.items():
        (out_dir / name).write_bytes(encode_png(size, size, rendered[size]))
        print(f"  wrote {name}", flush=True)

    print("Building icon.ico...", flush=True)
    ico_sizes = [16, 24, 32, 48, 64, 256]
    ico_imgs = {}
    for s in ico_sizes:
        if s not in rendered:
            rendered[s] = downsample(master, s)
        ico_imgs[s] = rendered[s]
    (out_dir / "icon.ico").write_bytes(encode_ico(ico_imgs))
    print("  wrote icon.ico", flush=True)

    print("Building icon.icns...", flush=True)
    (out_dir / "icon.icns").write_bytes(encode_icns(encode_png(MASTER, MASTER, rendered[MASTER])))
    print("  wrote icon.icns", flush=True)

    print(f"Done -> {out_dir}")


if __name__ == "__main__":
    main()
