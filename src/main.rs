mod catalog;
mod input;
mod ipc;

use anyhow::{Context as _, Result, bail};
use catalog::Application;
use gpui::{
    AccessibleAction, AnyElement, App, Bounds, Context, Entity, Focusable, Global, KeyBinding,
    Render, Role, ScrollStrategy, Subscription, UniformListScrollHandle, Window, WindowBounds,
    WindowDecorations, WindowHandle, WindowOptions, actions, div, img, prelude::*, px, rgb, size,
    uniform_list,
};
use input::TextInput;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

actions!(launcher, [Next, Previous, Launch, Dismiss, Quit]);

struct Runtime {
    view: Entity<Launcher>,
    window: Option<WindowHandle<Launcher>>,
}
impl Global for Runtime {}

struct Launcher {
    input: Entity<TextInput>,
    apps: Arc<Vec<Application>>,
    icons: Vec<Option<PathBuf>>,
    results: Vec<usize>,
    query: String,
    selected: usize,
    scroll: UniformListScrollHandle,
    error: Option<String>,
    launching: bool,
    render_count: u64,
    _input_subscription: Subscription,
    activation_subscription: Option<Subscription>,
}

impl Launcher {
    fn new(
        apps: Arc<Vec<Application>>,
        icons: Vec<Option<PathBuf>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(TextInput::new);
        let subscription = cx.observe(&input, |this, input, cx| {
            let query = input.read(cx).text().to_owned();
            if this.query != query {
                this.update_results(query, cx);
            }
        });
        Self {
            input,
            results: (0..apps.len()).collect(),
            apps,
            icons,
            query: String::new(),
            selected: 0,
            scroll: UniformListScrollHandle::new(),
            error: None,
            launching: false,
            render_count: 0,
            _input_subscription: subscription,
            activation_subscription: None,
        }
    }

    fn update_results(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query;
        self.results = catalog::search(&self.apps, &self.query);
        self.selected = 0;
        self.error = None;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    fn attach(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.set_window_title("Raycast Linux");
        window.focus(&self.input.focus_handle(cx), cx);
        self.activation_subscription =
            Some(cx.observe_window_activation(window, |this, window, cx| {
                if window.is_window_active() {
                    window.focus(&this.input.focus_handle(cx), cx);
                }
            }));
        window.on_window_should_close(cx, |_, cx| {
            cx.update_global::<Runtime, _>(|runtime, _| runtime.window = None);
            true
        });
    }

    fn next(&mut self, _: &Next, _: &mut Window, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() {
            cx.propagate();
            return;
        }
        self.selected = (self.selected + 1).min(self.results.len().saturating_sub(1));
        self.scroll
            .scroll_to_item(self.selected, ScrollStrategy::Bottom);
        cx.notify();
    }

    fn previous(&mut self, _: &Previous, _: &mut Window, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() {
            cx.propagate();
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.scroll
            .scroll_to_item(self.selected, ScrollStrategy::Top);
        cx.notify();
    }

    fn dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() {
            cx.propagate();
            return;
        }
        window.remove_window();
        cx.update_global::<Runtime, _>(|runtime, _| runtime.window = None);
    }

    fn launch(&mut self, _: &Launch, _: &mut Window, cx: &mut Context<Self>) {
        if self.launching || self.input.read(cx).is_composing() {
            cx.propagate();
            return;
        }
        let Some(index) = self.results.get(self.selected) else {
            return;
        };
        let app = self.apps[*index].clone();
        self.launching = true;
        self.error = None;
        cx.notify();
        let task = cx.background_executor().spawn(async move {
            // GIO interprets desktop Exec field codes, quoting, terminal and D-Bus activation.
            let output = std::process::Command::new("gio")
                .arg("launch")
                .arg(&app.desktop_file)
                .output()
                .context("Could not start gio launch")?;
            if !output.status.success() {
                bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
            }
            Ok::<_, anyhow::Error>(())
        });
        cx.spawn(async move |view, cx| {
            let result = task.await;
            let succeeded = result.is_ok();
            let _ = view.update(cx, |this, cx| {
                this.launching = false;
                this.error = result.err().map(|error| error.to_string());
                cx.notify();
            });
            if succeeded {
                cx.update(hide_window);
            }
        })
        .detach();
    }

    fn row(&self, position: usize, cx: &mut Context<Self>) -> AnyElement {
        let index = self.results[position];
        let app = &self.apps[index];
        let selected = position == self.selected;
        let icon = if let Some(path) = &self.icons[index] {
            img(path.clone()).size(px(30.)).into_any_element()
        } else {
            div()
                .size(px(30.))
                .rounded(px(7.))
                .bg(rgb(0x41434c))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(16.))
                .child(app.name.chars().next().unwrap_or('?').to_string())
                .into_any_element()
        };
        let weak = cx.weak_entity();
        div()
            .id(gpui::SharedString::from(app.id.clone()))
            .role(Role::ListBoxOption)
            .accessibility_id(format!("app-{}", app.id))
            .aria_label(app.name.clone())
            .aria_selected(selected)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.selected = position;
                this.launch(&Launch, window, cx);
            }))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                let _ = weak.update(cx, |this, cx| {
                    this.selected = position;
                    this.launch(&Launch, window, cx);
                });
            })
            .h(px(54.))
            .w_full()
            .px(px(12.))
            .rounded(px(8.))
            .flex()
            .items_center()
            .gap(px(12.))
            .cursor_pointer()
            .when(selected, |row| row.bg(rgb(0x3a3c43)))
            .child(icon)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(div().text_size(px(14.)).truncate().child(app.name.clone()))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x9b9da5))
                            .truncate()
                            .child(if app.description.is_empty() {
                                app.executable.clone()
                            } else {
                                app.description.clone()
                            }),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x9698a0))
                    .child("Application"),
            )
            .into_any_element()
    }
}

