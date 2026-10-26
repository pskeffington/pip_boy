# Rust-Boy | U.S. Census county ingestion

This source contract acquires **real vector records** for a deliberately selected U.S. county. No bundled demonstration city or manufactured geometry is part of normal runtime. The map is styled by Rust-Boy, but source shapes are not altered into fictional roads or points.

## Provenance and fixed acquisition path

- **County resolution:** after GPS or explicit WGS84 manual center, pressing COUNTY sends that *selected point* to U.S. Census TIGERweb `State_County/MapServer/37` for a bounded polygon and five-digit county GEOID. The county boundary is legal/statistical geography, **not** a surveyed shoreline.
- **Local package API:** `GET /api/map/sources` provides the pinned source manifest; `POST /api/map/package` accepts `{geoid,source,confirm,cache_only}` and **does not accept a URL or GPS coordinate**. Outbound downloads are restricted to `www2.census.gov` via fixed HTTPS origin.
- **Vintage:** all downloadable archives are pinned to **2025** until compatibility tests explicitly approve 2026 or later data. Census released the 2026 TIGER/Line shapefiles on October 14, 2026; do not mix file vintage silently.

| Source ID | Scope | Allowlisted archive | Rendering purpose |
| --- | --- | --- | --- |
| `tiger-roads` | county | `TIGER2025/ROADS/tl_2025_{GEOID}_roads.zip` | MTFCC-classified major/local streets and trails |
| `tiger-areawater` | county | `TIGER2025/AREAWATER/tl_2025_{GEOID}_areawater.zip` | lakes, bays and other areal water |
| `tiger-linearwater` | county | `TIGER2025/LINEARWATER/tl_2025_{GEOID}_linearwater.zip` | streams and rivers |
| `tiger-pointlm` | **state** | `TIGER2025/POINTLM/tl_2025_{STATEFP}_pointlm.zip` | named point landmarks, clipped to confirmed county |

The host derives `STATEFP` strictly from the first two digits of the validated county GEOID and never accepts a caller-defined state, path or origin. The state POINTLM archive is stored under a **county-isolated cache filename**, and is drawn only after a verified county polygon was provided to the decoder. Without that polygon, the source is skipped with an explicit restriction: no state-level markers are presented as county markers.

## Decode and visualization

1. Server downloads with bounded TLS-verified `curl` (fixed host, no redirects, no shell, no retries, 40-second cap, 12 MiB per ZIP). Writes use temporary files and atomic persistence so failed downloads never corrupt existing caches.
2. The browser validates the ZIP central directory/member identities, decompressed sizes and CRC. It accepts only expected SHP/DBF members for the exact archive name. It validates SHP header, record sequence, geometry types, finite geographic coordinate ranges, path structure and resource budgets.
3. For roads and hydrography, the viewer uses actual SHP coordinates, names and MTFCC attributes. At lower zoom, minor roads are suppressed for legibility; source data is not simplified or fabricated.
4. For state POINTLM, every actual point must pass the verified county polygon's even/odd ring test, including hole exclusion. Only named, in-county features receive visible landmark markers, with source-provided MTFCC retained. Invalid/missing county boundary means **no landmark display**.
5. Valid layers replace a prior county only after a source passes validation, and subsequent layers are kept grouped by the same GEOID. A completely failed new acquisition preserves the last good county geometry. Partial-source failures are labeled; they do not claim a complete inventory.
6. FIT frames the verified loaded county geometry. The SOURCE manifest reports per-layer feature/vertex counts and whether the original state-based landmark dataset was county-clipped.

Source CRS is **GCS NAD83**; Rust-Boy displays raw longitude and latitude in a Web Mercator viewer and currently does **not** apply a formal NAD83-to-WGS84 transform. Features are reference cartography, not survey-grade alignment, road condition, or verified safe passage. Water polygons do not by themselves certify coastlines.

## Cache and access controls

- Up to four county cache keys; original ZIP archives persist under `PIPBOY_HOME/county_cache/` (default `~/.pipboy/county_cache/`). An active GEOID is stored locally and reveals an approximate area; protect the device and its storage.
- On MAP entry, the local loopback API can restore the active county ZIPs **without GPS, Census or OSM network requests**. A cached state POINTLM file cannot be clipped in a cold session that lacks an authoritative county polygon; it remains hidden until one is available.
- Every ZIP is capped at 12 MiB; decompressed data, record counts (60,000) and geometry vertices (450,000 per linear/polygon source) are bounded. Oversize county archives require a validated smaller RustGIS/GeoJSON extract rather than silently dropping shape data.
- COUNTY sends the chosen point to Census; the subsequent ZIP fetch sends only the GEOID/state identifiers to the Census archive host. Separate **WEB ON** sends the current viewport to Overpass as community reference data. Neither action can share saved private Reclamation site coordinates.

## Test and release gates

`tools/test_county_sources.cjs` validates source-list/API boundaries, SHP/DBF, ZIP/CRC and offline cached area geometry. `tools/test_pointlm_ingestion.cjs` validates the state-based archive name, point SHP decoding, required real county polygon, exclusion of out-of-county points and interior holes, and refusal of invalid records. Rust unit tests validate fixed URL derivation and cache integrity. Browser and Rust CI must pass; production acceptance still requires real Census service calls, real dense counties, Raspberry Pi touchscreen/GNSS and visual datum verification.

Sources: [Census TIGER/Line](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html), [2025 MTFCC reference](https://www.census.gov/library/reference/code-lists/mt-feature-class-codes.html), [2025 ZIP archive](https://www2.census.gov/geo/tiger/TIGER2025/).
