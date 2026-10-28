#!/usr/bin/env python3
"""Offline stdlib-only checks for user-approved CRT avatar sprite assets."""
from pathlib import Path
import struct
import zlib

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


def validate():
    assert png_properties(ASSET/'avatar-approved-reference.png')[:2]==(1536,1024)
    assert png_properties(ASSET/'avatar-approved-atlas.png')==(588,900,6)
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
    print('PASS: source-derived CRT avatar atlas, PNG CRCs, frame contract and reduced motion')

if __name__=='__main__': validate()
