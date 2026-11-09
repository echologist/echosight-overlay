//! Spike S3: hide / show (without activating) / move / re-assert topmost / focus.
//!
//! GPUI's `Window` has no per-window `hide`/`show` and no runtime `set_position`, so
//! `OverlayWindowHandle` calls Win32 on the raw `HWND`:
//!   * hide            -> `ShowWindow(hwnd, SW_HIDE)`
//!   * show            -> `ShowWindow(hwnd, SW_SHOWNOACTIVATE)` + `SetWindowPos(HWND_TOPMOST, …)`
//!   * set_position    -> `SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE)`
//!   * assert_topmost  -> `SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)`
//!   * focus           -> `SetForegroundWindow(hwnd)`
//!
//! Keys: F10 hide, F11 show (no activate), F12 move to the top-right anchor, F7 focus.
//! With `ECHOSIGHT_SPIKE_CYCLE=1` the window hides and re-shows itself 5 times, 2 s apart,
//! right after start so hide/show can be observed without touching the keyboard.
//!
//! Fallback design if this fails on Windows: close the GPUI window and reopen it with
//! fresh `WindowOptions` (state lives in entities, not in the window), accepting a
//! short flash and a GPU surface re-creation per toggle.
//!
//! Manual test script: `docs/plans/gpui-port/spike-s3.md`.

mod common;

use std::time::{Duration, Instant};

use common::{Button, ShellCx, Spike};

const CYCLE_PERIOD: Duration = Duration::from_secs(2);
const CYCLE_COUNT: u32 = 5;

struct Cycle {
    remaining: u32,
    hidden: bool,
    next_at: Instant,
}

struct S3 {
    hidden: bool,
    hides: u32,
    shows: u32,
    cycle: Option<Cycle>,
}

impl S3 {
    fn hide(&mut self, cx: &mut ShellCx) {
        cx.native_op("hide", |h| h.hide());
        self.hidden = true;
        self.hides += 1;
    }

    fn show(&mut self, cx: &mut ShellCx) {
        cx.native_op("show (no activate) + topmost", |h| h.show());
        self.hidden = false;
        self.shows += 1;
    }

    fn move_to_anchor(&mut self, cx: &mut ShellCx) {
        let (x, y) = cx.anchor_logical();
        // Display bounds are logical pixels; SetWindowPos wants physical ones. The spike
        // uses the window's current scale factor (fine for a single-DPI primary display).
        let scale = cx.window.scale_factor();
        let (px_x, px_y) = (
            (x as f32 * scale).round() as i32,
            (y as f32 * scale).round() as i32,
        );
        cx.native_op(
            &format!("set_position({px_x}, {px_y}) [logical {x},{y}]"),
            |h| h.set_position(px_x, px_y),
        );
    }
}

impl Spike for S3 {
    const TITLE: &'static str = "Echosight spike S3: show / hide / move";
    const HINT: &'static str = "F10 hide, F11 show (no activate), F12 move to top-right, F7 focus. \
                                ECHOSIGHT_SPIKE_CYCLE=1 auto-cycles 5x at start.";

    fn buttons(&self) -> Vec<Button> {
        vec![
            Button::new("hide", "Hide 3 s, then show"),
            Button::new("move", "Move to top-right"),
            Button::new("topmost", "Re-assert topmost"),
            Button::new("focus", "Focus"),
        ]
    }

    fn status(&self) -> Vec<String> {
        let cycle = match &self.cycle {
            Some(c) => format!("auto-cycle: {} left", c.remaining),
            None => "auto-cycle: off".to_owned(),
        };
        vec![
            format!(
                "visible: {}   hides: {}   shows: {}   {cycle}",
                if self.hidden { "no (hidden)" } else { "yes" },
                self.hides,
                self.shows
            ),
            "Expected: re-shown window keeps counters/log, stays topmost, takes no focus;"
                .to_owned(),
            "F12 snaps to the top-right inset 20 px; GPUI logs no errors after re-show.".to_owned(),
        ]
    }

    fn on_button(&mut self, id: &str, cx: &mut ShellCx) {
        match id {
            "hide" => {
                self.hide(cx);
                self.cycle = Some(Cycle {
                    remaining: 1,
                    hidden: true,
                    next_at: Instant::now() + Duration::from_secs(3),
                });
            }
            "move" => self.move_to_anchor(cx),
            "topmost" => cx.native_op("assert_topmost", |h| h.assert_topmost()),
            "focus" => cx.native_op("focus", |h| h.focus()),
            _ => {}
        }
    }

    fn on_key(&mut self, key: &str, cx: &mut ShellCx) {
        match key {
            "f10" => self.hide(cx),
            "f11" => self.show(cx),
            "f12" => self.move_to_anchor(cx),
            "f7" => cx.native_op("focus", |h| h.focus()),
            other => cx.log(format!("key: {other}")),
        }
    }

    fn on_tick(&mut self, cx: &mut ShellCx) {
        let Some(cycle) = self.cycle.as_mut() else {
            return;
        };
        if Instant::now() < cycle.next_at {
            return;
        }
        if cycle.hidden {
            cycle.hidden = false;
            cycle.remaining -= 1;
            cycle.next_at = Instant::now() + CYCLE_PERIOD;
            let done = cycle.remaining == 0;
            self.show(cx);
            if done {
                cx.log("auto-cycle finished");
                self.cycle = None;
            }
        } else {
            cycle.hidden = true;
            cycle.next_at = Instant::now() + CYCLE_PERIOD;
            self.hide(cx);
        }
    }
}

fn main() {
    let cycle = (std::env::var("ECHOSIGHT_SPIKE_CYCLE").as_deref() == Ok("1")).then(|| Cycle {
        remaining: CYCLE_COUNT,
        hidden: false,
        next_at: Instant::now() + CYCLE_PERIOD,
    });
    common::run(move |_cx| S3 {
        hidden: false,
        hides: 0,
        shows: 0,
        cycle,
    });
}
