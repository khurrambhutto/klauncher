# Working on klauncher

This repository contains a native Linux launcher built with Rust and GPUI. The executable and application window currently use the name `raycast-linux` and “Raycast Linux”. The previous Tauri implementation is retained in Git history.

## Before changing behavior

Read [development status](docs/development.md) for code ownership, tested behavior, and open platform gates. Read [the README](README.md) for build, run, and validation commands. The first milestone searches and launches applications; other product features remain unimplemented.

Keep discovery and search in `src/catalog.rs`, text editing in `src/input.rs`, and single-instance control in `src/ipc.rs`. Extract additional modules as features grow rather than adding unrelated services to `src/main.rs`.

Keep filesystem scans, subprocess waits, and network work off the GPUI UI thread. Interpret desktop launch commands through GIO rather than a shell or hand-written `Exec` splitting.

## Linux behavior

Treat native Wayland, XWayland, and native Xorg as separate validation targets. Report compositor window activation separately from internal input focus. Preserve input-method composition handling when changing keyboard actions.

The pinned GPUI Linux app-level activate/hide methods are unimplemented. The launcher uses native window activation and removal, retains its view entity, and runs in explicit quit mode. Preserve the persistent process when closing the last window.

Keep global shortcuts, companion installation, and desktop settings changes explicit. Missing desktop capabilities must have visible, accurate behavior. Keep local queries and app data local unless a feature explicitly requires a network service.

## Validation and evidence

Run formatting, relevant tests, and Clippy for Rust changes. Use the live smoke test for changes to window lifecycle, input, result layout, or launch behavior. Quit the launcher before running it; the script temporarily enables accessibility and restores its previous setting. X11 smoke events target only the launcher window identified by its PID.

The files in `docs/verification` record a particular build, with source and binary hashes. Preserve their historical meaning or regenerate them from an actual run. Describe physical keyboard, input-method, accessibility, and compositor coverage precisely.

Publish application source, authored documentation, tests, and required third-party notices. Keep original macOS artifacts, extracted code, local analysis, build outputs, and temporary captures outside the Git tree. See `THIRD_PARTY.md` when editing the adapted input component.
