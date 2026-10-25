# Rust-Boy | Native RustGIS map data + opt-in GPS

**Development branch:** `feat/scout-gps-rustgis-map` (draft PR #10). Rust-Boy now has an independent Web Mercator canvas viewer that reads *native RustGIS data exports*, draws embedded WGS84 vector geometry, and overlays opt-in browser geolocation. It is **not** RustGIS's native raster, basemap, symbology or map-renderer engine; no roads, terrain or imagery appear unless their *actual* geographic data is imported.

## On RustGIS: export the field map

Preferred interchange: **Publish > Save As > Map File** (`.rgismap`). This is native RustGIS JSON, containing map layers and their embedded feature tables. A normal RustGIS **project save** (`.rgis`) is also accepted: Rust-Boy reads its active map. For a field-sized/portable package, use **Publish > Package > Mobile Map** (`.rgismpk`); Rust-Boy reads the embedded `.rgismap` from the ZIP archive. **Publish > Package > Project** (`.rgispkg`) is accepted too. An individual saved layer file (`.rgislyr`) and WGS84 GeoJSON exports still work.

RustGIS writes vector geometry as WGS84 longitude/latitude (including when a layer's `dataset.crs` refers to its *original* source system); Rust-Boy projects these geometries into EPSG:3857 for its own map display. A layer marked `crs_incomplete` is refused because its coordinates are not trustworthy. Native project map order, visible layers, source references, groups hidden by visibility, and supported single-symbol colors are preserved.

**Only embedded vector feature data is currently drawn.** A project that references external GeoPackage/PostGIS/GeoParquet sources, linked rasters, RustGIS's built-in world backdrop, or tile URLs is **not** self-sufficient in Rust-Boy. Those layers are reported as excluded; they are never fetched silently. A map made only of those sources fails with an explicit error instead of drawing an empty frame that appears to be a functioning map.

For truly portable, authoritative geography, load the appropriate public-agency/source layers in RustGIS, narrow the area and choose an embedded vector map or export separate, WGS84 GeoJSON features. **RustGIS is the GIS tool, not the data authority.** Record the underlying data producer, dataset date, license, accuracy, and layer purpose. Rust-Boy's DATA SOURCES view lists the retained `dataset.source` names; a missing source is marked *Not specified*, not assumed official.

## Run in Rust-Boy

```bash
git clone --branch feat/scout-gps-rustgis-map https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run --locked -p pipboy -- dashboard --micro --open
```

Open **MAP** and press **RUSTGIS** to select `.rgismap`, `.rgis`, `.rgislyr`, `.rgismpk`, `.rgispkg`, or `.geojson`. The map uses a project camera center/zoom if available. **GPS ON** requests browser location permission, then **FOLLOW** centers on the live fix without altering map feature coordinates. GPS is *never* activated by selecting a map. You can drag/touch pan, zoom, and manually enter a center (marked as *not* GPS).

Expand **DATA SOURCES** under the map to see visible imported layers, per-layer feature counts, original source references and unavailable-layer warnings. WGS84 geometry, GPS and accuracy circle share the same screen projection. No files, coordinates or photos are uploaded by Rust-Boy. Switching away from MAP destroys the current imported layer and position watch.

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
node tools/test_rustgis_map_import.cjs
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
python3 tools/smoke_dashboard.py
python3 tools/smoke_micro.py
```

The Node tests cover real RustGIS serde-style geometries, projects' active maps, layer ordering, source metadata, CRS refusals, ZIP stored/Deflate extraction and CRC rejection. **Browser integration, actual RustGIS-exported field layers, Raspberry Pi 480×320 display, GPS precision and native RustGIS renderer/raster acceptance still require manual field tests.** To confirm alignment, import a known authoritative road/parcel/trail layer and compare a recorded surveyed position with the same map in RustGIS.
