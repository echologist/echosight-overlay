//! Shared shell for the Windows spike examples (S1..S4).
//!
//! Each spike implements [`Spike`] and calls [`run`]. The shell owns the GPUI window
//! (frameless, `PopUp`, transparent, resizable, min 450x600, placed top-right of the
//! primary display's work area), the draggable header, a render/tick counter so the
//! tester can see the window keeps painting, a button row, a status block and a small
//! log. Key presses and button clicks are forwarded to the spike.
#![allow(dead_code)] // each example uses a different subset of this module

use std::collections::VecDeque;
use std::time::Duration;

use echosight_core::window::placement::{
    DEFAULT_MARGIN, DEFAULT_OVERLAY_HEIGHT, DEFAULT_OVERLAY_WIDTH, MIN_OVERLAY_HEIGHT,
    MIN_OVERLAY_WIDTH, Rect, anchor_position, default_bounds,
};
use echosight_platform::OverlayWindowHandle;
use gpui::{
    App, Bounds, Context, FocusHandle, KeyDownEvent, Pixels, Window, WindowBackgroundAppearance,
    WindowBounds, WindowControlArea, WindowDecorations, WindowKind, WindowOptions, div, point,
    prelude::*, px, rgb, rgba, size,
};

/// Tick period of the shell timer (drives `Spike::on_tick` and the tick counter).
pub const TICK: Duration = Duration::from_millis(100);
const LOG_LINES: usize = 8;

/// A clickable button in the shell's button row.
pub struct Button {
    pub id: &'static str,
    pub label: String,
}

impl Button {
    pub fn new(id: &'static str, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}

/// What the shell hands to a spike while it handles an event.
pub struct ShellCx<'a> {
    /// Native handle of the spike window (`None` only if GPUI refused to hand one out).
    pub native: Option<&'a OverlayWindowHandle>,
    pub window: &'a mut Window,
    pub app: &'a mut App,
    log: &'a mut VecDeque<String>,
}

impl ShellCx<'_> {
    /// Appends a line to the on-screen log (and to the process log).
    pub fn log(&mut self, line: impl Into<String>) {
        let line = line.into();
        log::info!("{line}");
        self.show(line);
    }

    /// Appends a line to the on-screen log only (for lines already in the process log).
    pub fn show(&mut self, line: impl Into<String>) {
        if self.log.len() == LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(line.into());
    }

    /// Runs a native operation and logs the outcome either way.
    pub fn native_op(
        &mut self,
        what: &str,
        op: impl FnOnce(&OverlayWindowHandle) -> Result<(), echosight_platform::PlatformError>,
    ) {
        let outcome = match self.native {
            Some(handle) => op(handle).map_err(|e| e.to_string()),
            None => Err("no native handle".to_owned()),
        };
        match outcome {
            Ok(()) => self.log(format!("{what}: ok")),
            Err(e) => self.log(format!("{what}: FAILED ({e})")),
        }
    }

    /// Top-right anchor of the primary display's work area in logical pixels, for the
    /// window's current width (mirrors `getOverlayAnchorPosition` in the Electron app).
    pub fn anchor_logical(&self) -> (i32, i32) {
        let width = self.window.bounds().size.width.as_f32().round() as i32;
        anchor_position(work_area(self.app), width, DEFAULT_MARGIN)
    }
}

/// One spike = one implementation of this trait.
pub trait Spike: 'static {
    const TITLE: &'static str;
    /// One-line hint shown under the title (which keys do what).
    const HINT: &'static str;

    fn buttons(&self) -> Vec<Button>;
    /// Extra status lines rendered above the log.
    fn status(&self) -> Vec<String>;
    fn on_button(&mut self, id: &str, cx: &mut ShellCx);
    /// `key` is GPUI's lowercase key name, e.g. `"f9"`.
    fn on_key(&mut self, key: &str, cx: &mut ShellCx);
    /// Called every [`TICK`] on the UI thread. Must not block.
    fn on_tick(&mut self, _cx: &mut ShellCx) {}
}

/// GPUI view wrapping a [`Spike`].
pub struct Shell<S: Spike> {
    spike: S,
    native: Option<OverlayWindowHandle>,
    focus_handle: FocusHandle,
    renders: u64,
    ticks: u64,
    log: VecDeque<String>,
}

