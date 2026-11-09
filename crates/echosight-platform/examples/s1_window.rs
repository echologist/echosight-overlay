//! Spike S1: frameless, transparent, always-on-top, resizable overlay window.
//!
//! Question answered: can GPUI (zed main, pinned in the workspace Cargo.toml) create the
//! Echosight overlay window on Windows with `WindowOptions` alone?
//!   * `titlebar: None` + `window_decorations: Client`  -> frameless
//!   * `kind: WindowKind::PopUp`                          -> `WS_EX_TOPMOST | WS_EX_TOOLWINDOW`
//!     (above a borderless-fullscreen game, no taskbar button)
//!   * `window_background: Transparent`                  -> per-pixel alpha (DirectComposition)
//!   * `is_resizable: true`, `window_min_size: 450x600`  -> edge resize with a floor
//!   * `window_bounds: Windowed(default_bounds(work_area))` -> top-right, inset 20 px
//!   * header `div().window_control_area(WindowControlArea::Drag)` -> drag by header
//!
//! Open question flagged by the research: `PopUp` uses window style 0 (no
//! `WS_THICKFRAME`), so OS edge-resize may not work. If it does not, the fallback is
//! `WindowKind::Normal` + `SetWindowPos(HWND_TOPMOST)` + `WS_EX_TOOLWINDOW` via the raw
//! handle (see `docs/plans/gpui-port/spike-s1.md`).
//!
//! Manual test script: `docs/plans/gpui-port/spike-s1.md`.

mod common;

use common::{Button, ShellCx, Spike};

struct S1 {
    clicks: u32,
}

impl Spike for S1 {
    const TITLE: &'static str = "Echosight spike S1: window";
    const HINT: &'static str =
        "Drag the header. Resize at the edges (min 450x600). Quit = top right.";

    fn buttons(&self) -> Vec<Button> {
        vec![Button::new("click", format!("Click me ({})", self.clicks))]
    }

    fn status(&self) -> Vec<String> {
        vec![
            "Expected: top-right of the primary work area, 20 px inset, 500x700.".to_owned(),
            "Expected: desktop visible through the rounded corners and the panel.".to_owned(),
            "Expected: stays above a borderless-fullscreen game; no taskbar button.".to_owned(),
        ]
    }

    fn on_button(&mut self, id: &str, cx: &mut ShellCx) {
        if id == "click" {
            self.clicks += 1;
            cx.log(format!("button clicked {} time(s)", self.clicks));
        }
    }

    fn on_key(&mut self, key: &str, cx: &mut ShellCx) {
        cx.log(format!("key: {key}"));
    }
}

fn main() {
    common::run(|_cx| S1 { clicks: 0 });
}
