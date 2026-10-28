#!/usr/bin/env python3
"""Deterministically rig the user-approved CRT character sheet into an 8-frame atlas.

Authoring-only dependencies: Pillow, NumPy, OpenCV. Runtime remains Rust + PNG only.
The supplied user art is the canonical source. Run in the repository root.
"""
from pathlib import Path
import argparse
import cv2
import numpy as np
from PIL import Image, ImageDraw

TILE_W, TILE_H = 196, 300
GRID_COLS = 3
WALK_FRAMES = 8
TOTAL_FRAMES = 9
SOURCE_BOX = (466, 92, 632, 350)
IDLE_BOX = (462, 422, 628, 683)
# Silhouette subparts are traced against the approved first walking pose.
# Worn gear stays on the source's original arm; no full-character mirroring.
POLYS = {
    'back_leg': [(52,142),(83,143),(81,163),(71,176),(60,189),(48,200),(38,213),(51,244),(50,255),(28,255),(18,241),(10,224),(11,211),(28,199),(45,178),(57,159)],
    'front_leg': [(73,140),(111,140),(109,158),(117,178),(126,200),(130,219),(148,222),(150,236),(135,247),(105,256),(96,257),(92,236),(87,214),(79,193),(67,177),(61,159)],
    'back_arm': [(57,84),(72,93),(62,106),(51,121),(44,137),(46,153),(43,167),(31,170),(18,164),(9,154),(14,134),(11,123),(17,110),(38,94)],
    'front_arm': [(104,89),(117,95),(120,107),(129,120),(144,125),(149,139),(141,153),(128,164),(116,164),(112,154),(114,145),(107,129),(103,114)],
    'core': [(41,7),(111,4),(126,20),(119,67),(98,81),(108,87),(114,104),(112,123),(105,139),(111,150),(103,159),(49,158),(44,148),(44,126),(46,109),(50,95),(54,82),(43,72),(39,54)],
}
STRIDES = [0, 13, 32, 54, 54, 32, 13, 0]
ARMS = [0, 5, 12, 20, 20, 12, 5, 0]
BOBS = [0, -2, -4, -2, 0, -2, -4, -2]


