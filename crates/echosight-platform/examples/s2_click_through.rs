//! Spike S2: runtime click-through toggle while GPUI keeps rendering.
//!
//! GPUI has no mouse-passthrough API on Windows, so `OverlayWindowHandle::set_click_through`
//! takes the `HWND` from `raw_window_handle::HasWindowHandle` and toggles
//! `WS_EX_TRANSPARENT` (+ `WS_EX_LAYERED`) with `SetWindowLongPtrW(GWL_EXSTYLE)`.
//!
//! GPUI creates the window with `WS_EX_NOREDIRECTIONBITMAP` and draws through a
//! DirectComposition swap chain; whether `WS_EX_LAYERED` breaks that is unverified, so
//! the mode is selectable:
//!   * `ECHOSIGHT_SPIKE_LAYERED=1` (default): `WS_EX_TRANSPARENT | WS_EX_LAYERED`
//!   * `ECHOSIGHT_SPIKE_LAYERED=0`:           `WS_EX_TRANSPARENT` only
//!
//! While click-through is on the panel cannot be clicked, so it switches itself back
//! off after `AUTO_OFF` seconds (or press F9 again while the window still has keyboard
//! focus). Non-Windows builds log `unsupported on this platform` (X11/Wayland: spike S7).
//!
//! Manual test script: `docs/plans/gpui-port/spike-s2.md`.

mod common;

use std::time::{Duration, Instant};

use common::{Button, ShellCx, Spike};
use echosight_platform::ClickThroughMode;

const AUTO_OFF: Duration = Duration::from_secs(8);

struct S2 {
    mode: ClickThroughMode,
    enabled: bool,
    toggles: u32,
    auto_off_at: Option<Instant>,
}

impl S2 {
    fn set(&mut self, enabled: bool, cx: &mut ShellCx) {
        let mode = self.mode;
        cx.native_op(&format!("set_click_through({enabled}, {mode:?})"), |h| {
            h.set_click_through_with(enabled, mode)
        });
        self.enabled = enabled;
        self.toggles += 1;
        self.auto_off_at = enabled.then(|| Instant::now() + AUTO_OFF);
    }

    fn toggle(&mut self, cx: &mut ShellCx) {
        self.set(!self.enabled, cx);
    }
}

impl Spike for S2 {
    const TITLE: &'static str = "Echosight spike S2: click-through";
    const HINT: &'static str =
        "F9 = toggle click-through (auto-off after 8 s). Click the game underneath while ON.";

    fn buttons(&self) -> Vec<Button> {
        vec![Button::new(
            "toggle",
            if self.enabled {
                "Click-through: ON (cannot click me)"
            } else {
                "Click-through: OFF (click to enable)"
            },
        )]
    }

    fn status(&self) -> Vec<String> {
        let remaining = self
            .auto_off_at
            .map(|at| at.saturating_duration_since(Instant::now()).as_secs_f32())
            .unwrap_or(0.0);
        vec![
            format!("mode: {:?} (ECHOSIGHT_SPIKE_LAYERED)", self.mode),
            format!(
                "click-through: {}   toggles: {}   auto-off in: {remaining:.1} s",
                if self.enabled { "ON" } else { "OFF" },
                self.toggles
            ),
            "Expected while ON: clicks reach the window below; this panel keeps rendering"
                .to_owned(),
            "with alpha intact; hover highlights stop; no flicker when toggling.".to_owned(),
        ]
    }

    fn on_button(&mut self, id: &str, cx: &mut ShellCx) {
        if id == "toggle" {
            self.toggle(cx);
        }
    }

    fn on_key(&mut self, key: &str, cx: &mut ShellCx) {
        match key {
            "f9" => self.toggle(cx),
            other => cx.log(format!("key: {other}")),
        }
    }

    fn on_tick(&mut self, cx: &mut ShellCx) {
        if let Some(at) = self.auto_off_at
            && Instant::now() >= at
        {
            cx.log("auto-off timer fired");
            self.set(false, cx);
        }
    }
}

fn main() {
    common::run(|_cx| S2 {
        mode: ClickThroughMode::from_env(),
        enabled: false,
        toggles: 0,
        auto_off_at: None,
    });
}
