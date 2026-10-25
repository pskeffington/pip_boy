# Development roadmap

## G0: Independent foundation (this pass)

- [x] Inventory representative public Pip-Boy projects and license signals.
- [x] Publish local source catalog and reuse policy.
- [x] Native Rust workspace with CLI, Ratatui panels, telemetry, offline notes.
- [x] Define build/test/lint checks (execution status tracked by GitHub Actions).
- [ ] Verify CI green and perform human terminal smoke test on macOS, Linux and Windows.

Exit: reproducible clean build; honest placeholders for unimplemented services.

## G1: Field-computing utility

- [ ] Export telemetry to line-delimited JSON with explicit sampling interval.
- [ ] Nonprivileged hardware capability enumeration and manual adapter enablement.
- [ ] Separate battery / storage / network link metrics, showing unsupported explicitly.
- [ ] Notes search, import/export, schema migrations and local retention controls.
- [ ] Accessibility: keyboard-only controls, low-motion mode, monochrome/contrast profiles.

Exit: proven portability and deterministic fixtures.

## G2: Wearable hardware

- [ ] Raspberry Pi / ARM64 image, tested GPIO encoder/button mapper.
- [ ] Display and backlight profiles; input event replay in simulation.
- [ ] Serial device protocol integration with explicit device selection.
- [ ] Reversible settings changes, watchdog/restore behavior, hardware-in-loop tests.

Exit: device operations gated, reversible and auditable.

## G3: Location, mapping, receiver tools

- [ ] User-supplied GPX/GeoJSON with provenance and offline map datasets whose license permits use.
- [ ] GNSS/NMEA with stale-fix indicators and opt-in location history.
- [ ] Receive-only SDR/audio adapter that respects relevant laws and privacy.
- [ ] Manual BLE/mesh peer selection, authenticated messaging prototype, local-only keys.

Exit: no silent scanning or transmissions; independent threat-model review.

## G4: Ecosystem integrations

- [ ] Versioned module API and stable adapter manifest.
- [ ] Optional external feeds through user-provided, scoped connection settings.
- [ ] Signed builds, SBOM, reproducible release profiles, dependency scanning.
- [ ] Wearable packaging and device regression matrix.

Exit: documented security and IP checks before distribution.

## Non-goals

No unsupported claims of real GPS/radio/mesh functionality, no proprietary game assets, no unauthorized protocol exploitation, no self-updating remote code, and no aggressive data gathering by default.
