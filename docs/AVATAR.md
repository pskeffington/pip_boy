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

## Left-hip correction: anchored-gait pass (2026-10-28)

The approved sheet shows a small seam at the operator's left hip. The first
atlas attempted to rotate the complete top of each trouser leg as a rigid
piece. That swung the highlighted crotch fold past the fixed waistband,
creating a moving triangular green projection. An earlier front-leg polygon
trim removed a **detached** fleck but did not eliminate the **connected** flap,
which was visible in a later close-up.

The latest authoring rig keeps the top of **both** trouser legs aligned with
the waistband and eases the rotation down each leg: at source `y=150` swing
influence is zero; by source `y=190`, the full existing leg stride is reached.
The eased inverse remap uses OpenCV for **offline sprite generation only**.
The shipped Rust application still embeds the same 588×900 RGBA atlas and
renders the original eight 100 ms walk frames without running Python or CV.
No right-chest or back decal is introduced; the rod stays on front-left only.

Regression coverage now checks both disconnected sprite islands **and**
the waist-anchored silhouette after accounting for intentional body bob. The
second check is important: connected hip protrusions pass the old island test.
In the defined 29×23-pixel left-hip reference window, the original rigid
rotation altered up to 50 opaque silhouette pixels between frames. The new
articulated frames differ by at most 1 pixel in the same window. Keep the
regression when modifying the limb polygons, pivots, stride amplitude, or
body bob; validate all eight frames and the idle pose visually as well.

## Verification

```bash
python3 tools/test_avatar_assets.py
node --check crates/pipboy/assets/avatar-preview.js
cargo test --locked -p pipboy avatar
```

The Python gate checks PNG CRC, dimensions, source residency, uniform contract, and preview's reduced-motion/offline properties. Local preview quality is additionally reviewed visually at native size and 480 × 320. Real device performance and full repository CI remain separate acceptance gates.
