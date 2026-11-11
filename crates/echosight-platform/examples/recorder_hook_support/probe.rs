//! Minimal own-window GPUI probe. Visible diagnostics are counters, static labels
//! and one explicitly captured own-window numeric snapshot (never text/history).
//! No user text, raw keyboard bytes, settings or successful chords saved to disk.
use crate::{
    decode::{RecorderDecodeError, decode},
    native,
    policy::{Batch, Command},
};
use echosight_core::settings::HotkeyRecorder;
use gpui::{
    App, Bounds, Context, FocusHandle, KeyBinding, Window, WindowBounds, WindowOptions, actions,
    div, prelude::*, px, rgb, size,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
actions!(s9, [Canary]);
struct Probe {
    focus: FocusHandle,
    recorder: HotkeyRecorder,
    arm: u64,
    last_sequence: u64,
    records: u64,
    losses: u64,
    stale: u64,
    snapshot_errors: u64,
    unsupported: u64,
    gpui_keys: u64,
    actions: u64,
    fail_snapshot: bool,
    message: String,
    last_snapshot: String,
}
fn install(window: &Window) -> Result<(), String> {
    let handle = HasWindowHandle::window_handle(window).map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return Err("Not a Win32 HWND".into());
    };
    native::install(HWND(raw.hwnd.get() as *mut _))
}
impl Probe {
    fn new(window: &mut Window, cx: &mut Context<Self>, start: bool) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let installed = install(window);
        let mut this = Self {
            focus,
            recorder: HotkeyRecorder::default(),
            arm: 0,
            last_sequence: 0,
            records: 0,
            losses: 0,
            stale: 0,
            snapshot_errors: 0,
            unsupported: 0,
            gpui_keys: 0,
            actions: 0,
            fail_snapshot: false,
            last_snapshot: "No own-window snapshot yet".into(),
            message: installed.as_ref().map_or_else(
                |e| e.clone(),
                |_| "Installed INACTIVE. Explicit Start required. NO-GO for integration.".into(),
            ),
        };
        if start && installed.is_ok() {
            this.command(Command::Begin);
        }
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_window, cx| {
            weak.update(cx, |this, cx| {
                let ok = this.close();
                cx.notify();
                ok
            })
            .unwrap_or_else(|_| native::shutdown(false))
        });
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        this.apply(native::drain());
                        if !native::status().0 && this.recorder.is_recording() {
                            // Observer already stopped capture synchronously; apply
                            // copied old-arm records BEFORE changing the core state.
                            let _ = this.recorder.stop();
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        this
    }
    fn apply(&mut self, batch: Batch) {
        for record in batch.records.into_iter().flatten() {
            if record.sequence <= self.last_sequence || record.arm != self.arm {
                self.stale = self.stale.saturating_add(1);
                self.message = "ORDERING VIOLATION: NO-GO".into();
            }
            self.last_sequence = record.sequence;
            self.records = self.records.saturating_add(1);
            // Only the latest explicit-own-window numeric diagnostic is retained
            // on screen, never text/keyboard-state history or a disk/log sink.
            let shape = record.raw.shape();
            self.last_snapshot = format!(
                "Own snapshot VK={:02x} scan={:02x} ext={} repeat={} previous={} up={} location={:?} msgtime={} HKL={:x}; L/R Shift={:02x}/{:02x} Ctrl={:02x}/{:02x} Alt={:02x}/{:02x}",
                record.raw.vk,
                shape.scan,
                shape.extended,
                shape.repeat_count,
                shape.previously_down,
                shape.transition_up,
                shape.location,
                record.raw.time,
                record.raw.hkl,
                record.raw.keyboard[0xa0],
                record.raw.keyboard[0xa1],
                record.raw.keyboard[0xa2],
                record.raw.keyboard[0xa3],
                record.raw.keyboard[0xa4],
                record.raw.keyboard[0xa5]
            );
            // A copied snapshot never becomes a fabricated DOM event.
            // The full raw record is dropped at the end of this iteration.
            match decode(&record.raw) {
                Err(RecorderDecodeError::SnapshotFailed(_)) => {
                    self.snapshot_errors = self.snapshot_errors.saturating_add(1)
                }
                Err(RecorderDecodeError::UnsupportedTranslation(_)) => {
                    self.unsupported = self.unsupported.saturating_add(1)
                }
                Err(RecorderDecodeError::UnrepresentableKey(_)) => {}
                Ok(_) => {
                    self.message =
                        "Unexpected decoder success: adapter integration is not implemented".into()
                }
            }
        }
        if let Some(loss) = batch.loss {
            self.losses = self.losses.saturating_add(loss.count);
            self.last_sequence = loss
                .first_sequence
                .saturating_add(loss.count)
                .saturating_sub(1);
            self.message =
                "FIFO overflow: keys consumed; records lost; recording remains armed".into();
        }
    }
    fn command(&mut self, command: Command) {
        // Always drain even when Begin/preheld validation fails. No pumps/dialogs
        // between apply and native/core transition. The returned boundary batch
        // normally is empty; it protects that invariant if the adapter evolves.
        self.apply(native::drain());
        match native::boundary(command) {
            Ok(batch) => self.apply(batch),
            Err(e) => {
                self.message = e;
                return;
            }
        }
        match command {
            Command::Begin | Command::Switch => {
                let action = if self
                    .recorder
                    .action()
                    .is_some_and(|a| a.as_str() == "toggleVisibility")
                {
                    "toggleInteractive"
                } else {
                    "toggleVisibility"
                };
                // Real core business rules; no duplicated normalization/recording
                // reducer. Effects are observation-only: no registrations exist.
                let effects = self.recorder.record(action, Some("probe only"));
                self.message = format!(
                    "Raw recording; {} core Begin/switch effects observed; no registrations",
                    effects.len()
                );
            }
            Command::DetachedStop => {
                let _ = self.recorder.finish_deferred_stop();
                self.message = "Uncancelled 200 ms Stop fired on CURRENT recorder".into();
            }
            Command::Global => {
                self.message =
                    "Global-handler ordering control drained; no OS hotkey/action registered".into()
            }
            _ => {
                let _ = self.recorder.stop();
                self.message =
                    "Stopped; matching-release guard + bounded character tail only".into();
            }
        }
        self.arm = native::status().1;
    }
    fn close(&mut self) -> bool {
        self.command(Command::Close);
        let clean = native::shutdown(false);
        if !clean {
            self.message = "Close refused: inactive hook/observer cleanup debt; retry".into();
        }
        clean
    }
    fn click(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        match id {
            "begin" => self.command(Command::Begin),
            "switch" => self.command(Command::Switch),
            "stop" => self.command(Command::Stop),
            "save" => {
                self.command(Command::Save);
                self.message = "Save boundary ONLY; no userdata/storage written".into();
            }
            "cancel" => self.command(Command::Cancel),
            "global" => self.command(Command::Global),
            "focus" => self.command(Command::FocusLost),
            "timer" => {
                self.apply(native::drain());
                // Explicit timer injection, not a successfully decoded shortcut.
                // No arm/generation token is captured or checked in this closure.
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(200))
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.command(Command::DetachedStop);
                        cx.notify();
                    });
                })
                .detach();
                self.message =
                    "Queued independent 200 ms Stop; switch now to test old callback".into();
            }
            "chars" => {
                self.command(Command::Stop);
                match native::prequeue_characters() {
                    Ok(()) => self.command(Command::Begin),
                    Err(e) => self.message = e,
                }
            }
            "sent" | "retrieval" => {
                self.apply(native::drain());
                let sent = id == "sent";
                cx.spawn(async move |this, cx| {
                    if this
                        .update(cx, |this, _| this.apply(native::drain()))
                        .is_err()
                    {
                        return;
                    }
                    // SendMessage/PeekMessage can reenter GPUI. No App/entity
                    // borrow is alive here. This is a TEST, not the drain.
                    let result = if sent {
                        native::sent_negative_control().map(|_| {
                            "Sent key bypassed retrieval: expected negative coverage control".into()
                        })
                    } else {
                        native::retrieval_probe()
                    };
                    let _ = this.update(cx, |this, cx| {
                        this.apply(native::drain());
                        this.message = result.unwrap_or_else(|e| e);
                        cx.notify();
                    });
                })
                .detach();
            }
            "snapshot" => {
                self.fail_snapshot = !self.fail_snapshot;
                native::fail_snapshot(self.fail_snapshot);
            }
            "debt" => {
                self.command(Command::Close);
                let clean = native::shutdown(true);
                self.message = format!(
                    "Injected skipped unhook (real retained inactive installation). clean={clean}"
                );
            }
            "retry" => {
                self.apply(native::drain());
                self.message = format!("Real cleanup retry clean={}", native::shutdown(false));
            }
            "install" => {
                self.apply(native::drain());
                match install(window) {
                    Ok(()) => {
                        self.message = "Installed inactive; explicit Start required".into();
                        self.arm = 0;
                        self.last_sequence = 0;
                    }
                    Err(e) => self.message = e,
                }
            }
            "load" => {
                // Foreground work > GPUI's 10 ms task budget should reach its
                // nested PeekMessage route. A debugger stack is still required
                // to attest the route; the hook cannot infer its caller pump.
                for _ in 0..80 {
                    cx.spawn(async move |_this, _cx| {
                        let start = Instant::now();
                        while start.elapsed() < Duration::from_millis(12) {
                            std::hint::spin_loop();
                        }
                    })
                    .detach();
                }
                self.message =
                    "Foreground load requested; verify both pump stacks with debugger".into();
            }
            "close" if self.close() => window.remove_window(),
            _ => {}
        }
        cx.notify();
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (armed, arm, c, hook_debt, observer_debt) = native::status();
        let mods = window.modifiers();
        let pending = window.pending_input_keystrokes().map_or(0, |v| v.len());
        let mut controls = div().flex().flex_wrap().gap_2();
        for (id, label) in [
            ("begin", "Start recording"),
            ("switch", "Switch recorder"),
            ("stop", "Stop"),
            ("timer", "200 ms Stop"),
            ("save", "Save boundary"),
            ("cancel", "Cancel"),
            ("chars", "Prequeue chars + Begin"),
            ("snapshot", "Toggle snapshot failure"),
            ("global", "Global boundary"),
            ("focus", "Focus boundary"),
            ("sent", "Sent negative control"),
            ("retrieval", "Peek + FIFO probe"),
            ("load", "Nested-pump load"),
            ("debt", "Retain unhook debt"),
            ("retry", "Retry cleanup"),
            ("install", "Install inactive"),
            ("close", "Close"),
        ] {
            controls = controls.child(
                div()
                    .id(id)
                    .px_3()
                    .py_2()
                    .bg(rgb(0x283550))
                    .rounded_md()
                    .cursor_pointer()
                    .child(label)
                    .on_click(cx.listener(move |this, _, window, cx| this.click(id, window, cx))),
            );
        }
        div().size_full().p_5().bg(rgb(0x101725)).text_color(rgb(0xdce4f2)).flex().flex_col().gap_3().track_focus(&self.focus).key_context("S9")
            .on_key_down(cx.listener(|this,_,_,cx| {this.gpui_keys=this.gpui_keys.saturating_add(1);cx.notify();}))
            .on_action(cx.listener(|this,_:&Canary,_,cx| {this.apply(native::drain());this.actions=this.actions.saturating_add(1);cx.notify();}))
            .child("S9 — OWN-WINDOW RAW SNAPSHOT PROBE — NO-GO FOR INTEGRATION")
            .child("No DOM decoder; no text editor; no user data saved. Mouse controls remain usable while armed.")
            .child(controls)
            .child(format!("armed={armed} arm={arm} core_recording={} snapshot_fault={}",self.recorder.is_recording(),self.fail_snapshot))
            .child(format!("consumed={} down={} up={} chars={} raw={} overflow={} stale={} last_seq={}",c.consumed,c.downs,c.ups,c.characters,self.records,self.losses,self.stale,self.last_sequence))
            .child(format!("typed diagnostics: SnapshotFailed={} UnsupportedTranslation={} (all keydowns unsupported)",self.snapshot_errors,self.unsupported))
            .child(format!("GPUI keys={} canary actions={} pending_keys={pending}; modifiers Ctrl={} Alt={} Shift={} Win={}",self.gpui_keys,self.actions,mods.control,mods.alt,mods.shift,mods.platform))
            .child(format!("armed WNDPROC: system_commands={} IME={} bypass_keys={}; depth={} focus_losses={} destroyed={}",c.system_commands,c.ime_messages,c.bypass_keys,c.max_depth,c.focus_losses,c.destroyed))
            .child(format!("inactive cleanup debt: hook={hook_debt} observer={observer_debt}"))
            .child(self.last_snapshot.clone())
            .child(self.message.clone())
            .child("Canaries: F10, Alt, Ctrl+K then Ctrl+C. Counts alone do not prove native acceptance.")
            .child("BLOCKERS: preheld-release swallowing can strand GPUI private modifiers/keymaps; IME/composition is not reconciled. Sent input and OS-reserved keys are outside this seam.")
    }
}
pub fn run() {
    let mut start = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--start-recording" => start = true,
            "--help" => {
                println!(
                    "S9 raw-snapshot experiment. Optional --start-recording explicitly arms only its own focused window. No files written. Windows acceptance pending; integration NO-GO."
                );
                return;
            }
            _ => {
                eprintln!("Unknown argument; expected --start-recording or --help");
                std::process::exit(2);
            }
        }
    }
    gpui_platform::application().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("f10", Canary, Some("S9")),
            KeyBinding::new("alt", Canary, Some("S9")),
            KeyBinding::new("ctrl-k ctrl-c", Canary, Some("S9")),
        ]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                // Unexpected destruction can bypass should-close. The observer
                // has already cleared HWND synchronously; no new recorder can
                // receive these abandoned records. This is not business parity.
                let abandoned=native::drain();
                if abandoned.records.iter().any(Option::is_some) || abandoned.loss.is_some() {
                    eprintln!("S9 unexpected destruction: abandoned raw diagnostics discarded; close ordering not accepted");
                }
                if !native::shutdown(false) {
                    eprintln!("S9 exiting with inactive hook/observer cleanup debt; not a successful cleanup");
                }
                cx.quit();
            }
        })
        .detach();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(940.), px(760.)),
                cx,
            ))),
            ..Default::default()
        };
        if cx
            .open_window(options, |window, cx| {
                cx.new(|cx| Probe::new(window, cx, start))
            })
            .is_err()
        {
            eprintln!("S9 could not create its probe window");
            cx.quit();
        }
    });
}
