# Rust-Boy | County source acquisition

This is the **U.S.-only** county acquisition gate for handheld MAP. It uses actual geographic records, not procedural roads, fictional features or approximated shorelines.

## Acquisition chain

1. Open MAP. On an unconfigured device the embedded, provenance-pinned Boston SAM roads load without using network or GPS. If a previously selected county has cached packages, MAP retrieves the cache manifest from the **local** Rust server and decodes only the locally cached archives. Neither step activates location services or a remote connection.
2. Select **GPS ON** and accept the browser's location prompt, or select **CENTER**, enter valid WGS84 latitude/longitude, and submit. A manual center is explicitly **not** a GPS fix. Location can be acquired by the OS/browser, which may rely on external positioning services.
3. Select **COUNTY** (or select COUNTY before obtaining location). This permits the browser to transmit the chosen position to **U.S. Census TIGERweb**, query its containing county, and obtain a real, bounded 2025 county polygon + five-digit GEOID. This legal/statistical boundary does **not** independently establish the physical coastline.
4. Given a valid Census-resolved GEOID, Rust-Boy reads its fixed allowlist at \`GET /api/map/sources\`, then requests three county-wide 2025 TIGER/Line archives through the loopback-only local Rust API \`POST /api/map/package\`: **All Roads**, **Areal Water**, **Linear Water**. The server accepts **only** GEOID and predefined source IDs; no raw GPS coordinates or arbitrary network destination.
5. The browser validates ZIP directory, member paths, decompressed lengths and CRC; reads actual SHP vertices in WGS84 longitude/latitude order and optional DBF street names / MTFCC road classes; and draws water polygons, linear water and roads below imported RustGIS layers. Nothing is drawn if the source fails validation. Existing Boston geometry remains available as a fallback.
6. The loopback service stores successful ZIPs under \`PIPBOY_HOME/county_cache/\` (default \`~/.pipboy/county_cache/\`), and writes the last successfully acquired GEOID. Later MAP mounts load only available cached sources with \`cache_only:true\`. **COUNTY OFF** hides the current county layer and cancels requests; it does not destroy the offline cache.

## Fixed source list (vintage pinned)

| ID | Official source pattern | Geometry | Scope |
| --- | --- | --- | --- |
| \`tiger-roads\` | \`https://www2.census.gov/geo/tiger/TIGER2025/ROADS/tl_2025_{GEOID}_roads.zip\` | Street / road centerlines | Containing county |
| \`tiger-areawater\` | \`https://www2.census.gov/geo/tiger/TIGER2025/AREAWATER/tl_2025_{GEOID}_areawater.zip\` | Water polygons | Containing county |
| \`tiger-linearwater\` | \`https://www2.census.gov/geo/tiger/TIGER2025/LINEARWATER/tl_2025_{GEOID}_linearwater.zip\` | Rivers, streams, linear waterways | Containing county |

Census has published 2026 TIGER/Line records, but this release intentionally **pins known 2025 file paths and schema** until newer ZIPs, schemas and provenance have passed release validation. Do not silently swap vintages between boundary and downloaded roads.

Boundary provenance: U.S. Census TIGERweb \`State_County/MapServer/37\`, ACS 2025. Road and water packages: Census TIGER/Line 2025. Drawn water edges can provide reference shoreline context, but no source here is a survey-grade tidal shoreline or a NOAA nautical chart. Field routing and navigation require independent validation.

## Resource and privacy constraints

- Up to **4 counties** retained; at most **12 MiB** per downloaded ZIP and **40 seconds** per fixed-host request, with no automatic retries or redirect following. A missing optional layer leaves the remaining valid layers marked partial. No roaming across adjacent counties or synthetic stitching.
- \`curl\` / \`curl.exe\` must be available for outbound HTTPS on the native host. The server invokes it with fixed arguments, fixed Census origin, bounded time/size and no shell. It never accepts a caller-supplied URL. TLS verification is not disabled.
- The ZIP decoder limits entries, extracted file sizes, record counts (60,000) and vertices (450,000 per source); exceeding a bound fails without portraying an incomplete map as complete.
- Browser GPS is opt-in. Selecting COUNTY permits a separate external Census boundary lookup with the selected coordinates; package download itself sends only GEOID to Census. Downloaded archives are plaintext files. The saved active county GEOID reveals approximate region; operators should use \`PIPBOY_HOME\` on an appropriately protected device.
- External OSM WEB ON remains separate opt-in and viewport-limited; it is **not** used to imply a complete county road inventory. COUNTY will not expose saved private Reclamation site coordinates to remote services.
- Large counties that exceed safety limits must use smaller, validated RustGIS extracts. This is not a routing engine, offline GNSS receiver or certified hydrographic navigation dataset.

## Acceptance tests

\`node tools/test_county_sources.cjs\` exercises fixed-source lookup, real-format synthetic fixture ZIP/SHP/DBF parsing, CRC, raw geometry, and cache-only access. Rust unit tests check the server's GEOID/source restrictions, no side effects on invalid requests, and cached-byte responses. \`tools/test_county_map.cjs\` checks user-selected coordinate → Census boundary. Existing Boston/RustGIS/Reclamation tests ensure the new layers do not fabricate map features.

Production Census service availability, large real-world ZIP compatibility, 480×320 Raspberry Pi responsiveness and correct physical shoreline placement remain separate field gates.
