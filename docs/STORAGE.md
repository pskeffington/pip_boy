# Rust-Boy | Notes storage contract

## Compatibility and limits

The existing `PIPBOY_HOME/notes.json` path, schema version 1, and note fields (`id`, `created_unix_s`, `text`) are retained. Valid existing documents require no conversion. Data remains plaintext.

Reads are limited to 8 MiB plus one byte for size detection. Documents contain at most 10,000 notes, unique positive u64 identifiers, and 1–1024 characters of nonempty text without control characters per note. Unknown schema versions, unknown fields, duplicate/zero IDs and invalid text fail with an error; Rust-Boy does not rewrite or silently normalize them. Future schema changes require an explicit migration and backup design. No general migration/import/export service is implemented yet.

`notes list` and `notes search "query"` do not create directories, lock files or data files. Search trims the query and compares Unicode lowercase substrings; it is not locale-aware collation or full Unicode case folding. Empty/invalid search input is rejected. Search preserves original note text and IDs.

## Write transaction

1. Validate the requested text before opening user data.
2. Create the data directory if needed (mode 0700 for new Unix directories) and open a stable `notes.lock` file (0600 on Unix).
3. Try to take an exclusive advisory OS lock. A competing writer receives a retry error immediately. The CLI does not wait indefinitely or retry silently.
4. Read and validate the entire existing document while holding that lock, then assign the next ID with checked arithmetic.
5. Validate/serialize the replacement within the same size/count limits. Create a uniquely named staging file beside notes.json (0600 on Unix), write it and sync its contents.
6. Persist the staging file by replacing the destination. The existing notes file is never explicitly deleted first. On Unix, sync the containing directory afterwards.
7. Release the lock by closing its handle, including on error paths. Do not delete the stable lock file: concurrent writers must lock the same underlying file.

The implementation uses `fs2` 0.4.3 and `tempfile` 3.27.0; see [L35/L36](LITERATURE_MATRIX.md). Locks coordinate cooperating Rust-Boy writers and do not protect against external editors, same-user interference, hostile filesystems or untrusted linked code. Existing directory permissions and Windows ACLs are not changed.

## Failure and recovery semantics

- Normal staging/write/sync/replacement errors leave the previous notes document intact; owned temporary files are cleaned up when their owner drops.
- A process crash can leave abandoned staging files. Reads use only notes.json and never auto-promote or auto-delete abandoned files. Their presence does not prevent a subsequent valid append.
- OS locks are released when their handles close; a persistent notes.lock filename alone does not mean a writer is active. Do not delete it to resolve contention.
- If directory open/sync fails after replacement, the error explicitly says that notes were committed but durability is uncertain. List notes before retrying to avoid duplicating the append.
- File replacement and flushing do not establish a universal power-loss guarantee, particularly on network/removable filesystems or during initial directory creation. No power-failure, native Windows or native macOS runtime result is claimed by Linux fixtures.
- A corrupted/unsupported canonical notes file is preserved. Restore a known backup or repair a copy explicitly; Rust-Boy does not guess which staged or edited document is authoritative.

## Acceptance evidence

Unit fixtures cover lock contention/release, interrupted staging, failed replacement, orphaned staging, invalid schemas/records, size/count/ID limits, private Unix modes, read-only access and search. `python3 tools/smoke_notes.py` invokes real CLI processes concurrently and verifies every successful note and unique sequential ID, plus search and error preservation. See [VALIDATION.md](VALIDATION.md) for executed platform results.
