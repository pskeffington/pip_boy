# Rust-Boy | U.S. Census county ingestion

This source contract acquires **real vector records** for a deliberately selected U.S. county. No bundled demonstration city or manufactured geometry is part of normal runtime. The map is styled by Rust-Boy, but source shapes are not altered into fictional roads or points.

All acquisition/source controls now live under **Settings → Location / Map** in micro and desktop. MAP itself is a full geographic viewport without buttons.

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

## macOS offline integrity audit

After an operator-initiated COUNTY load, the [Mac-native cache audit](MAC_MAP_TESTING.md#audit-the-real-saved-county-without-a-network-connection) can replay the active county through Rust-Boy's own **cache-only** API. Run `bash scripts/test-map-macos.command --audit-cache`; no external map, GPS, or manual coordinates are requested. The audit checks the independently validated stored boundary and each 2025 TIGER archive's ZIP CRC, compressed/uncompressed size, source naming and SHP/DBF presence. It reports COMPLETE, PARTIAL, EMPTY or FAIL without exporting geometry or site records. This independent cross-check catches corrupted ZIP bytes that the Rust caching layer's initial signature check alone would not find. It is source integrity verification, **not** cryptographic authentication of Census files or a survey-grade map claim.

## Optional ZIP/ZCTA to county index

The MAP ZIP input and reverse **COUNTY ZIPs** listing use the [2020 Census ZCTA-to-county relationship](ZIP_COUNTY_LOOKUP.md). The national crosswalk is obtained from a fixed `www2.census.gov` URL only after explicit operator action, validated, and cached separately from up to four 2025 county GIS packages in `PIPBOY_HOME/geo_index/`. ZIP values are never forwarded to Census. A ZCTA can intersect multiple counties; the operator chooses the desired 2025 GEOID for GIS ingestion. ZCTAs are statistical approximations, **not** USPS delivery ZIPs, towns or certified county membership.

## Rendering and layer visibility

The local MAP engine groups source IDs into four display categories: **ROADS** (`tiger-roads`), **WATER** (`tiger-areawater` and `tiger-linearwater`), **LANDMARKS** (`tiger-pointlm`, clipped by verified boundary), and **BORDER** (the independently validated Census county polygon). Toggling only changes canvas visibility. It never edits source ZIPs, county identifiers, attributes, or the boundary cache, and never sends a request. RustGIS and OSM overlay controls remain independent.

A bounded, non-persistent grid index is constructed from the SHP-derived per-feature envelopes after each successful source parse. Viewport lookups use source geometries without approximating their vertices. Very large envelopes, missing bounds, world-spanning and antimeridian views are handled conservatively by scanning originals, never by silently omitting features. The low-zoom road overview selects major Census MTFCC types first, without throwing away local streets. A 20,000-feature synthetic test asserts reduced viewport candidates, toggles, source preservation, landmark-first labels, and no network/GPS side effects; this is **not** an on-device performance benchmark.

## Offline county-boundary acquisition

The 2025 Census TIGER/Line `COUNTY` shapefile is a **national** 80 MiB archive (`tl_2025_us_county.zip`), unsuitable for direct handheld loading. Rust-Boy instead uses bounded Census TIGERweb **ACS 2025 Counties layer 37**, queried for an exact five-digit county GEOID. No bounding-box approximation replaces real county geometry.

After the operator explicitly selects COUNTY, the browser first resolves the selected GPS/manual coordinate at TIGERweb. The local Rust host then **independently requests the corresponding boundary by GEOID only**, via `POST /api/map/boundary` with `{geoid,confirm:true,cache_only:false}`. Neither the loopback boundary request nor the host's Census request includes the user's coordinates or private sites. The host validates the HTTPS GeoJSON response: exactly one FeatureCollection feature, matching GEOID, a valid name, Polygon/MultiPolygon rings, geographic coordinate bounds and closure, at most 256 rings/12,000 vertices, and no more than 1 MiB. Fixed `curl` arguments prohibit arbitrary destinations, redirects, retries, shell execution and disabled TLS verification.

On success, the **raw validated GeoJSON** is atomically stored in `county_cache/boundary_2025_{GEOID}.geojson`, under the existing four-county cache limit. Every offline read revalidates the actual source JSON against its GEOID. A corrupt cache fails closed rather than being replaced silently. The local `GET /api/map/cache` reports `boundary_cached` without contacting Census or requesting geolocation.

On future MAP launches, `POST /api/map/boundary` with `cache_only:true` restores the polygon from local storage, then cached county roads/water and state POINTLM landmarks are decoded. The **county boundary clips the landmarks even after a cold offline restart**. If no valid boundary is cached, roads and water still load while state landmark points stay hidden. Automatic cache restore must not replace a center explicitly selected by the operator. An explicit COUNTY action while offline may reuse a cached county only when the chosen position falls within that polygon (respecting holes).

These county boundaries are generalized Census statistical geography, not signed files, surveyed shorelines or certified navigation guidance. Very large/complex polygons, upstream outages, mixed vintages, stale data and real handheld performance still require field acceptance.

Primary sources: [TIGERweb county layer directory](https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/State_County/MapServer/layers), [2025 national county archive](https://www2.census.gov/geo/tiger/TIGER2025/COUNTY/), [2025 technical documentation](https://www.census.gov/programs-surveys/geography/technical-documentation/complete-technical-documentation/tiger-geo-line.2025.html).

## Cache and access controls

- Up to four county cache keys; original ZIP archives persist under `PIPBOY_HOME/county_cache/` (default `~/.pipboy/county_cache/`). An active GEOID is stored locally and reveals an approximate area; protect the device and its storage.
- On MAP entry, the loopback API can restore the active county ZIPs and revalidated cached county polygon **without GPS, Census or OSM network requests**. POINTLM state points are then clipped and displayed offline when a valid stored polygon exists; otherwise they remain hidden.
- Every ZIP is capped at 12 MiB; decompressed data, record counts (60,000) and geometry vertices (450,000 per linear/polygon source) are bounded. Oversize county archives require a validated smaller RustGIS/GeoJSON extract rather than silently dropping shape data.
- COUNTY sends the chosen point to Census; the subsequent ZIP fetch sends only the GEOID/state identifiers to the Census archive host. Separate **WEB ON** sends the current viewport to Overpass as community reference data. Neither action can share saved private Reclamation site coordinates.

## Test and release gates

`tools/test_county_sources.cjs` validates source-list/API boundaries, SHP/DBF, ZIP/CRC and offline cached area geometry. `tools/test_pointlm_ingestion.cjs` validates the state-based archive name, point SHP decoding, required real county polygon, exclusion of out-of-county points and interior holes, and refusal of invalid records. Rust unit tests validate fixed URL derivation and cache integrity. Browser and Rust CI must pass; production acceptance still requires real Census service calls, real dense counties, Raspberry Pi touchscreen/GNSS and visual datum verification.

Sources: [Census TIGER/Line](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html), [2025 MTFCC reference](https://www.census.gov/library/reference/code-lists/mt-feature-class-codes.html), [2025 ZIP archive](https://www2.census.gov/geo/tiger/TIGER2025/).
