# Bounded package process protocol, `RBPK` v1

**Status (2026-10-30):** interoperable request/reply framing, a Rust Music stdio
endpoint, host SDK bridge, supervised child runner, executable hash roster,
deadline/terminate/reap handling, process-exit audio-lease callback, and
loopback shell select/input routes are implemented locally. Production
acceptance remains blocked: the reviewed executable roster is empty, no module
is mounted by live startup, and no release package manifests/binaries are
registered. Exact source commit [`d15247c0ee95663dd3b731db6716a42e6dc4e04f`](https://github.com/pskeffington/pip_boy/commit/d15247c0ee95663dd3b731db6716a42e6dc4e04f)
passed all seven hosted jobs in [run 41124747110](https://github.com/pskeffington/pip_boy/actions/runs/41124747110), including desktop/handheld regressions and native macOS/Windows jobs; target artifact behavior remains unverified.

**Protocol verification (2026-10-30):** `rust-music-ipc` passed all 7 tests
using a temporary Cargo target directory outside its checkout; the Pip-Boy SDK
process protocol passed all 5 targeted tests. Both suites independently lock
the same fixed hello vector and frame limits. This verifies codec compatibility,
not an end-to-end supervised launch of a released Rust Music binary.

The actual service can be smoke-checked without playback using
`python3 tools/check_package_process.py /path/to/reviewed/rust-music-cli`.
The script sends RBPK hello, Activate, and Shutdown frames, checks reply
identity/order and clean exit, and enforces a bounded process timeout. It does
not verify the binary's hash roster pin or exercise audio playback.

The supervised end-to-end check is `python3
tools/test_supervised_music_process.py`. It builds the independent Rust Music
CLI into `/private/tmp`, then runs Pip-Boy's opt-in integration test with that
binary as the child. The host test computes a test-only digest pin, exercises
the actual supervisor handshake/Activate/Shutdown path and verifies the shared
audio lease is released after child exit. This test-only pin does not modify or
approve `catalog/approved_package_executables.json`.

After a separate release review adds both hash pins to that roster, stage the
matching artifact with `python3 tools/install_approved_package.py
<package-id> /path/to/package-artifact`. The artifact directory contains the
roster-relative executable and manifest files. The command copies only those
two files into `$PIPBOY_HOME/packages/<package-id>` (default
`~/.pipboy/packages/<package-id>`), verifies the bytes before and after copy,
and refuses unknown IDs, symlinks, mismatches, and existing installs. It does
not edit the roster, download code, launch the package, or grant capabilities.
When the roster has no entry, staging is denied. Replacement and rollback need
a separate reviewed update flow.

## Purpose and trust boundary

`RBPK` is a local, synchronous, length-prefixed binary protocol for a trusted
Pip-Boy adapter to talk to an independently built module such as Rust Music.
It separates module source/builds and contains malformed messages and crashes.
It is **not** an OS sandbox, privilege boundary, stable Rust ABI, or proof of
package authenticity. Only an explicitly trusted host bootstrap may select an
installed executable. The JSON `UniversalPackageAdapter` vocabulary remains
the host API; `RBPK` is its child-process transport.

The parent owns the real shared audio-focus RAII lease, package permissions,
module identity, foreground selection, and Settings. It sends `Play` only
after host policy and output ownership permit it. The child can report its
local proxy state (`audio_claimed`) but that field never grants host audio
ownership. The parent retains/relinquishes its lease using host lifecycle and
verified child state. No autoplay, unsolicited scans, or background work.

## Framing

Every stdin/stdout record is:

| Bytes | Meaning |
| --- | --- |
| 0..3 | ASCII `RBPK` |
| 4..7 | payload byte length, unsigned big-endian; 1..16384 |
| 8.. | exactly that many payload bytes |

The payload begins with protocol version `u8=1`, message kind (`u8`: request
1, reply 2), request ID (`u64` big-endian, nonzero), package ID (length `u16`
big-endian then UTF-8 bytes, 1..96), then a typed body. Package IDs are
lowercase dotted identifiers (at least three components; each component uses
lowercase ASCII, digits, and hyphen). Text fields are strict UTF-8, reject
control characters, and have per-field byte caps. Unknown tags, trailing bytes,
wrong identity, wrong sequence, partial frames, and unsupported versions fail
closed. Length is checked before allocation; there is at most one outstanding
request per process channel. Clean EOF is only valid before any next-frame
header byte.

The fixed hello payload for `org.rustboy.music`, request ID 1, is the shared
golden vector tested in both repositories:

```text
01 01 00 00 00 00 00 00 00 01 00 11
6f 72 67 2e 72 75 73 74 62 6f 79 2e 6d 75 73 69 63
00 00 01 00 01
```

## Message and field limits

All integers are unsigned big-endian unless stated. The body begins with a
one-byte tag.

| Request tag | Fields / effect |
| --- | --- |
| 0 Hello | minimum and maximum protocol `u16`; first request, ID 1 |
| 1 Activate | viewport width, height `u16`; no scan or playback |
| 2 Suspend | stop output, preserve mounted catalog/selection |
| 3 Resume | viewport width, height `u16`; no autoplay |
| 4 Resize | viewport width, height `u16` |
| 5 ScanLibrary | explicit root string, at most 4096 bytes; policy/consent required |
| 6 Select | track index `u16`; no autoplay |
| 7 Previous / 8 Next | select only; no autoplay |
| 9 Play | explicit user intent and host audio grant required |
| 10 Stop | stop output and release child proxy state |
| 11 Snapshot | page number `u16` |
| 12 Shutdown | stop and acknowledge before exit |

Replies: tag 0 Hello plus selected version; tag 1 Ack; tag 2 View; tag 3 Error
(code `u8`, message up to 256 bytes). A view contains playback enum, optional
selection, total tracks (maximum 2048), page offset, at most 32 entries, each
with index/title (maximum 128 bytes)/container enum, optional validated PCM
format, delivered/optional total frame counters, and a boolean `audio_claimed`.
Views carry no paths, artwork, executable content, or arbitrary UI markup.
The root path is sent only on an explicitly authorized scan request and must
be absolute; it is not echoed back.

## Process and lifecycle rules

- Parent launches a fixed, absolute, reviewed executable directly; no shell,
  command-string interpolation, PATH lookup, or user-controlled arguments.
- stdin/stdout are exclusively protocol streams. Diagnostics go to stderr;
  the host drains or redirects stderr so a full pipe cannot deadlock the child.
- One dedicated host worker owns the channel. UI/render and audio callback
  threads never read/write process pipes or wait for a reply.
- The package worker does not instantiate/open its platform audio sink until
  the explicit Play path. Activate, scan, selection, snapshots and shutdown do
  not create an output device.
- Host supervisor applies bounded deadlines (control transaction: 2 s;
  explicit library scan: 30 s), terminates and reaps a timed-out direct child,
  and faults that module. The SDK `ProcessConnection` remains synchronous;
  `pipboy-core::package_process` owns the worker, deadlines, kill and wait.
- On host shutdown/unmount, send Shutdown, wait at most 2 s, then terminate and
  reap. If stop/cleanup is uncertain while audio may still be active, keep the
  shared audio claim held until the host can prove the process exited.
- Host grants only declared scopes. `media.audio` gates Play/Stop. A distinct
  `media.library.read` grant gates a user-selected local library root; neither
  manifest declarations nor a valid protocol frame are grants.
- Process isolation is fault containment only. Child code executes as the same
  user with the same OS rights unless separately sandboxed by the platform.

## Implementation map and acceptance

- Rust Music codec and stdio endpoint: `rust_music/crates/rust-music-ipc` and
  `rust_music-cli serve`.
- Pip-Boy matching codec and SDK bridge: `rust-boy-adapter-sdk::process_protocol`.
- Host lifecycle/focus broker: `pipboy-core::UniversalAdapterHost` and
  `audio_focus`; the codec bridge is mounted only as a trusted built-in wrapper.
- Protocol acceptance requires the identical golden vector, malformed/oversize
  and sequence tests in both repos, a spawned child hello/activate/shutdown
  test, offline builds, and later an audited supervisor integration test that
  proves timeout termination/reap and no stale audio lease.

The shell can select a mounted adapter or request on-demand mount for a package
whose executable and manifest are both pinned in the compiled host roster and
installed beneath `$PIPBOY_HOME/packages/<package-id>` (default
`~/.pipboy/packages/<package-id>`),
outside the signed application bundle. The host verifies
both hashes and the manifest identity before launch. The shell forwards only
`ui.*` intents through the deny-all scope authority. It renders the adapter's
bounded view model as text and buttons; it does not accept HTML or executable
data. The roster currently has no entries, so the UI reports reference designs
as unavailable. Timeout containment covers
the direct child process only; process-tree containment, an OS sandbox, signed
distribution, runtime installation, and cross-platform production acceptance
remain open. This document does not claim a production-ready external loader.
