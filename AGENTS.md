# AGENTS.md

## Project Goal

Build a Linux launcher inspired by Tuna for macOS, using Rust + Tauri.

The target is not a visual clone. The important part to mimic is Tuna's product model:

`subject -> action -> optional target`

Every feature should feed this same command pipeline. Fuzzy search, text input, dictation, key-combo mode, scripts, clipboard history, smart links, files, and extensions should all produce subjects, actions, targets, or command results.

## Current Tech Stack

The project was scaffolded with `npm create tauri-app@latest`.

Current app directory:

```text
/home/khurram/Projects/klauncher
```

Important workspace note: the Tauri app has been flattened into `/home/khurram/Projects/klauncher`. Run app commands from the repo root.

Frontend:

- React `^19.1.0`
- TypeScript `~5.8.3`
- Vite `^7.0.4`
- npm package manager
- Tauri JS API `@tauri-apps/api ^2`
- Tauri opener plugin `@tauri-apps/plugin-opener ^2`

Backend:

- Tauri `2`
- Rust edition `2021`
- Crate name: `klauncher`
- Library crate name: `klauncher_lib`
- Current Rust dependencies: `tauri`, `tauri-plugin-opener`, `serde`, `serde_json`

Useful commands:

```bash
cd /home/khurram/Projects/klauncher
npm install
npm run tauri dev
npm run build
npm run tauri build
```

Do not use Android commands for this Linux desktop launcher unless explicitly requested. The scaffold suggested `npm run tauri android init`, but this project is targeting Linux desktop first.

## Tuna Research Snapshot

Research date: 2026-05-09.

Primary sources:

- Tuna homepage: https://tunaformac.com/
- Tuna docs start page: https://tunaformac.com/docs/start-here
- Command model: https://tunaformac.com/docs/how-commands-work
- Fuzzy Mode: https://tunaformac.com/docs/fuzzy-mode
- Text Mode: https://tunaformac.com/docs/text-mode
- Talk Mode: https://tunaformac.com/docs/talk-mode
- Combo Mode / keyboard reference: https://tunaformac.com/docs/keyboard-shortcuts
- Clipboard History and Shelf: https://tunaformac.com/docs/clipboard-history-and-shelf
- Built-in tools: https://tunaformac.com/docs/built-in-tools
- Send Keys: https://tunaformac.com/docs/send-keys
- Smart Links: https://tunaformac.com/docs/smart-links
- Custom scripts: https://tunaformac.com/docs/custom-scripts-and-script-directories
- Hotkeys: https://tunaformac.com/docs/hotkeys-and-activation
- Themes: https://tunaformac.com/docs/themes-and-appearance
- Extensions: https://tunaformac.com/docs/extensions-overview
- Privacy: https://tunaformac.com/docs/privacy-and-local-processing
- Changelog: https://tunaformac.com/changelog

Tuna is a macOS launcher built around Quicksilver-style composable commands. It has four modes: Fuzzy, Text, Talk, and Combo. Tuna renamed Leader Mode to Combo Mode around version `0.60`, but some docs still use the old name.

## Core Product Model

Represent every operation as a command with these parts:

- `Subject`: one or more items, such as an app, file, folder, text, clipboard item, shelf item, current selection, browser page, note, pull request, or script.
- `Action`: a verb that can run against the subject, such as open, reveal, copy, move, rename, transform, run, paste, send keys, send to smart link, append to file, or open with.
- `Target`: optional second object required by some actions, such as an app for `Open With`, a folder for `Move To`, a file for `Append To`, or a smart-link destination.

Implementation implication: do not hard-code each feature as a standalone flow. Build a registry where item providers emit typed subjects, action providers declare compatibility with subject types, and target providers supply valid targets.

## Modes To Mimic

### Fuzzy Mode

Purpose: classic launcher entry point.

Observed behavior:

- User types partial or non-contiguous letters to find apps, files, folders, dynamic catalog items, or commands.
- Example behavior from docs: `saf` opens Safari; `sfi` can still match Safari.
- Results improve through implicit learning: repeated picks for a query rise in rank.
- Explicit aliases can bind custom names to items.
- `Tab` moves from subject pane to action pane.
- Right arrow browses into folders and dynamic catalogs.
- Files/apps are not terminal results; they can be acted on with verbs like reveal, copy, open with, move, rename, browse catalog, etc.

