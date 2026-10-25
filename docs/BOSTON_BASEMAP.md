# Rust-Boy | Boston real-geometry offline basemap

## Ground truth and coverage

The default MAP view shows **real, modern central Boston** street centerlines. This is not the fictional Fallout map. No streets, landmarks or travel routes are invented.

- **Source:** [City of Boston Street Segments, SAM System](https://data.boston.gov/dataset/boston-street-segments-sam-system) via the [Boston Segments GIS service](https://gisportal.boston.gov/arcgis/rest/services/Infrastructure/OpenData/MapServer/8).
- **Pinned source mirror:** [Olin College/Volpe night-light research](https://github.com/olincollege/night-light/blob/33b91f4179c5c910ad3b79934bc457e86c829c31/datasets/boston_street_segments.geojson); Git blob SHA **a86dc5f6913987585e429e625bdc61781d03d40e**. This is a historical snapshot, not a current city feed.
- **Licensing:** [Boston Street Segments metadata](https://www.arcgis.com/home/item.html?id=8f31b239f40847e29543322093c717fc) states ODC PDDL 1.0. The City also has a [general data license agreement](https://www.boston.gov/government/cabinets/innovation-and-technology/city-boston-data-license-agreement) with restrictions. Confirm how these terms interact **before public dataset redistribution or release**; Olin's software license alone does not license the GIS data.
- **Geographic selection:** WGS84 longitude -71.140 to -70.990, latitude 42.309 to 42.399. A source segment is selected if at least one **real source vertex** falls inside. Entire original segments, including small excursions beyond the box, are retained.
- **Included:** 10,634 uniquely identified actual road segments, 47,994 source vertices, 2,522 original street names. The source CFCC road classification is preserved. Coordinates are rounded to six WGS84 decimals (~0.1 m coordinate quantization, **not** a ground-survey accuracy claim).
- **Not included in the street dataset:** The whole city or Greater Boston, buildings, live closures, trails missing from source, Cambridge/Somerville streets or any fictional game location. Separate pinned City boundary and Natural Earth coastline references now provide real **shoreline and land geometry**; see [county and coast guide](COUNTY_MAP.md) for their limitations.

## Implementation

The data file at crates/pipboy/assets/boston-base.js contains **source-derived records** only. Its decoder (boston-map.js) refuses unknown source IDs or malformed/changed counts, verifies WGS84 bounds, and returns actual street polylines. The original map.js displays the offline Boston layer below separately imported RustGIS/GeoJSON and Reclamation site markers. Lower zooms display a smaller **real** subset based on source road classes. Labels are taken only from original street name/type fields.

Original phosphor colors, road-width hierarchy, fine grid and optional scanlines provide a retro-futurist look **without changing the street geometry**.

MAP starts with local road, land-footprint and coastline geometry and no external tiles or location permissions. Browser GPS still requires operator action. WEB ON is a distinct optional provider request that can disclose a viewport; it remains blocked when saved Reclamation sites are visible. The BOS OFF / BOS ON button hides or restores the included base; CLEAR MAP clears imported/online data while leaving the fixed Boston reference available.

The map provides its municipal, Natural Earth and historic Suffolk county sources in DATA SOURCES and attribution. **AREA** accepts explicit manual coordinates or one-time GPS; **LOAD COUNTY** retrieves and renders the actual 2026 Census county boundary, roads, rail and hydrography for the selected location. No automatic location disclosure occurs on opening MAP. See [county source and privacy guide](COUNTY_MAP.md). It is **not certified for navigation**, current closures, road safety or field hardware; no raster or full native RustGIS renderer is bundled.

## Rebuild from pinned source (offline)

Obtain the exact original pinned GeoJSON from the source mirror, then run:

    python3 tools/build_boston_basemap.py --input /path/to/boston_street_segments.geojson --check

The check requires a byte-identical Git source blob and compares its decoded source geometry with the bundled file. Remove --check to regenerate. No network, guesses, interpolated vertices or fabricated roads are permitted.

## Software quality

    node tools/test_boston_map.cjs
    node tools/test_rustgis_map_view.cjs
    node tools/test_reclamation_map.cjs

The tests cover WGS84 source coordinates, counts, default offline camera, source credit, optional base toggle, real imports, invalid data denial, and privacy preservation. Raspberry Pi display performance, geometry completeness and field GNSS accuracy are **not** established by software tests.