impl Render for Launcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_count += 1;
        div()
            .id("launcher")
            .key_context("Launcher")
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(0x232428))
            .text_color(rgb(0xf2f2f4))
            .font_family("Ubuntu")
            .border_1()
            .border_color(rgb(0x484a50))
            .rounded(px(12.))
            .on_action(cx.listener(Self::next))
            .on_action(cx.listener(Self::previous))
            .on_action(cx.listener(Self::launch))
            .on_action(cx.listener(Self::dismiss))
            .child(
                div()
                    .h(px(76.))
                    .flex_shrink_0()
                    .px(px(20.))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(0x3b3d43))
                    .child(self.input.clone()),
            )
            .child(
                div()
                    .h(px(34.))
                    .flex_shrink_0()
                    .px(px(20.))
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(rgb(0xa6a7af))
                    .child("Applications"),
            )
            .child(if self.results.is_empty() {
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("empty-results")
                            .role(Role::Label)
                            .aria_value("No applications found")
                            .text_size(px(16.))
                            .child("No applications found"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0x989aa4))
                            .child("Try a different name or keyword"),
                    )
                    .into_any_element()
            } else {
                uniform_list(
                    "applications",
                    self.results.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                        range.map(|position| this.row(position, cx)).collect()
                    }),
                )
                .track_scroll(&self.scroll)
                .px(px(8.))
                .flex_1()
                .into_any_element()
            })
            .when_some(self.error.clone(), |root, error| {
                root.child(
                    div()
                        .px(px(20.))
                        .py(px(8.))
                        .text_size(px(12.))
                        .text_color(rgb(0xffb0aa))
                        .child(error),
                )
            })
            .child(
                div()
                    .h(px(42.))
                    .flex_shrink_0()
                    .px(px(18.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(0x3b3d43))
                    .text_size(px(11.))
                    .child(
                        div()
                            .text_color(rgb(0x9698a0))
                            .child("↑ ↓  Navigate     esc  Close"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(if self.launching {
                                "Opening…"
                            } else {
                                "Open Application"
                            })
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(rgb(0x3c3e45))
                                    .child("↵"),
                            ),
                    ),
            )
    }
}

fn hide_window(cx: &mut App) {
    let window = cx.update_global::<Runtime, _>(|runtime, _| runtime.window.take());
    if let Some(window) = window {
        let _ = window.update(cx, |_, window, _| window.remove_window());
    }
}

fn show_window(cx: &mut App) -> Result<()> {
    if let Some(window) = cx.global::<Runtime>().window
        && window
            .update(cx, |view, window, cx| {
                window.activate_window();
                window.focus(&view.input.focus_handle(cx), cx);
            })
            .is_ok()
    {
        return Ok(());
    }
    let view = cx.global::<Runtime>().view.clone();
    let bounds = Bounds::centered(None, size(px(720.), px(500.)), cx);
    let window = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some("dev.rearay.Launcher".into()),
            is_resizable: false,
            ..Default::default()
        },
        |window, cx| {
            view.update(cx, |view, cx| view.attach(window, cx));
            view
        },
    )?;
    cx.update_global::<Runtime, _>(|runtime, _| runtime.window = Some(window));
    Ok(())
}

fn status(cx: &mut App) -> Value {
    let window = cx.global::<Runtime>().window;
    let (visible, active, focused) = window
        .and_then(|window| {
            window
                .update(cx, |view, window, cx| {
                    (
                        true,
                        window.is_window_active(),
                        view.input.focus_handle(cx).is_focused(window),
                    )
                })
                .ok()
        })
        .unwrap_or((false, false, false));
    let view = cx.global::<Runtime>().view.read(cx);
    json!({"pid":std::process::id(), "backend":cx.compositor_name(), "visible":visible,
        "active":active,"input_focused":focused,"query":view.query,
        "applications":view.apps.len(),"matches":view.results.len(),
        "selected":view.results.get(view.selected).map(|index| &view.apps[*index].id),
        "launching":view.launching,"launch_error":view.error,"render_count":view.render_count})
}