Linux/Tauri notes:

- Use `freedesktop` app discovery from `.desktop` files in `/usr/share/applications`, `/usr/local/share/applications`, and `~/.local/share/applications`.
- Support custom scan roots for files and folders.
- Use a fast local index. Candidate crates: `ignore`, `notify`, `walkdir`, `tantivy` if full-text indexing becomes needed.
- Store learning, aliases, and defaults in SQLite.
- Ranking should combine fuzzy score, recency, frequency, alias exactness, item type boosts, and per-query selection history.
- Phase-one fuzzy matching uses `nucleo-matcher`, isolated in `src-tauri/src/launcher/ranking.rs`. Do not spread Nucleo API calls through providers, commands, or UI code.
- Default app discovery must skip noisy non-app directories inspired by Ulauncher defaults: `/usr/share/locale`, `/usr/share/app-install`, `/usr/share/kservices5`, `/usr/share/kf5`, `/usr/share/kservicetypes5`, `/usr/share/applications/screensavers`, `/usr/share/kde4`, and `/usr/share/mimelnk`.

### Text Mode

Purpose: text becomes the subject.

Observed behavior:

- Can open directly with a global Text Mode hotkey.
- `.` while launcher is open switches current query into Text Mode.
- `'` edits selected item's best text value, such as a file path instead of display name.
- Supports quick calculations, unit/currency conversions, timezone expressions, text transforms, copy/paste, send keys, append to file, send to smart link, and task creation through extensions.
- Math examples include `2+2`, `sqrt(9)`, `sin(pi / 2)`, `log10(1000)`, `10 dkk in usd`, and `10pm UTC tomorrow`.

Linux/Tauri notes:

- Implement text subjects as first-class `ItemKind::Text`.
- For MVP, support math via a Rust expression evaluator and common text transforms.
- Currency/timezone conversion can be later because it introduces network/update policy questions.
- Pasting into current app needs Linux-specific backend work. X11 can use `xdotool`/XTest-style APIs; Wayland requires compositor portals/protocol support and may be limited.

### Talk Mode

Purpose: local-first dictation that feeds Text Mode.

Observed behavior:

- Speech is transcribed locally.
- Transcript becomes text and enters the normal command model.
- Useful for copy, paste, transform, and continued chaining.
- Supports hotkey style and modifier hold-to-talk/toggle flows.
- Changelog mentions Whisper and Parakeet model handling, media pause/resume, model offload, and dictation history.

Linux/Tauri notes:

- Treat dictation as optional post-MVP.
- Candidate local engines: `whisper.cpp`, `faster-whisper` via sidecar, `vosk`, or ONNX-based Parakeet if practical.
- Audio capture can use PipeWire/PulseAudio through Rust crates or a sidecar.
- Keep transcripts local by default.

### Combo Mode

Purpose: known key-chord commands, similar to Leader Key.

Observed behavior:

- Previously called Leader Mode.
- Has customizable bindings and a default root map.
- Can show a cheatsheet with `?`.
- Can be triggered by hotkey or modifier hold.
- Supports group execution and sticky modifier behavior.
- Default examples include keys for Safari, Terminal, Finder, Messages, Notes, Downloads, and Home.

Linux/Tauri notes:

- Model Combo Mode as a trie/keymap where nodes can be commands or groups.
- Provide a cheatsheet overlay rendered by Tauri.
- Default Linux map should use Firefox/browser, terminal, file manager, downloads, home, settings, clipboard, and app search.
- Global key capture is desktop-environment sensitive. X11 is easier; Wayland may need shortcuts portal integration or user-configured compositor shortcuts that call the app CLI.

## Everyday Tools To Mimic

### Clipboard History

Observed behavior:

- Automatic local history for text, links, files, images, and colors.
- Clipboard items become normal command subjects.
- Can be limited or ignored for specific apps.
- Changelog notes SQLite migration for performance, date-based retention, rich text/images, previews, and forget action.