impl<S: Spike> Shell<S> {
    fn new(spike: S, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let native = match OverlayWindowHandle::from_window(window) {
            Ok(handle) => Some(handle),
            Err(e) => {
                log::error!("no native window handle: {e}");
                None
            }
        };
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let alive = this
                    .update_in(cx, |shell, window, cx| {
                        shell.ticks += 1;
                        shell.with_shell_cx(window, cx, |spike, scx| spike.on_tick(scx));
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();

        Self {
            spike,
            native,
            focus_handle,
            renders: 0,
            ticks: 0,
            log: VecDeque::with_capacity(LOG_LINES),
        }
    }

    fn with_shell_cx(
        &mut self,
        window: &mut Window,
        app: &mut App,
        f: impl FnOnce(&mut S, &mut ShellCx),
    ) {
        let mut scx = ShellCx {
            native: self.native.as_ref(),
            window,
            app,
            log: &mut self.log,
        };
        f(&mut self.spike, &mut scx);
    }

    fn handle_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.is_held {
            return;
        }
        let key = event.keystroke.key.clone();
        self.with_shell_cx(window, cx, |spike, scx| spike.on_key(&key, scx));
        cx.notify();
    }

    fn handle_button(&mut self, id: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.with_shell_cx(window, cx, |spike, scx| spike.on_button(id, scx));
        cx.notify();
    }
}

impl<S: Spike> Render for Shell<S> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders += 1;
        let bounds = window.bounds();
        let header = div()
            .id("header")
            .window_control_area(WindowControlArea::Drag)
            .flex()
            .items_center()
            .justify_between()
            .h(px(44.))
            .px_4()
            .bg(rgba(0x3b2f6fcc))
            .rounded_t_xl()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(S::TITLE),
            )
            .child(
                div()
                    .id("quit")
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgba(0xff5555aa))
                    .hover(|s| s.bg(rgba(0xff5555ff)))
                    .cursor_pointer()
                    .text_xs()
                    .child("Quit")
                    .on_click(|_, _, cx| cx.quit()),
            );

        let buttons =
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(self.spike.buttons().into_iter().map(|button| {
                    let id = button.id;
                    div()
                        .id(id)
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .bg(rgba(0x5a4fcfcc))
                        .hover(|s| s.bg(rgba(0x7b6ff0ff)))
                        .active(|s| s.bg(rgba(0x9d93ffff)))
                        .cursor_pointer()
                        .text_sm()
                        .child(button.label)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.handle_button(id, window, cx);
                        }))
                }));

        let counters = format!(
            "renders: {}   ticks: {}   bounds: {}x{} @ {},{}   scale: {:.2}",
            self.renders,
            self.ticks,
            bounds.size.width.as_f32().round(),
            bounds.size.height.as_f32().round(),
            bounds.origin.x.as_f32().round(),
            bounds.origin.y.as_f32().round(),
            window.scale_factor(),
        );

        let status = div().flex().flex_col().gap_1().text_sm().children(
            self.spike
                .status()
                .into_iter()
                .map(|line| div().child(line)),
        );

        let log = div()
            .flex()
            .flex_col()
            .flex_1()
            .gap_0p5()
            .p_2()
            .rounded_md()
            .bg(rgba(0x00000066))
            .text_xs()
            .font_family("monospace")
            .children(self.log.iter().cloned().map(|line| div().child(line)));

        div()
            .id("root")
            .track_focus(&self.focus_handle)
            .key_context("Spike")
            .on_key_down(cx.listener(Self::handle_key))
            .size_full()
            .flex()
            .flex_col()
            .rounded_xl()
            .bg(rgba(0x1e1b2ecc))
            .border_1()
            .border_color(rgba(0xa89cffaa))
            .text_color(rgb(0xf4f1ff))
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_3()
                    .p_4()
                    .child(div().text_xs().text_color(rgb(0xbdb6e6)).child(S::HINT))
                    .child(div().text_xs().text_color(rgb(0xbdb6e6)).child(counters))
                    .child(buttons)
                    .child(status)
                    .child(log),
            )
    }
}

/// Work area of the primary display (falls back to its full bounds, then to 1920x1080).
pub fn work_area(cx: &App) -> Rect {
    let bounds: Option<Bounds<Pixels>> = cx.primary_display().map(|d| d.visible_bounds());
    match bounds {
        Some(b) => Rect {
            x: b.origin.x.as_f32().round() as i32,
            y: b.origin.y.as_f32().round() as i32,
            width: b.size.width.as_f32().round() as i32,
            height: b.size.height.as_f32().round() as i32,
        },
        None => {
            log::warn!("no primary display reported; assuming 1920x1080");
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }
        }
    }
}

fn to_gpui_bounds(rect: Rect) -> Bounds<Pixels> {
    Bounds::new(
        point(px(rect.x as f32), px(rect.y as f32)),
        size(px(rect.width as f32), px(rect.height as f32)),
    )
}

/// Window options shared by all spikes (see spike S1 for the rationale of every field).
pub fn window_options(cx: &App) -> WindowOptions {
    let area = work_area(cx);
    let bounds = default_bounds(area);
    log::info!(
        "work area {area:?}; opening {}x{} at {},{}",
        bounds.width,
        bounds.height,
        bounds.x,
        bounds.y
    );
    debug_assert_eq!(
        (bounds.width, bounds.height),
        (DEFAULT_OVERLAY_WIDTH, DEFAULT_OVERLAY_HEIGHT)
    );
    let focus = std::env::var("ECHOSIGHT_SPIKE_FOCUS").as_deref() != Ok("0");
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(to_gpui_bounds(bounds))),
        titlebar: None,
        focus,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: true,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        window_min_size: Some(size(
            px(MIN_OVERLAY_WIDTH as f32),
            px(MIN_OVERLAY_HEIGHT as f32),
        )),
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some("echosight".to_owned()),
        ..Default::default()
    }
}

/// Initialises logging, starts GPUI and opens the spike window.
///
/// `make` runs inside `Application::run` on the main thread, so spikes may create
/// thread-affine things (e.g. the global hotkey manager) there.
pub fn run<S: Spike>(make: impl FnOnce(&mut App) -> S + 'static) {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,zbus=warn,tracing=warn"),
    )
    .init();
    log::info!("{} starting (gpui {})", S::TITLE, gpui_rev());
    gpui_platform::application().run(move |cx: &mut App| {
        let options = window_options(cx);
        let spike = make(cx);
        let opened = cx.open_window(options, |window, cx| {
            cx.new(|cx| Shell::new(spike, window, cx))
        });
        match opened {
            Ok(_) => log::info!("window opened"),
            Err(e) => {
                log::error!("open_window failed: {e:#}");
                cx.quit();
            }
        }
    });
}

/// Pinned GPUI source, for the log header (kept in step with the workspace Cargo.toml).
pub fn gpui_rev() -> &'static str {
    "zed@4ed72e0135aaab74a43914491c915dd6cbd10c67"
}
