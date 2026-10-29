# klauncher

A native Linux application launcher built with Rust and GPUI. The first development milestone searches and opens Linux applications. Extensions, clipboard history, window management, AI, and full visual parity are still future work.

## Run the launcher

The current executable is named `raycast-linux`, and its window title is “Raycast Linux”.

Build with the pinned Rust toolchain and GPUI revision:

```sh
cargo build --locked -j 3
./target/debug/raycast-linux
```

Type an application name, use Up and Down to select a result, and press Enter to open it. Escape closes the window while the process stays running. Ctrl+Q quits the process.

The application uses native Wayland when the process environment names a Wayland display. To test its X11 backend under a Wayland desktop, quit the running instance and run:

```sh
./target/debug/raycast-linux --quit
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET ./target/debug/raycast-linux
```

This tests XWayland on a Wayland desktop. A native Xorg desktop remains a separate test target.

## Control the running process

```sh
./target/debug/raycast-linux --toggle
./target/debug/raycast-linux --query terminal
./target/debug/raycast-linux --hide
./target/debug/raycast-linux --status
./target/debug/raycast-linux --quit
```

Set your desktop's custom shortcut to the absolute path of the executable followed by `--toggle`. No shortcut or desktop configuration is installed automatically. A template desktop entry is in [packaging](packaging/dev.rearay.Launcher.desktop); it expects the executable on `PATH`.

`--list [QUERY]` lists discovered applications as JSON without opening a window. `--status` reports the actual backend, window activation state, internal input focus, current query, and selected result. Internal input focus can be true while the compositor denies window activation. `launch_error` describes the last application launch failure; command errors use `error` and a nonzero exit code.

## Validate changes

```sh
cargo fmt --all -- --check
cargo test --locked -j 3
cargo clippy --locked -j 3 --all-targets -- -D warnings
python3 scripts/smoke.py --backend wayland --output .dev/smoke-wayland.json
python3 scripts/smoke.py --backend x11 --output .dev/smoke-x11.json
```

Quit the launcher before each smoke test. The smoke test needs a live desktop session, Python GObject introspection, the AT-SPI typelib, and XWayland for the X11 run. It creates temporary desktop entries, checks the rendered accessibility tree, and launches a Python fixture that writes a proof file. It temporarily enables `org.a11y.Status.IsEnabled` and restores its previous value in cleanup. The X11 test sends synthetic key events directly to the window identified by the launcher PID. It does not inject global keyboard input or launch your installed applications.

The build currently requires Linux desktop libraries for GIO, GLib, XCB, xkbcommon, font discovery, and graphics. Running a window requires a working Vulkan or OpenGL driver. The build has been exercised on this Ubuntu GNOME machine; distro packages and sandbox packaging have not been validated elsewhere.

See [development status](docs/development.md) for verified behavior and remaining platform work.
