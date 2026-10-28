#!/usr/bin/env python3
"""Offline stdlib-only checks for user-approved CRT avatar sprite assets."""
from pathlib import Path
import struct
import zlib
from collections import deque

ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / 'crates/pipboy/assets'


def png_properties(path):
    data=path.read_bytes()
    assert data.startswith(b'\x89PNG\r\n\x1a\n'), path
    cursor=8
    width=height=color=None
    chunks=0
    while cursor<len(data):
        length=struct.unpack_from('>I',data,cursor)[0]
        kind=data[cursor+4:cursor+8]
        payload=data[cursor+8:cursor+8+length]
        saved_crc=struct.unpack_from('>I',data,cursor+8+length)[0]
        calculated=zlib.crc32(kind+payload)&0xffffffff
        assert saved_crc==calculated,(path,kind)
        if kind==b'IHDR':
            width,height,depth,color=struct.unpack_from('>IIBB',payload)
            assert depth==8
        chunks+=1
        cursor+=12+length
        if kind==b'IEND': break
    assert chunks>=3 and cursor==len(data)
    return width,height,color


def png_rgba_alpha(path):
    """Read PNG alpha channels with only stdlib; no authoring-time OpenCV in CI."""
    data = path.read_bytes()
    assert data.startswith(b'\x89PNG\r\n\x1a\n')
    cursor = 8
    compressed = bytearray()
    width = height = 0
    while cursor < len(data):
        length = struct.unpack_from('>I', data, cursor)[0]
        kind = data[cursor + 4:cursor + 8]
        payload = data[cursor + 8:cursor + 8 + length]
        cursor += 12 + length
        if kind == b'IHDR':
            width, height, bit_depth, color = struct.unpack_from('>IIBB', payload)
            assert bit_depth == 8 and color == 6, 'RGBA sprite atlas required'
        elif kind == b'IDAT':
            compressed.extend(payload)
        elif kind == b'IEND':
            break
    assert width == 588 and height == 900
    decompressed = zlib.decompress(compressed)
    stride = width * 4
    prev = bytearray(stride)
    alpha_rows = []
    pos = 0
    for _ in range(height):
        filter_kind = decompressed[pos]
        pos += 1
        row = bytearray(decompressed[pos:pos + stride])
        pos += stride
        assert len(row) == stride
        for j in range(stride):
            a = row[j - 4] if j >= 4 else 0
            b = prev[j]
            c = prev[j - 4] if j >= 4 else 0
            if filter_kind == 1:  # Sub
                row[j] = (row[j] + a) & 255
            elif filter_kind == 2:  # Up
                row[j] = (row[j] + b) & 255
            elif filter_kind == 3:  # Average
                row[j] = (row[j] + ((a + b) >> 1)) & 255
            elif filter_kind == 4:  # Paeth
                p = a + b - c
                da, db, dc = abs(p - a), abs(p - b), abs(p - c)
                predictor = a if da <= db and da <= dc else b if db <= dc else c
                row[j] = (row[j] + predictor) & 255
            else:
                assert filter_kind == 0, f'unsupported PNG filter: {filter_kind}'
        alpha_rows.append(row[3::4])
        prev = row
    assert pos == len(decompressed)
    return alpha_rows


def walk_island_areas(alpha_rows, frame):
    """Every walk frame must remain one figure, with no stray pixel island."""
    tile_w, tile_h = 196, 300
    x0 = (frame % 3) * tile_w
    y0 = (frame // 3) * tile_h
    pixels = bytearray(tile_w * tile_h)
    for y in range(tile_h):
        pixels[y * tile_w:(y + 1) * tile_w] = bytearray(
            1 if a >= 8 else 0 for a in alpha_rows[y0 + y][x0:x0 + tile_w]
        )
    areas = []
    for seed, opaque in enumerate(pixels):
        if opaque != 1:
            continue
        areas.append(0)
        queue = deque([seed])
        pixels[seed] = 2
        while queue:
            index = queue.popleft()
            areas[-1] += 1
            x, y = index % tile_w, index // tile_w
            for dy in (-1, 0, 1):
                ny = y + dy
                if not 0 <= ny < tile_h:
                    continue
                for dx in (-1, 0, 1):
                    nx = x + dx
                    if dx == dy == 0 or not 0 <= nx < tile_w:
                        continue
                    neighbor = ny * tile_w + nx
                    if pixels[neighbor] == 1:
                        pixels[neighbor] = 2
                        queue.append(neighbor)
    return sorted(areas, reverse=True)


def assert_no_walk_hip_leaks():
    alpha_rows = png_rgba_alpha(ASSET / 'avatar-approved-atlas.png')
    for frame in range(8):
        areas = walk_island_areas(alpha_rows, frame)
        assert areas and areas[0] > 10_000, f'walk frame {frame}: missing operator silhouette'
        assert not any(size >= 8 for size in areas[1:]), (
            f'walk frame {frame}: detached sprite island(s) {areas[1:]} — '
            'check moving left-hip pixels in the leg cutout'
        )


def validate():
    assert png_properties(ASSET/'avatar-approved-reference.png')[:2]==(1536,1024)
    assert png_properties(ASSET/'avatar-approved-atlas.png')==(588,900,6)
    assert_no_walk_hip_leaks()
    rust=(ROOT/'crates/pipboy/src/avatar.rs').read_text()
    assert 'include_bytes!("../assets/avatar-approved-atlas.png")' in rust
    assert 'ROD_PANEL: UniformPanel = UniformPanel::FrontLeft' in rust
    assert 'WALK_FRAMES: u32 = 8' in rust and 'IDLE_FRAME: u32 = 8' in rust
    assert 'avatar-operator.svg' not in rust
    js=(ASSET/'avatar-preview.js').read_text()
    html=(ASSET/'avatar-preview.html').read_text()
    css=(ASSET/'avatar-preview.css').read_text()
    assert 'avatar-approved-atlas.png' in js
    assert 'prefers-reduced-motion: reduce' in js and css
    assert '/assets/avatar-preview.js' in html and '/assets/avatar-preview.css' in html
    assert '<script>' not in html and '<style>' not in html
    assert 'https://' not in js and 'fetch(' not in js and 'eval(' not in js
    assert 'frameIndex' in js and 'requestAnimationFrame' in js
    assert 'front-left' in html
    print('PASS: source-derived CRT avatar atlas, PNG CRCs, hip-island regression, frame contract and reduced motion')

if __name__=='__main__': validate()