Linux/Tauri notes:

- Store metadata and text in SQLite; store larger binary payloads on disk under app data.
- Support text first. Add images/files after command pipeline is stable.
- App-ignore lists are harder on Wayland because active-window/app identity may be restricted.

### Shelf

Observed behavior:

- Intentional working tray, not automatic history.
- Holds reusable files/snippets and background task results.
- Items can be staged later and used as normal subjects.
- Has behavior settings like stay-on-top and task completion behavior.

Linux/Tauri notes:

- Implement as a persistent collection table plus optional floating Tauri window.
- Background script results should land here.

### Built-In Tools

Observed behavior:

- Includes text utilities, emoji, recent clipboard items, shelf items, smart links, folders/file hierarchies, and current items from active app.
- Built-ins are still just subjects/actions/targets.

Linux/Tauri notes:

- Avoid a separate "tools" architecture. Built-ins should register providers into the same command registry.

### Send Keys / Type Text

Observed behavior:

- `Type Text` sends literal characters.
- `Send Keys` sends a shortcut like `cmd+shift+f`, one shortcut at a time.
- Can send to current app, start from an app then provide shortcut as target, or start from shortcut text and choose receiving app.
- Requires macOS Accessibility permission.

Linux/Tauri notes:

- Use `ctrl`, `alt`, `shift`, `super` terminology rather than macOS `cmd`/`opt`.
- X11 implementation can use `xdotool`, `enigo`, or XTest.
- Wayland implementation is constrained by compositor security. Prefer explicit docs and fallback to copying text plus user paste if synthetic input is blocked.

## Customization To Mimic

### Smart Links

Observed behavior:

- Saved URL templates that accept text input.
- Placeholder examples: `{{input}}` and `{{clipboard}}`.
- Example templates include GitHub search, maps search, and developer docs search.
- Best for predictable web destinations, not full automation.

Linux/Tauri notes:

- Store name, URL template, icon, and optional default browser/open behavior.
- Validate templates and URL-encode placeholder values.
- Smart links should be actions and/or targets for text subjects.

### Custom Scripts

Observed behavior:

- Tuna scans script directories and treats scripts as commands.
- macOS default is `~/Library/Scripts`; Linux should use `~/.local/share/klauncher/scripts` by default.
- Discovers executable scripts and shebang scripts.
- Tuna script metadata headers include:
  - `@tuna.name`
  - `@tuna.title`
  - `@tuna.subtitle`
  - `@tuna.icon`
  - `@tuna.mode`
  - `@tuna.input`
  - `@tuna.output`
- Mode values: `inline`, `background`.
- Input values: `arguments`, `stdin`, `none`.
- Output values: `none`, `text`.
- Metadata comments may start with `#`, `//`, `--`, or `;`.
- Inline scripts can return text into the command flow; background scripts should send output to Shelf.

Linux/Tauri notes:

- Keep the metadata format but rename internally to `@klauncher.*` later if desired. For Tuna compatibility research, preserve the Tuna fields in tests.
- Use `tokio::process` with timeouts and cancellation.
- Never block UI thread while scripts run.
- Treat scripts as untrusted local code. Show paths clearly before execution.

### Hotkeys and Activation

Observed behavior:

- Separate global hotkeys for Fuzzy, Text, Combo, Talk, and custom commands.
- Modifier-style triggers exist for Talk and Combo.
- `.` and `'` are important in-window mode switches.
- Tuna supports Hyper shortcuts if another tool maps a key to Hyper.

Linux/Tauri notes:

- Tauri global shortcuts may work for normal hotkeys, but Wayland support varies.
- Provide a CLI command such as `klauncher open --mode fuzzy` so users can bind compositor shortcuts manually.
- Implement custom command hotkeys only after command serialization is stable.

### Themes and Appearance

Observed behavior:

- Themes change presentation but not the command model.
- Combo Mode can have its own presentation.
- The recommended default is readability and speed, not visual complexity.

Linux/Tauri notes:

- Keep UI skinning separate from command state.
- A theme should not change keyboard behavior or command semantics.