def isolate(src, box, canvas=(160, 260), bottom=254):
    """Extract a connected figure silhouette, preserving black interior linework."""
    x0, y0, x1, y1 = box
    crop = src[y0:y1, x0:x1].copy()
    bright = crop[:,:,1].astype(np.int16)
    fg = ((bright > 63) & (bright > 1.6 * crop[:,:,0])).astype(np.uint8) * 255
    fg = cv2.morphologyEx(fg, cv2.MORPH_CLOSE, np.ones((5,5), np.uint8))
    contours, _ = cv2.findContours(fg, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        raise ValueError('No character contour in source art')
    mask = np.zeros_like(fg)
    cv2.drawContours(mask, [max(contours, key=cv2.contourArea)], -1, 255, -1)
    mask = cv2.dilate(mask, np.ones((3,3), np.uint8), iterations=1)
    mask = cv2.GaussianBlur(mask, (3,3), .55)
    crop[:,:,0] = np.maximum(crop[:,:,0], 3)
    crop[:,:,1] = np.maximum(crop[:,:,1], 10)
    crop[:,:,2] = np.maximum(crop[:,:,2], 4)
    rgba = np.dstack((crop, mask))
    ys, xs = np.where(mask > 150)
    cut = Image.fromarray(rgba[ys.min():ys.max()+1, xs.min():xs.max()+1], 'RGBA')
    w, h = cut.size
    scale = min((canvas[0]-12)/w, (canvas[1]-12)/h, 1.0)
    if scale < 1:
        cut = cut.resize((round(w*scale), round(h*scale)), Image.Resampling.LANCZOS)
    tile = Image.new('RGBA', canvas, (0,0,0,0))
    tile.alpha_composite(cut, ((canvas[0]-cut.width)//2, bottom-cut.height))
    return tile


def layers_from_walk(walk):
    parts = {}
    raw = np.asarray(walk).copy()
    for label, polygon in POLYS.items():
        mask = Image.new('L', walk.size, 0)
        ImageDraw.Draw(mask).polygon(polygon, fill=255)
        pixels = raw.copy()
        pixels[:,:,3] = (raw[:,:,3].astype(np.float32) * (np.asarray(mask)/255)).astype(np.uint8)
        parts[label] = Image.fromarray(pixels, 'RGBA')
    return parts


def shifted(layer, degrees=0, pivot=(0,0), bob=0):
    m = cv2.getRotationMatrix2D(pivot, degrees, 1.0)
    m[0,2] += 18
    m[1,2] += 16+bob
    image = cv2.warpAffine(np.asarray(layer), m, (TILE_W, TILE_H),
        flags=cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=(0,0,0,0))
    return Image.fromarray(image, 'RGBA')


def build_frames(source):
    art = np.asarray(source.convert('RGB'))
    walk = isolate(art, SOURCE_BOX)
    pieces = layers_from_walk(walk)
    rendered = []
    for idx in range(WALK_FRAMES):
        stride, arm, bob = STRIDES[idx], ARMS[idx], BOBS[idx]
        frame = Image.new('RGBA', (TILE_W,TILE_H))
        frame.alpha_composite(shifted(pieces['back_leg'], stride, (69,152), bob))
        frame.alpha_composite(shifted(pieces['front_leg'], -stride, (86,153), bob))
        frame.alpha_composite(shifted(pieces['back_arm'], -arm, (56,88), bob))
        frame.alpha_composite(shifted(pieces['core'], bob=bob))
        # The original character sheet has the rod on wearer's front-left
        # (viewer-right) only. Keep the source art and its arm positions, never mirror.
        frame.alpha_composite(shifted(pieces['front_arm'], arm, (109,100), bob))
        rendered.append(frame)
    # The approved upright/thumbs-up action contains the canonical front-left rod.
    idle = isolate(art, IDLE_BOX, canvas=(TILE_W,TILE_H), bottom=293)
    rendered.append(idle)
    return rendered


def generate(source_path, atlas_path, gif_path=None):
    src = Image.open(source_path)
    assert src.size == (1536,1024), src.size
    frames = build_frames(src)
    atlas = Image.new('RGBA', (GRID_COLS*TILE_W, GRID_COLS*TILE_H), (0,0,0,0))
    for idx,frame in enumerate(frames):
        x = (idx % GRID_COLS)*TILE_W
        y = (idx // GRID_COLS)*TILE_H
        atlas.alpha_composite(frame, (x,y))
    atlas.save(atlas_path, optimize=True)
    if gif_path is not None:
        backdrops=[]
        for frame in frames[:WALK_FRAMES]:
            canvas = Image.new('RGBA', (TILE_W,TILE_H), (6,18,11,255))
            canvas.alpha_composite(frame)
            backdrops.append(canvas.convert('P', palette=Image.Palette.ADAPTIVE, colors=64))
        backdrops[0].save(gif_path, save_all=True, append_images=backdrops[1: ], duration=100, loop=0, disposal=2, optimize=True)
    return atlas,frames


if __name__ == '__main__':
    ap=argparse.ArgumentParser()
    ap.add_argument('--source',type=Path, default=Path('crates/pipboy/assets/avatar-approved-reference.png'))
    ap.add_argument('--output',type=Path, default=Path('crates/pipboy/assets/avatar-approved-atlas.png'))
    ap.add_argument('--gif',type=Path, default=None)
    args=ap.parse_args()
    args.output.parent.mkdir(parents=True,exist_ok=True)
    atlas, frames=generate(args.source,args.output,args.gif)
    print(f'Generated {atlas.width}x{atlas.height} RGBA atlas with {len(frames)} frames')
