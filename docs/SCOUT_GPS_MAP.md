# Rust-Boy | Native RustGIS map data + opt-in GPS

**Boston basemap development:** Rust-Boy's independent Web Mercator viewer now supports a bundled, **real City of Boston SAM street-centreline extract** that loads by default when present, plus separately imported RustGIS/GeoJSON. The Boston subset is not the complete city, a native RustGIS raster renderer or Fallout's fictional map. See [Boston source and limitations](BOSTON_BASEMAP.md).

## On RustGIS: export the field map

Preferred interchange: **Publish > Save As > Map File** (`.rgismap`). This is native RustGIS JSON, containing map layers and their embedded feature tables. A normal RustGIS **project save** (`.rgis`) is also accepted: Rust-Boy reads its active map. For a field-sized/portable package, use **Publish > Package > Mobile Map** (`.rgismpk`); Rust-Boy reads the embedded `.rgismap` from the ZIP archive. **Publish > Package > Project** (`.rgispkg`) is accepted too. An individual saved layer file (`.rgislyr`) and WGS84 GeoJSON exports still work.

RustGIS writes vector geometry as WGS84 longitude/latitude (including when a layer's `dataset.crs` refers to its *original* source system); Rust-Boy projects these geometries into EPSG:3857 for its own map display. A layer marked `crs_incomplete` is refused because its coordinates are not trustworthy. Native project map order, visible layers, source references, groups hidden by visibility, and supported single-symbol colors are preserved.

**Only embedded vector feature data is currently drawn.** A project that references external GeoPackage/PostGIS/GeoParquet sources, linked rasters, RustGIS's built-in world backdrop, or tile URLs is **not** self-sufficient in Rust-Boy. Those layers are reported as excluded; they are never fetched silently. A map made only of those sources fails with an explicit import error; the separately sourced Boston street basemap can remain visible below the invalid import.

For truly portable, authoritative geography, load the appropriate public-agency/source layers in RustGIS, narrow the area and choose an embedded vector map or export separate, WGS84 GeoJSON features. **RustGIS is the GIS tool, not the data authority.** Record the underlying data producer, dataset date, license, accuracy, and layer purpose. Rust-Boy's DATA SOURCES view lists the retained `dataset.source` names; a missing source is marked *Not specified*, not assumed official.

## Run in Rust-Boy

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run --locked -p pipboy -- dashboard --micro --open
```

Open **MAP** and press **RUSTGIS** to select `.rgismap`, `.rgis`, `.rgislyr`, `.rgismpk`, `.rgispkg`, or `.geojson`. The map uses a project camera center/zoom if available. **GPS ON** requests browser location permission, then **FOLLOW** centers on the live fix without altering map feature coordinates. GPS is *never* activated by selecting a map. You can drag/touch pan, zoom, and manually enter a center (marked as *not* GPS).

Expand **DATA SOURCES** under the map to see visible imported layers, per-layer feature counts, original source references and unavailable-layer warnings. WGS84 geometry, GPS and accuracy circle share the same screen projection. No files, coordinates or photos are uploaded by Rust-Boy. Switching away from MAP **stops the GPS watch and discards the fix**, while retaining the imported vector geography only in volatile browser page memory for fast return to MAP. The **CLEAR MAP** control erases the temporary imported/online copy, not the installed Boston base; **BOS OFF** hides Boston. Reloading/closing the page clears temporary imports and GPS fixes.


## Online roads and basic features (new: RustGIS-aligned connector)

RustGIS already supports **osm.download** presets through the Overpass API. Rust-Boy's web reference overlay uses the **same provider and basic categories**, without embedding RustGIS's full query/relations engine: roads/paths (`highway`), waterways, water bodies, buildings, public parks, amenity and tourism landmarks. This is **OpenStreetMap community reference data, not an official or government-validated road inventory**; native RustGIS files and the source's own authority remain primary.

**On entering MAP with bundled Boston geometry, there is no automatic external ping.** If that bundled source is missing, the preexisting grid-only fallback may check online reachability with one small GET to `https://overpass-api.de/api/status` (reused for up to ten minutes). That reachability ping sends **no map coordinates, GPS fix, map features, or stored notes**, but does reveal the device's IP address, the local browser origin (as a Referer required by the public Overpass operator), and basic connection metadata to the service. If the provider is unavailable, offline navigation is unaffected.

**To retrieve roads**, tap **WEB ON**. This separate opt-in sends the coordinates of the *currently displayed map area* to `https://overpass-api.de/api/interpreter`; the area may reveal the device's approximate location if the map is following its GPS fix. The service combines the categories into a single bounded Overpass QL request (same preset taxonomy as RustGIS). At zoom below 12.5 it requests that the operator first zoom in or activate GPS. The OSM reference features render **under** imported RustGIS geometry; the local layer is never replaced by an OSM copy. Tap **SYNC** to check the area again; **WEB OFF** stops new requests but keeps the previously fetched shapes as a temporary offline reference.

Network protection: at most one small area per request (roughly 0.044° wide and 0.038° tall), 1,800 returned elements, up to 50,000 geometry vertices and 4 MiB response payload; at most eight data requests per page lifetime, a two-minute minimum interval, and longer backoff for server-busy or rate-limit responses. No rapid-fire requests during pan or zoom, no blind retry loop, and no hidden GPS broadcasts. A new GPS fix that moves outside the cached reference area may request a refresh **only while WEB ON** and subject to those limits. The provider is public shared infrastructure and may refuse or throttle requests.

The visible map attribution **© OpenStreetMap contributors · ODbL** links to [OpenStreetMap copyright](https://www.openstreetmap.org/copyright). The collapsible DATA SOURCES manifest distinguishes the online OSM reference from local source names, lists the source area, fetch timestamp, categories and warnings. Raster imagery, route planning, OSM multipolygon relations and a full RustGIS `osm.download` geometry/attribute parser remain out of scope. For production-scale usage, prefer a self-hosted/licensed data service or bundled regional extracts to avoid burdening shared Overpass servers.

**Disconnect and cleanup:** Failed pings or data retrievals never erase the last good map or native layers. The last successful online map geometry is held in **browser memory only**, including across MAP tab switches. Browser page reload, closing the page, or the **CLEAR MAP** action removes it. WEB ON consent and live GPS watches do not persist across tab switches. No network secrets or user position are written to Rust-Boy's Rust workbench store.

## Supported data and safety limits

- Native `.rgis`, `.rgismap`, `.rgislyr`: JSON up to **12 MiB**; 128 layers, 8,000 features and 120,000 vertices. Active map from a project is selected. GeoJSON retains its stricter **2 MiB** input cap.
- RustGIS ZIP packages: **32 MiB** maximum archive; only one embedded `.rgis` or `.rgismap` JSON is unpacked; size, CRC and path are checked. Supports ZIP stored and raw Deflate compression with a browser implementing `DecompressionStream("deflate-raw")`. If the browser does not support it, extract the `.rgismap` with a ZIP tool and import that. No archive paths are written to disk.
- Native geometries: Point, MultiPoint, multipart Line and multipart Polygon; WGS84 coordinates are strictly validated. Multi-layer drawing is back-to-front, following RustGIS's zero-index-on-top layer contract. Only `Single` symbology is reflected in the canvas; complex classified rendering, labels/scale gates and layer graphics are not guaranteed identical to RustGIS.
- GeoJSON: Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon and GeometryCollection, including FeatureCollections. GeoJSON positions are **[longitude, latitude]**.
- Browser location may be derived from Wi-Fi/IP rather than a physical GNSS receiver. The Raspberry Pi needs an actual browser geolocation provider; USB/serial NMEA input requires a future separately authorized Rust device broker.
- Browser/OS location services may use their own external positioning providers. Rust-Boy itself makes **no** location API posts, remote map tile downloads, hidden GPS recordings, TAK broadcasts or background network discoveries.
- North-up, EPSG:3857 map, no route planning, no raster terrain, no offline raster tile cache, no survey certification. An unreferenced `basemap: Paper` preference in a RustGIS project does not carry actual world geography.

## Verification

```bash
node --check crates/pipboy/assets/map-import.js
node --check crates/pipboy/assets/map.js
node --check crates/pipboy/assets/map-online.js
node tools/test_rustgis_online.cjs
node tools/test_rustgis_online_view.cjs
node tools/test_rustgis_map_import.cjs
node tools/test_rustgis_map_view.cjs
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
python3 tools/smoke_dashboard.py
python3 tools/smoke_micro.py
```

The Node tests cover real RustGIS serde-style geometries, projects' active maps, layer ordering, source metadata, CRS refusals, ZIP stored/Deflate extraction and CRC rejection. **Browser integration, actual RustGIS-exported field layers, Raspberry Pi 480×320 display, GPS precision and native RustGIS renderer/raster acceptance still require manual field tests.** To confirm alignment, import a known authoritative road/parcel/trail layer and compare a recorded surveyed position with the same map in RustGIS.

## Reclamation site markers (manual-only)

From the Reclamation Field Kit, select **MAP** on a recorded site or open **MAP → SITES**. This is a separate, operator-requested local overlay of up to 256 saved manual WGS84 points. A visible marker and its paged, keyboard-accessible list can select/center a site; **no basemap is required** to see site markers on the coordinate grid. The marker feed excludes notes; no server request for it occurs just by opening MAP.

Saved sites are **not** browser GPS fixes, verified survey-grade positions or official RustGIS reference layers. Site-derived centers disable the WEB reference lookup, and **WEB ON** stays blocked while site records are visible. Close SITES to restore the previous center, then explicitly use a manual center or imported layer before opting in to web reference. On closing or navigating away, marker records and selection are discarded from browser map memory. Field verification on Pi/RustGIS remains open.