## Extensions

Observed behavior:

- Extensions add catalogs, search roots, actions, object types, and service-backed data.
- First-party examples include Notes, Reminders, Safari, Obsidian, Things, and GitHub.
- Extensions are vocabulary for the same command language, not separate apps.

Linux/Tauri notes:

- Start with built-in providers before designing third-party extension ABI.
- Reasonable Linux-first extension targets:
  - Firefox/Chromium tabs/bookmarks/history where accessible.
  - Obsidian vault notes.
  - GitHub issues/PRs via token.
  - Freedesktop recent files.
  - System power/session actions.
  - Window manager actions where backend supports them.
- Future plugin options: WASM plugins, external JSON-RPC processes, or script metadata.

## Keyboard Behavior To Preserve

Core Tuna shortcuts worth copying conceptually:

- `Enter`: commit current selection/command.
- `Esc`: close launcher or exit text editing.
- `Tab` / `Shift+Tab`: move between subject, action, and target panes.
- Arrow keys: navigate results and browse into/out of catalogs.
- `'`: edit selected item's text value.
- `,`: stage current selection.
- `.`: switch focused pane into Text Mode and keep current query.
- Previous command replay equivalent: apply previous action/target to current subject.
- Quick-look equivalent: preview selected item if possible.
- In-app mode switches: fuzzy, text, talk, combo.
- Rescan library command.
- Context menu for current command and selected result.

Linux adaptation:

- Replace macOS command-symbol shortcuts with `Ctrl`/`Alt`/`Super` conventions.
- Avoid assuming global shortcuts work everywhere on Wayland.

## Privacy and Local-First Rules

Tuna's documented privacy boundary: search queries, clipboard contents, dictation transcripts, and file contents should not be intentionally collected by servers.

For this Linux launcher:

- Keep command input, clipboard history, shelf, aliases, learning, and transcripts local by default.
- Make network features explicit: updates, extension calls, smart links, currency rates, GitHub integration, telemetry if ever added.
- Telemetry should be opt-in only.
- Store sensitive local data under the platform app-data directory with clear retention controls.

## Linux Technical Nitty-Gritty

Major platform constraints:

- X11 allows more launcher behavior: global hotkeys, active window detection, synthetic key events, and window focus are practical.
- Wayland restricts global key capture, active window inspection, and synthetic input by design.
- KDE/GNOME portals can help with file picking and some shortcuts, but support differs by compositor.
- Clipboard access works, but background monitoring behavior can differ between X11 and Wayland.
- App launching should respect `.desktop` files and `gio open`/`xdg-open`.
- File reveal behavior differs by file manager. Use `dbus`/portal/open-folder fallbacks.

Architecture recommendation:

- Rust core owns command model, providers, action compatibility, persistence, indexing, script execution, clipboard watcher, and platform adapters.
- Tauri frontend owns UI rendering, keyboard handling while focused, themes, settings screens, previews, and mode overlays.
- Use a typed command state machine:
  - `Pane::Subject`
  - `Pane::Action`
  - `Pane::Target`
  - `Mode::Fuzzy`
  - `Mode::Text`
  - `Mode::Talk`
  - `Mode::Combo`
- Use provider traits:
  - `ItemProvider`
  - `ActionProvider`
  - `TargetProvider`
  - `PreviewProvider`
  - `Indexer`
- Use action compatibility rules rather than UI-specific branching.

Suggested storage:

- SQLite for items, aliases, learned rankings, command history, clipboard metadata, shelf, settings, smart links, and script metadata cache.
- Filesystem blob store for images, rich clipboard data, thumbnails, and script outputs.

Suggested MVP order:

1. Fuzzy Mode for apps from `.desktop` files.
2. Core command model with subject/action panes and `Open` action.
3. Learning, aliases, and command history.
4. File/folder provider with browse-in-place.
5. Text Mode with calculations and transforms.
6. Smart Links for text subjects.
7. Clipboard history as subjects.
8. Shelf and background task results.
9. Custom scripts with metadata headers.
10. Send Keys / Type Text with X11 support and Wayland caveats.
11. Combo Mode keymap/trie and cheatsheet.
12. Talk Mode local dictation.
13. Extension system.

