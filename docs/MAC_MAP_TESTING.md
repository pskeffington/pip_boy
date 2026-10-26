# Rust-Boy MAP — native macOS test bench

**Status:** Mac is the primary local development and acceptance platform. Rust-Boy is a native Cargo workspace; its MAP interface is browser JavaScript served by the native Rust loopback process. **This is a local development preview, not a macOS app bundle or a certified navigation tool.** Raspberry Pi/physical GNSS/touchscreen certification is deferred, not required for Mac feature development.

This is the operator procedure. The [official county ingestion contract](COUNTY_SOURCES.md) and [map controls](SCOUT_GPS_MAP.md) remain authoritative for source scope, consent, and cartographic limitations.

## Setup on your Mac

Install Rust **1.90+** and Cargo, Git, Python 3, and Node.js **22+** (used for the offline JS checks). Google Chrome or Chromium is optional for automated viewport checks; Safari or another browser can be used to inspect the UI. Node.js is a test dependency, **not** a requirement to run the packaged Rust server.

From your local clone:

```bash
git switch main
git pull --ff-only origin main
bash scripts/test-map-macos.command --verify
bash scripts/test-map-macos.command --launch
```

The **verify** step builds the actual macOS Rust `pipboy` binary, runs county-cache Rust tests, starts a temporary Rust loopback server for the micro HTTP integration test, and executes the offline JavaScript map, GPS permission, county-ingestion, boundary, landmark, layer and RustGIS import tests. It does **not** contact Census, OSM or your GPS provider. It doesn't alter your normal workbench. When Chrome/Chromium is installed it also runs the headless 480×320 / 800×480 / 320×568 viewport fixture. Require that check rather than silently skipping it with `--verify --require-chromium`.

### Audit the real saved county without a network connection

After using MAP → CENTER/GPS → COUNTY once to download the sources of **your chosen location**, stop the app and run:

```bash
# Existing test profile only; starts the real native Rust loopback API temporarily.
bash scripts/test-map-macos.command --audit-cache
# Structured public source metadata only, no coordinates or site records:
bash scripts/test-map-macos.command --audit-cache --json
```

The auditor reads the active GEOID from the **existing isolated test home**, requests its boundary and all four allowlisted source archives using `cache_only:true` and `confirm:false`, and validates exact GEOID/vintage, closed geographic rings, ZIP entry paths, SHP/DBF presence, archive size and CRC. It does not call geolocation, Census or Overpass, nor does it change the cache. It uses the actual compiled Rust server, not an in-memory replacement, and detects a corrupted ZIP even if its initial `PK` magic bytes are intact. It does **not** independently prove Census ZIP authenticity, validate the full SHP geometry, or guarantee every real landmark appears: the normal browser parser still does those ingestion and county-clip checks.

Exit status is **0 COMPLETE** (boundary + all four archives), **2 EMPTY** (no active county), **3 PARTIAL** (missing boundary or archives, or no usable base layer) or **1 FAIL** (corrupt/inconsistent cached data or API failure). Optional sources may legitimately be absent, so PARTIAL is an actionable diagnostic, not proof the MAP cannot render. Keep the GEOID/redacted summary locally; even the county code can disclose approximate location. For the offline acceptance gate, disconnect external networking, repeat `--audit-cache`, then use `--launch` with the *same* test directory and verify the map visually.

A deterministic end-to-end regression for this flow runs inside `--verify` using an invented synthetic county in a temporary directory, including empty, complete, partial, malformed boundary and corrupt ZIP cases. Those fixtures are **test-only**, never bundled as a default displayed map.

The **launch** step runs `cargo run --locked -p pipboy -- dashboard --micro --port 0 --open`. Rust selects a free loopback port and opens the exact URL in your default browser. The test home defaults to **`~/Library/Application Support/Rust-Boy/MapTest`**, which keeps this work distinct from your normal `~/.pipboy` profile. You can override this with an **absolute** `RUSTBOY_MAP_TEST_HOME` path. A cached county GEOID may reveal your approximate area; treat this directory as local location-related data. Do not commit it. Restarting with the same test home intentionally preserves the county cache.

Double-click `scripts/launch-micro.command` for the existing normal-profile launcher, or run `bash scripts/test-map-macos.command --launch` for the separate test profile. Both are native Rust processes using a browser UI. Press **Ctrl+C** in the launching Terminal to stop them. Do not expose or forward the loopback server to the public network.

### Live local MAP acceptance sequence

