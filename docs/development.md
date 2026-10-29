# Development status

The first implementation uses GPUI at Zed commit `e5bdd01cd819d6348ae764e69218adfee3d7edab` and Rust 1.97.1. It is a launcher prototype, with application discovery, ranked Unicode search, a virtualized result list, desktop icons with a letter fallback, mouse actions, keyboard actions, and a persistent process controlled through a private Unix socket.

GIO discovers desktop entries and applies visibility rules. The launcher delegates desktop `Exec` interpretation to `gio launch`, which handles quoting and field codes. It resolves icons once during startup. The current icon lookup covers common hicolor, Yaru, and Adwaita application locations and absolute paths; it does not implement the complete icon-theme specification. The catalog is refreshed by restarting the process.

The custom input adapts GPUI's Apache-licensed input example. It retains grapheme movement, UTF-16 input-method offsets, composition ranges, selection, and clipboard operations. This preserves the framework input path but does not prove all of those behaviors on a physical keyboard or input method.

## Verified on this machine

On 2026-10-29, the launcher built and ran with native Wayland and with X11 under the current GNOME Wayland session. Both fixture smoke tests passed application discovery, hidden-entry filtering, Unicode search, empty results, hide/reopen in one process, visible launch errors, desktop launch with a quoted path and field-code expansion, and shutdown. The XWayland run additionally passed targeted synthetic key events for typing, Up/Down selection, Ctrl+A, Backspace, Escape, and Enter to launch. These events do not prove physical keyboard or native Wayland input behavior.

Evidence is saved in [Wayland results](verification/wayland.json) and [XWayland results](verification/xwayland.json), including binary and source hashes. Three search unit tests and Clippy with warnings denied also passed. A [window capture](verification/launcher-xwayland.png) records the corrected result layout on XWayland.

## Code ownership

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Window lifecycle, launcher state, results, actions, command-line entry point |
| `src/catalog.rs` | GIO application discovery, ranking, icon lookup |
| `src/input.rs` | Text input, selection, clipboard, GPUI input-method interface |
| `src/ipc.rs` | Single-instance lock, private socket, bounded requests and replies |
| `scripts/smoke.py` | Repeatable checks against a real desktop window and fixture process |

Linux GPUI's app-level activate/hide methods are unimplemented in the pinned source. This launcher instead activates a native window, removes it to hide, retains the view entity, and creates a new window to show it again. GPUI uses explicit quit mode so closing the last window does not stop the command endpoint. Requests wake the UI through an asynchronous channel; the control path has no polling timer.

## Remaining platform gates

- GNOME can deny focus after reopening. External activation tokens received by the existing process are not forwarded to its native surface. The initial process uses GPUI's startup activation path. GlobalShortcuts portal binding and a GNOME companion have not been implemented.
- The search field exposes its role and value through AccessKit. The pinned Unix adapter does not provide the AT-SPI EditableText interface. Full screen-reader input, selected-result announcements, and Orca navigation are incomplete.
- Physical keyboard input on native Wayland, IME preedit/commit, first-keystroke delivery after a shortcut, clipboard round trips, and mixed-scale displays require dedicated validation. Input rendering also needs long-query horizontal scrolling and a caret blink policy.
- No native Xorg, Plasma, Hyprland, or Sway session has been exercised. XWayland evidence does not replace those environments.
- Application launch focus and startup notification need an activation-aware launch context. The current fixture proves process launch and desktop field expansion.
- Window positioning follows compositor policy. Layer-shell, multi-monitor placement, blur, and visual fidelity comparisons remain open.

The smoke tests cover this launcher milestone. Broader release checks remain open until each behavior and desktop environment has its own execution evidence.