## Non-Goals For Early Versions

- Do not start with dictation; it has audio/model complexity.
- Do not start with a third-party extension ABI; first validate built-in provider/action architecture.
- Do not promise full Wayland synthetic input or active-window control.
- Do not build a generic macro recorder before the basic `subject -> action -> target` pipeline is reliable.
- Do not make themes affect behavior.

## Working Rule For Future Agents

When adding any new feature, first answer:

- What subject(s) does it create?
- What action(s) does it add?
- What target(s), if any, does it require?
- What item/action types are compatible?
- Is it local-only or networked?
- Does it behave differently on X11 and Wayland?

If the feature cannot be expressed in the command model, reconsider the design before implementing it.

## Code Organization Rules

This project must not grow into a single large `main.rs`. Keep phase one simple, but preserve boundaries from the beginning so later Tuna-like features can be added incrementally.

Recommended Rust/Tauri structure:

```text
klauncher/
  src-tauri/
    src/
      main.rs
      app.rs

      commands/
        mod.rs
        launcher.rs

      core/
        mod.rs
        item.rs
        action.rs
        command_model.rs

      launcher/
        mod.rs
        search.rs
        ranking.rs
        state.rs

      providers/
        mod.rs
        apps.rs

      platform/
        mod.rs
        linux/
          mod.rs
          desktop_entries.rs
          open.rs

      storage/
        mod.rs
        db.rs
        settings.rs

      error.rs
```

For phase one, implement only the files needed for a simple fuzzy app launcher:

```text
main.rs
app.rs
commands/launcher.rs
core/item.rs
launcher/search.rs
launcher/ranking.rs
providers/apps.rs
platform/linux/desktop_entries.rs
platform/linux/open.rs
error.rs
```

Responsibilities:

- `main.rs`: only starts Tauri and calls `app::build()`.
- `app.rs`: registers Tauri commands, shared state, plugins, and setup hooks.
- `commands/`: thin Tauri command layer. Convert frontend calls into Rust service calls, but do not put real business logic here.
- `core/`: pure domain types. No Tauri, no Linux APIs, no filesystem scanning logic.
- `launcher/`: search/session logic. Query in, ranked results out.
- `providers/`: searchable item sources. Start with apps; add files, scripts, clipboard, smart links, and extensions later.
- `platform/`: OS-specific code. Linux `.desktop` parsing, launching apps, global shortcuts, active-window behavior, and Wayland/X11 differences live here.
- `storage/`: SQLite/settings/history later. Do not add persistence until it is actually needed.
- `error.rs`: shared `AppError` and `Result` type.

Phase one data flow:

```text
Frontend query
  -> Tauri command search_apps(query)
  -> providers::apps::AppProvider
  -> platform::linux::desktop_entries scans .desktop files
  -> launcher::search filters/ranks
  -> frontend displays results

Frontend selects app
  -> Tauri command launch_app(app_id)
  -> providers::apps resolves app
  -> platform::linux::open launches it
```

Suggested first core type:

```rust
pub struct LauncherItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub kind: ItemKind,
}

pub enum ItemKind {
    App,
    File,
    Folder,
    Text,
    Clipboard,
    Script,
}
```

Incremental feature order:

1. App fuzzy launcher.
2. Files/folders provider.
3. Actions: open, reveal, copy path, open with.
4. Text mode.
5. Clipboard history.
6. Smart links.
7. Scripts.
8. Combo mode.

Code hygiene rules:

- Do not put `.desktop` parsing in `main.rs`.
- Do not let frontend command handlers own search logic.
- Do not make apps special forever; make apps one provider among many.
- Do not mix UI state and domain state in the same struct.
- Do not design a third-party plugin system before the provider/action model works.
- Do not fight Wayland global-shortcut and synthetic-input limitations in phase one.

Every new feature must enter through one of these buckets: `core`, `launcher`, `providers`, `platform`, `commands`, or `storage`. If it is not clear where a feature belongs, stop before coding and clarify the boundary.
