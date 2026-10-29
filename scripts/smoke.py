#!/usr/bin/env python3
"""Exercise the actual desktop window through IPC and AT-SPI; no global input injection."""
import argparse
import hashlib
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, Gio, GLib


def wait_for(check, description, timeout=12):
    end = time.monotonic() + timeout
    last = None
    while time.monotonic() < end:
        while GLib.MainContext.default().pending():
            GLib.MainContext.default().iteration(False)
        try:
            last = check()
            if last:
                return last
        except GLib.Error as error:
            last = str(error)
        time.sleep(0.08)
    raise AssertionError(f"Timed out: {description}; last result: {last!r}")


def walk(node):
    if node is None:
        return
    yield node
    for i in range(node.get_child_count()):
        yield from walk(node.get_child_at_index(i))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", choices=["wayland", "x11"], default="wayland")
    parser.add_argument("--binary", type=Path, default=Path("target/debug/raycast-linux"))
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    if subprocess.run([binary, "--status"], capture_output=True).returncode == 0:
        raise SystemExit("A launcher is already running. Quit it before this isolated fixture test.")
    bus = Gio.bus_get_sync(Gio.BusType.SESSION)

    def a11y(method, arguments):
        return bus.call_sync("org.a11y.Bus", "/org/a11y/bus", "org.freedesktop.DBus.Properties",
                             method, arguments, None, Gio.DBusCallFlags.NONE, 3000, None)

    enabled = a11y("Get", GLib.Variant("(ss)", ("org.a11y.Status", "IsEnabled"))).unpack()[0]
    report = {"created_at": datetime.now(timezone.utc).isoformat(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "source_sha256": {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in [Path("Cargo.toml"), Path("Cargo.lock"), *sorted(Path("src").glob("*.rs"))]},
              "session_type": os.environ.get("XDG_SESSION_TYPE"),
              "desktop": os.environ.get("XDG_CURRENT_DESKTOP"),
              "backend_requested": args.backend, "checks": [], "limitations": [
        "CLI query updates and AT-SPI result clicking do not prove physical keyboard input, IME composition, or Orca narration.",
        "X11 in this session means XWayland, not a separate native Xorg session.",
        "Reopen focus remains subject to compositor activation policy.",
        "The pinned AccessKit Unix adapter exposes no AT-SPI EditableText interface for the search field."
    ]}

    def passed(name, **evidence):
        report["checks"].append({"name": name, "passed": True, **evidence})

    process = None
    log = None
    try:
        a11y("Set", GLib.Variant("(ssv)", ("org.a11y.Status", "IsEnabled", GLib.Variant("b", True))))
        with tempfile.TemporaryDirectory(prefix="rea-launcher-") as temporary:
            root = Path(temporary)
            data = root / "data"
            applications = data / "applications"
            applications.mkdir(parents=True)
            proof = root / "launched.json"
            helper = root / "launch proof.py"
            helper.write_text("import json, pathlib, sys\npathlib.Path(" + repr(str(proof)) + ").write_text(json.dumps(sys.argv[1:]))\n")

            def desktop(identifier, name, execute, extra=""):
                (applications / f"{identifier}.desktop").write_text(
                    f"[Desktop Entry]\nType=Application\nName={name}\nExec={execute}\nTerminal=false\n{extra}")

            desktop("launcher-verification", "Launcher Verification", f'/usr/bin/python3 "{helper}" %c')
            desktop("launcher-failure", "Launcher Failure", "/usr/bin/true", "Path=/nonexistent/rea-launcher-proof\n")
            desktop("launcher-unicode", "Café 文書", "/usr/bin/true")
            desktop("launcher-hidden", "Launcher Hidden", "/usr/bin/true", "NoDisplay=true\n")
            env = dict(os.environ, XDG_DATA_HOME=str(data), RUST_LOG="warn,raycast_linux=info")
            if args.backend == "x11":
                env.pop("WAYLAND_DISPLAY", None)
                env.pop("WAYLAND_SOCKET", None)
            log_path = Path(".dev") / f"smoke-{args.backend}.log"
            log_path.parent.mkdir(exist_ok=True)
            log = log_path.open("w")
            process = subprocess.Popen([binary], env=env, stdout=log, stderr=subprocess.STDOUT)

            def command(*arguments):
                result = subprocess.run([binary, *arguments], text=True, capture_output=True, timeout=6)
                if result.returncode:
                    raise AssertionError(result.stderr)
                return json.loads(result.stdout)

            def ready():
                if process.poll() is not None:
                    raise AssertionError(f"Launcher exited: {log_path.read_text()[-3000:]}")
                result = subprocess.run([binary, "--status"], text=True, capture_output=True, timeout=6)
                return json.loads(result.stdout) if result.returncode == 0 else None

            initial = wait_for(ready, "launcher startup")
            expected = "Wayland" if args.backend == "wayland" else "X11"
            assert initial["backend"] == expected, initial
            passed("native backend and window", status=initial)
            listed = json.loads(subprocess.check_output([binary, "--list"], env=env))
            assert any(a["id"] == "launcher-verification.desktop" for a in listed)
            assert not any(a["id"] == "launcher-hidden.desktop" for a in listed)
            assert any(a["id"] == "launcher-failure.desktop" for a in listed)
            passed("desktop discovery and NoDisplay handling")
            pid = initial["pid"]
            if args.backend == "x11":
                from x11_window import OwnWindow
                def mapped_window():
                    try:
                        return OwnWindow(pid)
                    except AssertionError:
                        return None
                native = wait_for(mapped_window, "mapped X11 window")
                try:
                    command("--query", "")
                    for character in "launcher":
                        native.key(character)
                    wait_for(lambda: command("--status")["query"] == "launcher", "X11 typed query")
                    first = command("--status")["selected"]
                    native.key("Down")
                    wait_for(lambda: command("--status")["selected"] != first, "X11 Down selection")
                    native.key("Up")
                    wait_for(lambda: command("--status")["selected"] == first, "X11 Up selection")
                    native.key("a", control=True)
                    native.key("BackSpace")
                    wait_for(lambda: command("--status")["query"] == "", "X11 select-all and delete")
                    native.key("Escape")
                    wait_for(lambda: not command("--status")["visible"], "X11 Escape hides window")
                    passed("targeted X11 key events type, navigate, select-all, delete and hide")
                finally:
                    native.close()
                command("--show")

            def find(name):
                desktop_root = Atspi.get_desktop(0)
                desktop_root.clear_cache()
                for i in range(desktop_root.get_child_count()):
                    app = desktop_root.get_child_at_index(i)
                    if app.get_process_id() == pid:
                        app.clear_cache()
                        for node in walk(app):
                            if node.get_name() == name:
                                return node
                return None

            search = wait_for(lambda: find("Search applications"), "AT-SPI search field")
            passed("AT-SPI search role", role=search.get_role_name(), interfaces=search.get_interfaces())
            command("--query", "Café 文書")
            state = wait_for(lambda: (s if (s := command("--status"))["query"] == "Café 文書" and s["matches"] == 1 else None), "Unicode search")
            assert state["selected"] == "launcher-unicode.desktop", state
            visible_result = wait_for(lambda: find("Café 文書"), "Unicode result in actual UI tree")
            bounds = visible_result.get_extents(Atspi.CoordType.WINDOW)
            assert 650 <= bounds.width <= 720, bounds
            passed("result row fills available window width", width=bounds.width)
            passed("Unicode query filters real results and renders accessible result", status=state)
            command("--query", "no application should match this qxz982")
            wait_for(lambda: command("--status")["matches"] == 0, "empty results")
            wait_for(lambda: find("No applications found"), "empty state accessibility")
            passed("empty search state")
            command("--hide")
            hidden = command("--status")
            assert not hidden["visible"] and hidden["pid"] == pid
            command("--toggle")
            reopened = command("--status")
            assert reopened["visible"] and reopened["pid"] == pid
            passed("hide and reopen preserve the process", status=reopened)
            command("--query", "Launcher Failure")
            failure = wait_for(lambda: find("Launcher Failure"), "failure result")
            assert failure.do_action(0)
            error_state = wait_for(lambda: (s if (s := command("--status"))["launch_error"] else None), "launch failure feedback")
            assert error_state["visible"] and not error_state["launching"]
            passed("launch error stays visible and clears busy state", status=error_state)
            command("--query", "Launcher Verification")
            result = wait_for(lambda: find("Launcher Verification"), "launch result")
            assert result.do_action(0)
            wait_for(proof.exists, "launched process proof")
            assert json.loads(proof.read_text()) == ["Launcher Verification"]
            wait_for(lambda: not command("--status")["visible"], "hide after launch")
            passed("AT-SPI result action launches desktop Exec with field code and quoted path")
            command("--show")
            wait_for(lambda: command("--status")["visible"], "show after successful launch")
            if args.backend == "x11":
                proof.unlink()
                native = wait_for(mapped_window, "X11 window after launch")
                try:
                    native.key("Return")
                    wait_for(proof.exists, "X11 Enter launches selected desktop entry")
                    assert json.loads(proof.read_text()) == ["Launcher Verification"]
                    wait_for(lambda: not command("--status")["visible"], "Enter launch hides window")
                    passed("targeted X11 Enter opens the selected application")
                finally:
                    native.close()
            command("--quit")
            assert process.wait(timeout=6) == 0
            passed("explicit shutdown")
            report["passed"] = True
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=6)
        if log is not None:
            log.close()
        a11y("Set", GLib.Variant("(ssv)", ("org.a11y.Status", "IsEnabled", GLib.Variant("b", enabled))))
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
