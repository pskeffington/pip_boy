# Rust-Boy | Opt-in location and GeoJSON map viewport

**Status:** working browser-based WGS84 coordinate viewer on `feat/scout-gps-rustgis-map`. This is an independently implemented map viewport, **not** an embedded RustGIS application or its native renderer. It displays georeferenced user-imported RustGIS-compatible GeoJSON, grid, position, accuracy circle, panning and zoom. Without an imported layer, **no streets, terrain, basemap tiles or routing are shown**.

## Launch

```bash
git clone --branch feat/scout-gps-rustgis-map https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run --locked -p pipboy -- dashboard --micro --open
```

Open **MAP**. Press **GPS ON** and grant the *browser's* location permission. GPS is never started by opening the map. Browser geolocation requires a secure context; this local dashboard uses http://127.0.0.1, which modern browsers treat as potentially trustworthy. A desktop Mac may have only Wi-Fi/IP positioning, or no location provider. On Raspberry Pi, the browser needs a working geolocation provider; attaching a USB GNSS receiver does *not* automatically expose it to `navigator.geolocation`. A future GNSS broker can supply NMEA/serial fixes only after explicit device consent and OS permission controls.

Use **LAYER** to import an offline WGS84 GeoJSON file exported from RustGIS. Supported GeoJSON: Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, GeometryCollection, Feature, FeatureCollection. **GeoJSON is always [longitude,latitude]**, whereas the displayed manual center uses LAT then LON. The renderer applies a Web Mercator (EPSG:3857) transform in memory and clamps rendering latitudes to ~85.05 degrees, including antimeridian-aware longitude wrapping. It rejects non-WGS84 CRS declarations, invalid coordinates, unknown geometries, files over 2 MiB, >2,000 features or >30,000 vertices. Polygons use even-odd fill for interior rings. Do not import unknown or highly sensitive layers.

Controls: **GPS ON/OFF** starts and stops a position watch; **FOLLOW/RECENTER** follows/recenters on the last fix; **LAYER** selects a local file; **CENTER** manually enters a coordinate (never labeled as GPS); **+/-**, mouse wheel and drag/touch pan change the view. Pressing other tabs or switching profiles closes the watcher and discards the fix and imported layer. The screen shows *NO BASEMAP LAYER* when there is no imported geography; the center crosshair is only a viewport reference.

## Privacy and limitations

- No location is posted to Rust-Boy HTTP APIs, persisted, shared to peers, transmitted to TAK, or added to notes. No tile service, geolocation proxy or third-party JavaScript is loaded by Rust-Boy.
- Browser/OS location services may independently consult operating-system or third-party positioning providers; their privacy behavior is **outside** Rust-Boy's control.
- No geocoding, turn-by-turn route calculation, paths, raw satellite lock indicators, magnetic declination or quality-certified navigation is present.
- Imported layers may have been created on a different coordinate reference system: convert them to WGS84 (EPSG:4326) **before** GeoJSON export from RustGIS. Matching display positions is not a field survey.
- Browser-provided accuracy and timestamps are displayed; >60-second-old samples and malformed coordinates are rejected. Loss of permission or provider error removes the fix.
- The OS device broker remains deny-only. Do not treat browser GPS as authorization to access serial GNSS hardware.

## Acceptance

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
node --check crates/pipboy/assets/map.js
node --check crates/pipboy/assets/micro.js
node --check crates/pipboy/assets/dashboard.js
python3 tools/smoke_micro.py
python3 tools/smoke_dashboard.py
```

Manual tests: load RustGIS GeoJSON exported in EPSG:4326; ensure fixed landmarks, own position and uncertainty circle agree at the same coordinates; verify no fake basemap; test pan/zoom and antimeridian layer; deny GPS; exit MAP while watching and confirm browser location watch stops; test 480x320 and 800x480 Pi touchscreens, missing GNSS and denied permission. Real Pi hardware/browser acceptance and upstream RustGIS native renderer integration remain separate gates.
