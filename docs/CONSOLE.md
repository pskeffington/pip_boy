# Rust-Boy | Console controls and layout

The TUI supports keyboard navigation with static presentation and explicit status text. These changes do not establish WCAG compliance, screen-reader compatibility or tested support for every terminal emulator.

| Key | Action |
| --- | --- |
| Left/Right, Tab, Shift+Tab | Previous/next panel; selections are retained per panel |
| Up/Down | Select a source/note, or move text one line on other panels |
| Page Up/Page Down | Scroll the selected detail text by a viewport, with one-line overlap |
| Home/End | First/last source or note; top/bottom text on other panels |
| R | Reload notes while in NOTES |
| P | Revoke/resume local host telemetry and clear its history on pause |
| Q, Escape | Exit and restore the terminal |

## Collections and freshness

STAT shows explicit field quality beside CPU, cores, memory, OS/architecture and uptime. CPU starts as unknown until a full sampling window is available; startup and resume do not sleep to prime it. Expiry marks measured fields stale, while unknown/invalid fields keep no numeric value. P clears both detailed readings and raw history. See [host telemetry](HOST_TELEMETRY.md) for units, timing and CLI compatibility.

LIBRARY and NOTES show an item selector above a scrollable detail pane. Every stored note is reachable, most recently stored first; the old 12-note display cutoff is removed. The selected note's entire text can be paged, including content wrapping below a narrow viewport. LIBRARY details show repository, license status, reference-only disposition, URL and summary. Selecting a URL does not open it or initiate network access.

The selected position is marked by `>` and bold text, not color alone. Item and text-position counters indicate the current location. Selecting another item resets its detail scroll; resizing clamps the current scroll to the resized text bounds. Empty collections and planned modules use explicit text.

Notes load when entering NOTES or pressing R. Redraws reuse the cached list, so external changes appear after R or re-entry. Corrupt/unsupported notes still produce an error exit and restore the terminal; the TUI does not repair or rewrite them.

## Terminal size

At 70 columns or wider, all panel tabs are displayed. From 40 columns, a compact header names the current panel and position. The supported compact layout requires at least 40 columns and 12 rows, with visible controls and scrollable details. At smaller dimensions the console shows resize/exit guidance and retains navigation state for recovery when enlarged. Exit keys remain active even when the screen cannot display the whole guidance message.

Rendering and navigation live in console.rs and perform no file, collector or terminal-mode operations. tui.rs handles those operations and maps key events to navigation actions. Scroll bounds use the same Ratatui word wrapper as rendering; the locked 0.29.0 dependency's `unstable-rendered-line-info` feature must be reviewed on upgrades (L05/L37).

## Executed checks

Memory-backed fixtures exercise all six panels at tiny, compact and standard sizes, empty collections, selection bounds, older notes, Unicode wrapped text tails and resize clamping. Linux and native macOS PTY smoke checks exercise note reload, End selection, shrink/grow recovery, pause/resume and normal/error terminal restoration, with macOS CI passing at [run 40162059963](https://github.com/pskeffington/pip_boy/actions/runs/40162059963). A new first-party Windows ConPTY regression now uses the OS pseudoconsole API to exercise launch, P pause/resume, shrink/grow, Q exit, alternate-screen restoration and corrupt-notes exit, without third-party runner actions. **The Windows ConPTY test passed** in [GitHub Actions run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734), including clean exit and corrupt-notes restoration. Windows ConPTY is a terminal test, not an OS device-permission broker. Contrast and assistive-technology acceptance remain open; see [validation evidence](VALIDATION.md).
