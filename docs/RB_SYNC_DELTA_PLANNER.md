# RB sync v2: offline delta planning (experimental)

The pure Rust `pipboy_core::sync_delta` component is a bounded deterministic planner, **not** a fleet synchronization service. It does not open sockets, grant organizational access, accept live receipts, read keys, or persist state. Existing task and Messenger wire protocols remain unchanged.

## Contract

- `EventHeader` contains an origin ID, strictly positive origin-local sequence, and event ID. This minimal header is not a signed envelope.
- `CursorSet` contains the last contiguous, *durably verified* sequence per origin. Callers must not populate it from a peer's unauthenticated claim.
- `plan_batch` sorts by origin and sequence, validates identifiers, rejects gaps and duplicate unseen sequences, limits the batch to 1–64 events and caps tracked origins at 256. It does not inspect payloads, classify data, or authorize transmission.
- `advance_verified` advances exactly one step; its name is a caller precondition, **not** cryptographic receipt verification. An external verifier must authenticate the receipt, confirm correct organization/peer/dataset/event binding, persist the receipt and only then call this function.
- Missing receipts, relay metadata, or a neighboring RB never establish durable delivery. A future transport must retry idempotently, implement backpressure, and fail closed if root, key epoch, trust time or revocation freshness cannot be established.

## Current acceptance

Unit cases exercise ordered selection, limit rejection, missing sequence, duplicate future sequence, advancing without skipping or replaying, and filtering previously committed history. **These are source-level test definitions; field delivery and post-commit CI are separate gates.** Run:

```bash
cargo fmt --all -- --check
cargo test --locked -p pipboy-core sync_delta
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Next integration

1. Add a bounded persisted append-only event ledger under `PIPBOY_HOME` with atomic write/fsync and simulated crash/restart coverage.
2. Bind immutable origin identity and monotonic sequences to signed v2 envelopes. Validate organization, dataset, sender grant and receiver grant independently.
3. Implement authenticated durable receipt verification and anti-replay storage. Prevent cursor advancement on signature mismatch, rejection, uncertainty, or persistence failure.
4. Connect only a synthetic `public_demo` dataset to an explicitly initiated pinned-SSH exchange, with two-host loss/recovery acceptance.
5. Keep enrollment, automatic discovery, background reconciliation, clinical and restricted data disabled until their own security gates pass.

See [fleet synchronization roadmap](RB_SYNC_ROADMAP.md) and [organization identity evidence](../crates/pipboy-core/src/organization_identity.rs).
