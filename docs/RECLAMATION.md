# Reclamation Field Kit

Rust-Boy Reclamation Scout now includes a **local-only site register** with manually entered WGS84 positions. The role is pre-pinned with RECLAMATION, SCOUT GIS, GAME LINK, NET ENGINEER, NOTES and HOST STAT.

## Operator workflow

1. Start `./scripts/launch-micro.command`, load **Reclamation Field Kit**, then select **SITES**.
2. Select **NEW SITE** and enter a label, site type, observed condition, priority, latitude/longitude and optional position uncertainty in meters.
3. Save locally. Sites are retained across profile changes and application restarts in `PIPBOY_HOME/reclamation.json` (default `~/.pipboy/reclamation.json`).
4. Change a site's workflow state using **UPDATE**: `logged`, `in_progress`, `needs_assistance` or `completed`. A revision conflict rejects stale edits.
5. Select **GEOJSON** to download an RFC 7946 FeatureCollection of Point features. Coordinates use **[longitude, latitude]** in decimal degrees; attribute properties include type, condition, priority, status, notes, and provenance.

## Constraints and safeguards

- **Manual input only.** Entered positions do not become GPS fixes. Accuracy is an operator estimate (or unknown), not a measured sensor value. The map's separate browser-GPS path is permission-gated.
- A site marked **unsafe** is an operator observation, not a formal inspection or regulatory finding. **Completed** means the operator updated the workflow, not that remediation was independently verified.
- No background network connection, uploading, automatic peer transmission, tracking or outbound map requests are performed by the register.
- Existing task/peer transport is separate; creating a site does not create, assign or transmit a task.
- Storage is **plaintext** and intended for non-sensitive field notes. Do not enter private resident data, credentials, protected health information or detailed security observations.
- The current schema is version 1: up to 256 sites, 512 KiB total, 64-byte names, 512-byte notes, 16 retained status events per site. Invalid, future-schema and oversized source files are rejected without replacement. Writes are OS-locked with atomic replacement.
- GeoJSON export is operator-triggered in the browser; file handling depends on browser download controls. RustGIS import/overlay remains a separately validated path.

## Follow-on gates

- Add survey checklists, evidence references and task cross-linking with deliberate disclosure controls.
- Add explicit, independently validated position capture and freshness/uncertainty states.
- Exercise the map/export path on real RustGIS and test native Raspberry Pi ARM64 performance, touch navigation, interrupted writes and offline restoration.

## Tests

Native unit tests in `crates/pipboy-core/src/reclamation.rs` cover persistence, revision conflicts, invalid coordinates, corrupted/future documents and antimeridian GeoJSON ordering. The Rust CI workflow checks reclamation browser JavaScript syntax. Integration and hardware acceptance are separate from code presence.
