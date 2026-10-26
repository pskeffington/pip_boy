# MAP — ZIP/ZCTA ↔ County GIS lookup

The MAP **ZIP** control allows a user to enter a familiar five-digit U.S. ZIP code without needing coordinates or to grant GPS access. Rust-Boy looks for a matching **2020 Census ZIP Code Tabulation Area (ZCTA)** and presents its intersecting **2020 counties**. After one county is selected, the existing independent **2025 county-boundary and TIGER/Line GIS ingestion** pipeline loads real roads, waterways, and county-clipped landmarks. ZIP lookup is a *statistical routing aid*, not postal address validation.

## Operator workflow (Mac first)

1. Open MAP, press **ZIP**, type exactly five digits (including leading zeros), then press **FIND COUNTY**. No location or external provider request happens until FIND COUNTY is pressed.
2. On first connected use, the native Rust service downloads and validates the fixed Census 2020 **national ZCTA5-to-county relationship file** (~6.5 MB) into `PIPBOY_HOME/geo_index/`. **The entered ZIP is sent only to the local loopback Rust server**, not as a URL, query parameter, or payload to Census. Repeated ZIP lookups use the same cached index.
3. **One matching county:** the operator's FIND COUNTY action authorizes loading that county's 2025 Census boundary/roads/water and optional in-county landmarks. MAP fits its *verified county geometry*, not a guessed ZIP centroid.
4. **Multiple intersecting counties:** each source county, GEOID and percentage of the ZCTA's land area is offered as a separate button. The operator chooses which county GIS to load; the largest overlap is sorted first but **never silently selected**.
5. **No match:** show a clear **NO 2020 ZCTA MATCH** message. A valid USPS ZIP (especially an individual/PO Box/unique ZIP) may lack a 2020 ZCTA; enter CENTER manually or use GPS/COUNTY as before.
6. **Reverse lookup:** with a county selected, open ZIP and press **COUNTY ZIPs**. See the 2020 ZCTAs that **intersect** the county in pages of 12. These are *not* a certified USPS ZIP inventory or a complete postal town list.
7. Reopen ZIP with network unavailable. If the relationship index and selected county GIS are cached, ZIP lookup and the GIS can run offline. Missing archives fail visibly; no automatic fallback to OSM or an unrelated county.

The ZIP panel is collapsed by default to retain canvas room on the 480×320 handheld. This feature uses exactly the same Mac Rust process and browser micro shell as the existing GPS/manual center workflows. It is a location-selection alternative, **not** GPS.

## Why ZIP → town → county is not a safe mandatory join

A postal ZIP can serve **more than one town or county**, and a town can contain multiple ZIPs. The USPS "preferred city" is a delivery label, not necessarily a legal municipal boundary. Census ZCTAs are generalized statistical areas built from mailing ZIP data and may not exist for every current or special-purpose postal ZIP. Converting ZIP into one assumed town and then one assumed county would invent certainty.

The implementation instead uses the Census **ZCTA → county intersections** directly. A matching ZCTA can map to multiple counties; this remains visible, with operator choice. Once selected, **actual 2025 county GEOID geometry**—rather than a town label—is the prerequisite for loading and clipping county GIS.

The relationship source is **2020 geography**; the county layer and road GIS are **2025**. County borders and identifiers can change between vintages. A 2020 county identifier without a verified 2025 boundary is rejected, not mapped by guessing. ZCTA land-overlap percentage is an *area share*, not a probability that an address belongs to that county.

## Source registry and privacy

| Element | Bound source or operation | Validation |
| --- | --- | --- |
| ZIP→counties, county→ZCTAs | Census 2020 `tab20_zcta520_county20_natl.txt` | Exact pipe-delimited header, 5-digit identifiers, numeric area shares, 9 MiB hard size budget, 80,000 relationship row ceiling |
| Boundary | Existing Census TIGERweb ACS 2025 Counties layer 37 | GEOID-only lookup; Polygon/MultiPolygon, finite coordinates, closed ring checks and cached source revalidation |
| Roads/water | Existing TIGER/Line 2025 county source registry | Fixed HTTPS URL, bounded ZIP decompression, SHP/DBF feature validation |
| Landmarks | Existing TIGER/Line 2025 state POINTLM source | County polygon clipping only |

Outbound requests use **fixed allowlisted Census HTTPS origins** with no user-provided URL, shell interpolation, redirects, or disabled certificate checking. The 2020 relationship dataset is downloaded **only** following an explicit FIND COUNTY or COUNTY ZIPs action. The request does not contain the operator's submitted ZIP, GPS location, private site coordinates or device ID; only the national relationship file is requested. Browser-to-Rust actions remain subject to the loopback same-origin controls. The index persists in the local profile and can be inspected/removed using normal local file controls. No ZIP lookup history is intentionally stored.

Local-only endpoints:
- `POST /api/map/zip`: `{ "zip":"02360", "confirm":true, "cache_only":false }` → ordered intersecting county IDs, source names and land shares. Use `cache_only:true` to prevent downloads.
- `POST /api/map/county-zips`: `{ "geoid":"25023", "offset":0, "confirm":true, "cache_only":false }` → page of 12 associated ZCTAs and total. On an offline cache hit use `cache_only:true`.
- County GIS subsequently follows `POST /api/map/boundary` and `POST /api/map/package`, with the **county GEOID only**.

No selected ZIP can be treated as an exact position, used to compute emergency routing, or presented as a GPS fix. OSM is independently opt-in.

## Verification gates

- Rust unit tests: pipe-delimited header/area validation, multi-county ZIP, sorting without auto-selection, leading-zero ZIP, reverse paging, no-network cache misses and corrupt cached index.
- Rust API tests: same-origin/consent gate, invalid ZIP/GEOID/offset rejection, offline cache-only requests.
- JS tests: collapsed ZIP panel, five-digit validation, no requests on MAP entry, one-county automatic GIS, multi-county explicit choice, reverse paging, offline replay, last-good GIS rollback, no implicit GPS/OSM.
- Mac `scripts/test-map-macos.command --verify` and Rust GitHub Actions 1.90/stable/macOS/Windows.
- Pending operator acceptance: a **live** Census relationship download, 2025 boundary match, input ZIP with no ZCTA, cross-county ZIP, offline launch, and 480×320 touch readability on the actual Mac.

**Primary source:** [2020 Census ZCTA-to-county relationship download](https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/tab20_zcta520_county20_natl.txt) and [record layout](https://www.census.gov/programs-surveys/geography/technical-documentation/records-layout/2020-zcta-record-layout.html). [Census ZCTA guidance](https://www.census.gov/programs-surveys/geography/guidance/geo-areas/zctas.html).
