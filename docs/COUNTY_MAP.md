# Rust-Boy | County map acquisition and shoreline reference

## Operator workflow

1. Open **MAP**. Its initial screen uses a **fully offline Boston street and coastal reference**; no GPS permission is requested and no area data is transmitted simply by opening the map.
2. Press **AREA**. Enter latitude and longitude in WGS84 decimal degrees and press **LOAD COUNTY**, *or* press **USE GPS** to request one browser location fix and immediately load that position's county. The panel explicitly states that the selected position goes to the **U.S. Census TIGERweb** API.
3. The loader resolves a unique U.S. county polygon (January 1, 2026 vintage), then acquires real primary, secondary and local roads, railroads, water lines and water polygons for its surrounding envelope. Road/linear geometry is clipped to the returned county polygon. The viewport fits that county automatically; **no unrelated county** is loaded.
4. Press **BOSTON** in the AREA panel to return to the bundled Boston reference without another network request. **RUSTGIS** still allows operator-provided native maps, GIS packages and GeoJSON. **SITES** keeps locally entered Reclamation site markers distinct from any official map data.

**No GPS is inferred from a manual input.** The one-time **USE GPS** control requests location explicitly, not a background watch. A location can be precise and sensitive: choosing GPS or pressing LOAD COUNTY sends the selected coordinates to the third-party Census service and necessarily reveals normal connection metadata such as the device IP address. An offline/failing provider does not synthesize missing streets or erase the existing map; the last usable layers remain displayed.

## Offline Boston: what the geometry actually shows

| Layer | Provenance | Role and limitations |
| --- | --- | --- |
| Street centerlines | City of Boston SAM street segments (pinned Olin College mirror), 10,634 original real segments | Offline road reference, historic snapshot; restricted to the documented central-Boston extract |
| City land footprint | **City of Boston Boundary (Water Excluded)**, [pinned source mirror](https://github.com/olincollege/boston-bike-accidents/blob/36532f6e0bdefc92a9cb5f4a7cedf17a366c3ab0/data/City_of_Boston_Boundary_(Water_Excluded).geojson), blob `2f218f58966a22549d1430b2d2b4a77718a78235` | 34 real multipolygon parts/147 rings; source vertices selectively retained with ~3 m nominal Douglas–Peucker simplification. This is the **Boston city** land/water-excluded footprint, **not** a Suffolk County outline |
| Coastline | [Natural Earth 1:10m coastline](https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_coastline.geojson), pinned Git blob `ff2cb1a203a39eff0d202302b72e234733dd9bdf` | 167 actual source coordinates retained where the coastline intersects the regional extract; natural/generalized (public domain), not surveyed harbor shoreline or tidal navigation data |
| Historical Suffolk county outline | Plotly MIT [historical U.S. counties GeoJSON](https://github.com/plotly/datasets/blob/master/geojson-counties-fips.json), blob `c8dbfc0dc4920aae2c3ba1dae290ab986a6b44fd`, FIPS `25025` | Thin dashed **context only**, explicitly marked generalized/historical, never used as a 2026 legal boundary |
| User-loaded county | [U.S. Census TIGERweb](https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/State_County/MapServer), layer 1 counties; [Transportation](https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/Transportation/MapServer) layers 2/6/8/9; [Hydrography](https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/Hydro/MapServer) layers 0/1 | January 1, 2026 county outline and roads/water references from the service, requested by operator; no assumption they reflect current traffic, passability or surveyed precision |

Only sourced geographic vertices are used. County/road intersection endpoints are calculated mathematically where actual roads cross real polygon boundaries; these do not imply a new invented road. Only the presentation uses Rust-Boy's original phosphor styling. No fictional Fallout vaults, roads, landmarks, game regions or derived narratives are geographic features. Refer to [Boston street basemap policy](BOSTON_BASEMAP.md), [lore names](LORE_NAMING.md) and [branding](../BRANDING.md).

## Network, performance and correctness gates

- County downloads begin **only** after an explicit **LOAD COUNTY** or **USE GPS** action. Opening MAP or AREA, displaying shoreline geometry and loading saved survey sites never initiate a Census county request.
- Each county starts with one point-to-polygon lookup, followed by up to six feature layer pagination loops. Requests contain a limited WGS84 county envelope, which can contain non-target features **in transport**; rendered road/rail/linear-water vectors are clipped to the target county. Filled water polygons are clipped by the renderer to the county shape when canvas clipping is available.
- Bounded: no more than 4 MiB per response, 32 MiB total, 28,000 features and 230,000 vertices, up to 25 pages of 1,200 features per layer, 4 explicit county loads per page session. Excessive, ambiguous, corrupt and unavailable data **fail closed**, retaining the previous map. Large counties and offshore/remote locations may require a separately downloaded local RustGIS extract.
- The county download is not persisted to disk; the active result can survive a MAP tab switch in the same browser page only. No private Reclamation notes are sent. WEB ON/Overpass lookup is disallowed while a saved-site or county view is active; data-source labels distinguish each provider.
- Census TIGERweb, even with verified source metadata, is an *externally operated service*. Browser CORS, provider load, layer paging and full-scale county completeness still require live tests. No self-hosted county data service or regional offline atlas is claimed.
- The Boston city polygon and Natural Earth coastline are simplified/generalized context, not legally authoritative boundaries or charts. This map is **not** a routing engine, guaranteed-access road inventory, hydrographic chart, flood prediction, or survey navigation instrument.

## Validation commands

```sh
node --check crates/pipboy/assets/county-map.js
node --check crates/pipboy/assets/boston-land-base.js
node --check crates/pipboy/assets/boston-coast-base.js
node --check crates/pipboy/assets/map.js
node tools/test_county_map.cjs
node tools/test_boston_map.cjs
cargo test --workspace --locked
```

Automated browser simulations validate default **no-network startup**, real Boston shoreline/land rendering, manual and one-shot GPS county selection, county clipping, failure preservation, WEB restriction and restoration to Boston. Live Census retrieval, physical GNSS and Raspberry Pi screen performance remain separate acceptance gates.