fn handle_command(command: ipc::Command, cx: &mut App) -> Value {
    match command {
        ipc::Command::Show {
            query,
            activation_token,
        } => {
            if activation_token.is_some() {
                log::debug!(
                    "External activation token forwarding is not available through this GPUI API"
                );
            }
            if let Err(error) = show_window(cx) {
                return json!({"error":error.to_string()});
            }
            if let (Some(query), Some(window)) = (query, cx.global::<Runtime>().window) {
                let _ = window.update(cx, |view, window, cx| {
                    view.input
                        .update(cx, |input, cx| input.set_text(&query, window, cx));
                    view.update_results(query, cx);
                });
            }
        }
        ipc::Command::Toggle => {
            if cx.global::<Runtime>().window.is_some() {
                hide_window(cx);
            } else if let Err(error) = show_window(cx) {
                return json!({"error":error.to_string()});
            }
        }
        ipc::Command::Hide => hide_window(cx),
        ipc::Command::Status => {}
        ipc::Command::Quit => {
            cx.quit();
            return json!({"quitting":true});
        }
    }
    status(cx)
}

fn parse_args() -> Result<Option<ipc::Command>> {
    let mut args = std::env::args().skip(1);
    let mut command = ipc::Command::Show {
        query: None,
        activation_token: std::env::var("XDG_ACTIVATION_TOKEN").ok(),
    };
    while let Some(arg) = args.next() {
        command = match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Raycast Linux — native application launcher\n\nUsage: raycast-linux [--show | --toggle | --hide | --status | --quit]\n       raycast-linux --query TEXT\n       raycast-linux --list [QUERY]\n\nKeyboard: type to search, ↑/↓ to select, Enter to open, Esc to hide, Ctrl+Q to quit.\nBind --toggle to your desktop's custom shortcut. No global shortcut is installed automatically."
                );
                return Ok(None);
            }
            "--list" => {
                let apps = catalog::discover();
                let query = args.collect::<Vec<_>>().join(" ");
                let matches: Vec<_> = catalog::search(&apps, &query)
                    .into_iter()
                    .map(|i| &apps[i])
                    .collect();
                println!("{}", serde_json::to_string_pretty(&matches)?);
                return Ok(None);
            }
            "--show" => ipc::Command::Show {
                query: None,
                activation_token: std::env::var("XDG_ACTIVATION_TOKEN").ok(),
            },
            "--toggle" => ipc::Command::Toggle,
            "--hide" => ipc::Command::Hide,
            "--status" => ipc::Command::Status,
            "--quit" => ipc::Command::Quit,
            "--query" => ipc::Command::Show {
                query: Some(args.next().context("--query needs text")?),
                activation_token: std::env::var("XDG_ACTIVATION_TOKEN").ok(),
            },
            _ => bail!("Unknown argument: {arg}"),
        };
    }
    Ok(Some(command))
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,raycast_linux=info"),
    )
    .init();
    let Some(command) = parse_args()? else {
        return Ok(());
    };
    let server = match ipc::start(&command)? {
        ipc::Instance::Forwarded(response) => {
            println!("{}", serde_json::to_string_pretty(&response)?);
            if let Some(error) = response.get("error").and_then(Value::as_str) {
                bail!("{error}");
            }
            return Ok(());
        }
        ipc::Instance::Primary(server) => server,
    };
    let requests = server.requests.clone();
    let apps = Arc::new(catalog::discover());
    let icons = apps
        .iter()
        .map(|app| catalog::resolve_icon(app.icon.as_deref()))
        .collect();
    log::info!("Discovered {} applications", apps.len());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        cx.set_app_identity("dev.rearay.Launcher", "Raycast Linux");
        input::bind_keys(cx);
        cx.bind_keys([
            KeyBinding::new("down", Next, Some("Launcher")),
            KeyBinding::new("up", Previous, Some("Launcher")),
            KeyBinding::new("enter", Launch, Some("Launcher")),
            KeyBinding::new("escape", Dismiss, Some("Launcher")),
            KeyBinding::new("ctrl-q", Quit, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        let view = cx.new(|cx| Launcher::new(apps, icons, cx));
        cx.set_global(Runtime { view, window: None });
        let response = handle_command(command, cx);
        if let Some(error) = response.get("error").and_then(Value::as_str) {
            log::error!("{error}");
            cx.quit();
        }
        cx.spawn(async move |cx| {
            while let Ok(request) = requests.recv().await {
                let response = cx.update(|cx| handle_command(request.command, cx));
                let _ = request.reply.send(response);
            }
        })
        .detach();
    });
    drop(server);
    Ok(())
}
