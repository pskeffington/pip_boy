# Rust-Boy approved CRT character | sprite-atlas v2

The *source of truth* is the user-approved CRT character reference at `crates/pipboy/assets/avatar-approved-reference.png`. The former hand-drawn `avatar-operator.svg` has been discarded; do not restore it as the avatar source. This is a self-contained offline art and animation module, **not** a completed STATUS screen, motion sensor, or physiological monitor.

## Art contract

- The person is the bearded, short-haired operator pictured on the approved sheet, with phosphor green scanlines, geometric field uniform, and gear wrist terminal. Do not replace the character with a newly invented portrait.
- The uniform has four panels from the wearer's perspective: **front-right, front-left, back-left, back-right**.
- The **rod-and-serpent** must appear only on the *wearer's front-left* breast (viewer-right when front-facing). The wearer's front-right breast and both back panels must not show the rod. Never horizontally mirror a frame or decal. The wrist gear logo is a separate icon and can appear on the wrist.
- The source reference includes six near-identical walk drawings. Playing those directly would barely animate; the atlas rig uses the approved first walking figure as limb source, then pivots its actual drawn limbs through a gait. The idle frame comes from the approved upright/thumbs-up pose and retains the clear left-chest rod.

## Compiled assets and native API

- `crates/pipboy/assets/avatar-approved-reference.png`: source reference sheet, optimized indexed PNG (1536 × 1024), retained in the repo for provenance and reproducible revisions.
- `crates/pipboy/assets/avatar-approved-atlas.png`: 588 × 900 transparent RGBA PNG, **3 columns × 3 rows** of 196 × 300 sprites. Slots **0–7** are a 100 ms/frame walk over an **800 ms** loop; slot **8** is the non-animated idle pose.
- `crates/pipboy/src/avatar.rs`: deterministic `frame_at(Motion, Duration)` pure sampler and `ATLAS_PNG` bytes for native bitmap decoders. No timers, network, permissions, or heavyweight crates are introduced.
- `GET /avatar` is a standalone local animation test bench. `GET /assets/avatar-approved-atlas.png` serves the embedded PNG with the loopback dashboard's same-origin and strict CSP protections.
- `avatar-preview.html`, `avatar-preview.css` and `avatar-preview.js`: self-contained localhost preview with WALK, IDLE, PAUSE and `prefers-reduced-motion` support.

The current micro and desktop layouts are not altered. When a custom STATUS screen is built, its Rust-native renderer can decode `ATLAS_PNG` once and draw the rectangle returned by `frame_at`. Browser consumers can draw the same rectangle into a 196 × 300 canvas. No route or animation claims access to live walking, heart rate, health, identity, or GNSS data.

## Regeneration

The atlas is checked in so **runtime builds need no Python, Pillow or OpenCV**. To regenerate it from the checked-in CRT sheet, run:

```bash
python3 -m pip install pillow numpy opencv-python
python3 tools/generate_avatar_atlas.py
python3 tools/test_avatar_assets.py
```

Only use the supplied reference (or a user-approved replacement) and retain all four-panel uniform constraints. The Python authoring pipeline traces the silhouette, isolates real source limb pixels, pivots them without horizontal mirroring, and packs one idle frame. Its geometry/limb motion is an authoring choice, not a medical or physical gait model.

## Left-hip walk-cycle repair (2026-10-28)

The prior atlas could show a moving fleck to the viewer-right of the waistband (the operator's left hip) during frames 1–6. The authoring rig mistakenly included a tiny upper-right corner of the **front-leg source polygon** at x≈105–112, y≈139–144. Rotating that clipped corner about the hip detached it from the main figure, producing a 13–25 pixel floating patch. The source art did **not** contain a pouch or hip attachment.

The polygon cap is now kept tucked beneath the waist: its upper-right point was shifted from `(111, 140)` to `(104, 140)` with a transition at `(104, 146)`. This removes the stray fleck while preserving the approved face, beard, uniform, wrist terminal and front-left-only rod. The 3×3 atlas is regenerated; the Rust and browser frame-sampling APIs remain unchanged, as do the eight 100 ms walking frames.

The stdlib-only asset test now decodes PNG alpha and checks that each walking frame has one contiguous silhouette and no detached pixel islands of eight pixels or larger. Keep this regression gate when adjusting stride angles or limb polygons. Local visual verification should check all eight frames at normal and enlarged scale.

## Verification

```bash
python3 tools/test_avatar_assets.py
node --check crates/pipboy/assets/avatar-preview.js
cargo test --locked -p pipboy avatar
```

The Python gate checks PNG CRC, dimensions, source residency, uniform contract, and preview's reduced-motion/offline properties. Local preview quality is additionally reviewed visually at native size and 480 × 320. Real device performance and full repository CI remain separate acceptance gates.