# Rust-Boy | Notes storage contract

## Compatibility and limits

The existing `PIPBOY_HOME/notes.json` path, schema version 1, and note fields (`id`, `created_unix_s`, `text`) are retained. Valid existing documents require no conversion. Data remains plaintext.

Reads are limited to 8 MiB plus one byte for size detection. Documents contain at most 10,000 notes, unique positive u64 identifiers, and 1–1024 characters of nonempty text without control characters per note. Unknown schema versions, unknown fields, duplicate/zero IDs and invalid text fail with an error; Rust-Boy does not rewrite or silently normalize them. Future schema changes require an explicit migration and backup design. Notes export/import uses schema 1; a general migration service for other collections remains planned.

`notes list` and `notes search "query"` do not create directories, lock files or data files. Search trims the query and compares Unicode lowercase substrings; it is not locale-aware collation or full Unicode case folding. Empty/invalid search input is rejected. Search preserves original note text and IDs.

## Explicit backup and import

`pipboy notes export` writes one validated schema-1 JSON document to stdout. It does not create an export file or user directory, and validation finishes before any output is emitted. Output/write failures propagate without changing notes. Redirect stdout to a separate backup file, never to the live notes.json path (shell redirection happens before Rust-Boy can validate it).

`pipboy notes import FILE` reads only the named local file with the same 8 MiB/schema limits. Invalid input is rejected before any writer lock/directory is created. Empty valid imports do not create data. For nonempty imports, the local read/merge/save transaction uses the same exclusive lock and staged replacement as appends. Existing notes are never edited or deleted. Imported timestamps and text are preserved; newly appended records receive fresh IDs above the current local maximum.

Matching uses exact timestamp plus exact text. For repeated identical notes, the merged document keeps the larger occurrence count from the current and incoming collections. For example, two local copies and three backup copies yield three copies, not one or five. This supports repeat-safe re-import without discarding multiplicity, but does not infer globally unique note identity across devices. A future cross-device schema would need an explicit stable identity design.

The command reports imported, skipped and total counts. Re-importing the same backup does not append duplicates or rewrite the canonical file. If the combined record count, encoded size or local ID space would overflow, the whole import fails before replacement; no subset is committed. Future schema versions are preserved and rejected rather than guessed or auto-converted.

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

Unit fixtures cover lock contention/release, interrupted staging, failed replacement, orphaned staging, invalid schemas/records, size/count/ID limits, private Unix modes, read-only access and search. `python3 tools/smoke_notes.py` invokes real CLI processes concurrently and verifies every successful note and unique sequential ID, plus search, export/import/re-import and error preservation. See [VALIDATION.md](VALIDATION.md) for executed platform results.

## Messenger encrypted queue option

Messenger defaults to a bounded plaintext journal. Before any message exists,
`pipboy peer message-key-init --confirm` enrolls age encryption and an OS-backed
queue identity. `messages.recipient` contains only the public recipient;
`messages.json` then contains binary age ciphertext. Existing journals are never
automatically migrated. Other stores remain plaintext. Losing the OS credential
can make the queue unrecoverable; backup/rotation and independent review remain
release gates. See [full contract and limitations](MESSENGER_BUILD_PASSES.md).

## Emergency state

The independent plaintext `aid-state.json` latch and configuration survive profile changes and restarts. Its local incident counter must be preserved. Aid messages use the Messenger journal and its configured encryption mode. See [emergency storage and recovery](MEDIC_EMERGENCY.md).