| Gate | Procedure on your Mac | Expected result |
| --- | --- | --- |
| M1 — cold start | Launch in an empty test home, select Reclamation Scout → MAP | No default city, no location fix, no third-party map traffic until requested |
| M2 — manual center | Select CENTER and enter valid latitude/longitude for a location you choose | Center is clearly labeled manual, not a GPS observation |
| M3 — county ingestion | Select COUNTY and allow the official Census request | Real county GEOID, matched boundary, TIGER 2025 roads/water and optional county-clipped landmarks; partial source errors are visible |
| M4 — rendering | Use FIT, +/- zoom, pan with trackpad/pointer; expand LAYERS | ROADS, WATER, LANDMARKS, BORDER independently toggle without redownloading or erasing data; named landmarks are legible |
| M5 — source inspection | Expand DATA SOURCES | County GEOID/vintage, source counts, partial-source labels, canvas draw timings and source load duration are available |
| M6 — offline restart | After successful M3, stop Rust, disconnect internet, restart with the **same** test home | Previously verified county boundary, roads, water and clipped landmarks reappear without an outbound request |
| M7 — failure recovery | With internet unavailable, select a different uncached county or reopen existing cached data | Current valid map remains available; failure never substitutes unrelated county geometry |
| M8 — GPS permissions | Deliberately select GPS ON; allow or deny browser/OS location permission | Actual browser-provided fix only when available; denial explains recovery and CENTER remains usable |
| M9 — viewport | Chrome devtools device emulation, or `--verify --require-chromium` | 480×320 controls remain readable and reachable; browser emulation is **not** evidence of physical touch behavior |

M3 is the only step that necessarily requires live Census availability and permission. M6 must not use WEB/OSM when validating official offline TIGER records. Point landmarks can be absent legitimately even with valid county geometry; distinguish no-source/zero-feature states from parser failures. Census source geometry has datum and shoreline limitations and must not be treated as navigation-grade information.

### How to assess Mac performance

In MAP → **DATA SOURCES**, observe:

- **VIEWPORT:** candidate/loaded/drawn Census feature counts. This distinguishes indexing effectiveness from total county size.
- **LOCAL CANVAS DRAW:** latest draw duration and rolling **p95 of at most 60 redraws** (milliseconds). This measures canvas drawing only; it is not a measured frames-per-second value, a whole interaction latency, or a promise for another machine.
- **SOURCE DECODE + DELIVERY:** elapsed time from starting the client package load to completion of the source callback chain, including cache access or permitted network waits. The label identifies whether it was explicitly **cache-only** or **network permitted**; network-permitted loads may still be served from cache.

Record the Mac chip family (Apple Silicon/Intel), macOS/browser version, screen mode, county GEOID, features, draw p95, source load duration, source errors, and whether the test was online or offline. Compare **the same county, zoom and viewport** when evaluating optimizations. Avoid publishing actual GPS coordinates, site locations, tokens or personal device identifiers.

| Date | Mac / browser | County GEOID | Mode | Loaded / candidates / drawn | Draw p95 ms | Source load ms | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Not yet tested on operator Mac | — | — | — | — | — | — | Pending local acceptance |

### Mac-specific limits and troubleshooting

- **Missing cargo from Finder:** the Mac launchers prepend `~/.cargo/bin` and the standard Homebrew paths. Verify `cargo --version`; install Rust if unavailable.
- **Missing Node:** install Node 22+ for `--verify`. Rust-Boy itself still runs without Node when launched.
- **Chrome absent:** `--verify` explicitly reports the optional viewport test as skipped; `--require-chromium` fails. You can still inspect MAP manually in Safari. Set `CHROME_BIN` to an executable if Chrome is installed outside standard app locations.
- **GPS denied or missing:** macOS Location Services, browser site permissions and Mac location hardware/network can affect results. Mac browsers often estimate location from nearby networks; that is **not a direct GNSS field test**. Check GPS HELP and use CENTER as the supported fallback. `127.0.0.1` is a trustworthy local origin for browser geolocation; browser-specific settings still apply.
- **Offline map not restored:** confirm the first COUNTY acquisition completed with real source geometry and reuse the identical test home. An invalid/missing cached county polygon prevents state landmark display; roads/water may remain.
- **Slow drawing:** compare source feature counts and p95 at equal zoom. Test the collapsed layer toggles, especially ROADS and LANDMARKS; no style or spatial-index speedup should alter actual source geometry.

## Acceptance evidence boundaries

GitHub Actions on **macos-latest** may pass native build/API/unit/JS tests; these are CI-runner results, **not** measurements on your specific Mac. The operator's M1–M9 results should be recorded separately in [validation evidence](VALIDATION.md) after the actual session. The Mac is the active development device; a Raspberry Pi with physical GNSS/touch/buttons remains a future independent portability gate. No artificial Boston/demo geometry is bundled.
