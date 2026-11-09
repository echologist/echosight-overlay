//! Spike S4: `global-hotkey` inside GPUI's run loop.
//!
//! GPUI has no global hotkey API. The `global-hotkey` crate (tauri-apps) needs a Win32
//! message loop on the thread that creates the manager; GPUI's Windows run loop is a
//! plain `GetMessageW`/`DispatchMessageW` loop, so the manager is created inside
//! `Application::run` (main thread, before the window opens) and events are drained from
//! `GlobalHotKeyEvent::receiver()` with `try_recv` on the shell's 100 ms foreground
//! timer (never blocking the UI thread).
//!
//! Registers Ctrl+Shift+T / I / N / Z / Y (the Echosight defaults) and logs each result as
//! `<combo>: SUCCESS|FAILED` like the Electron main process does. F8 unregisters all and
//! re-registers (the hotkey-recording suspend/resume path). A combo already taken by another
//! app must be reported as FAILED, not crash the app.
//!
//! Manual test script: `docs/plans/gpui-port/spike-s4.md`.

mod common;

use common::{Button, ShellCx, Spike};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

const COMBOS: [(&str, Code); 5] = [
    ("Ctrl+Shift+T", Code::KeyT),
    ("Ctrl+Shift+I", Code::KeyI),
    ("Ctrl+Shift+N", Code::KeyN),
    ("Ctrl+Shift+Z", Code::KeyZ),
    ("Ctrl+Shift+Y", Code::KeyY),
];

struct Registered {
    label: &'static str,
    hotkey: HotKey,
    ok: bool,
}

struct S4 {
    manager: Option<GlobalHotKeyManager>,
    hotkeys: Vec<Registered>,
    registered: bool,
    fired: u64,
    last: String,
    reregisters: u32,
    /// Log lines produced before the window (and its on-screen log) existed.
    pending: Vec<String>,
}

/// Where registration messages go: the on-screen log once the window exists, a buffer
/// before that. Both also go to the process log.
enum Sink<'a, 'b> {
    Buffer(&'a mut Vec<String>),
    Shell(&'a mut ShellCx<'b>),
}

impl Sink<'_, '_> {
    fn log(&mut self, line: String) {
        match self {
            Sink::Buffer(buf) => {
                log::info!("{line}");
                buf.push(line);
            }
            Sink::Shell(cx) => cx.log(line),
        }
    }
}

impl S4 {
    /// Must run on the main thread inside `Application::run` (Win32 message loop).
    fn new() -> Self {
        let mut pending = Vec::new();
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => Some(m),
            Err(e) => {
                let line = format!("GlobalHotKeyManager::new FAILED: {e}");
                log::error!("{line}");
                pending.push(line);
                None
            }
        };
        let hotkeys = COMBOS
            .iter()
            .map(|(label, code)| Registered {
                label,
                hotkey: HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), *code),
                ok: false,
            })
            .collect();
        let mut this = Self {
            manager,
            hotkeys,
            registered: false,
            fired: 0,
            last: "(none yet)".to_owned(),
            reregisters: 0,
            pending,
        };
        let mut buffer = std::mem::take(&mut this.pending);
        this.register_all(&mut Sink::Buffer(&mut buffer));
        this.pending = buffer;
        this
    }

    fn register_all(&mut self, sink: &mut Sink) {
        let Some(manager) = self.manager.as_ref() else {
            sink.log("no manager; nothing registered".to_owned());
            return;
        };
        for entry in &mut self.hotkeys {
            entry.ok = match manager.register(entry.hotkey) {
                Ok(()) => {
                    sink.log(format!("{}: SUCCESS", entry.label));
                    true
                }
                Err(e) => {
                    sink.log(format!("{}: FAILED ({e})", entry.label));
                    false
                }
            };
        }
        self.registered = true;
    }

    fn unregister_all(&mut self, sink: &mut Sink) {
        let Some(manager) = self.manager.as_ref() else {
            return;
        };
        for entry in &mut self.hotkeys {
            if !entry.ok {
                continue;
            }
            match manager.unregister(entry.hotkey) {
                Ok(()) => sink.log(format!("{}: unregistered", entry.label)),
                Err(e) => sink.log(format!("{}: unregister FAILED ({e})", entry.label)),
            }
            entry.ok = false;
        }
        self.registered = false;
    }

    fn label_for(&self, id: u32) -> &'static str {
        self.hotkeys
            .iter()
            .find(|entry| entry.hotkey.id() == id)
            .map(|entry| entry.label)
            .unwrap_or("unknown id")
    }
}

impl Spike for S4 {
    const TITLE: &'static str = "Echosight spike S4: global hotkeys";
    const HINT: &'static str = "Press Ctrl+Shift+T/I/N/Z/Y while the GAME has focus. \
                                F8 = unregister all + re-register.";

    fn buttons(&self) -> Vec<Button> {
        vec![
            Button::new("rereg", "Unregister all + re-register"),
            Button::new(
                "suspend",
                if self.registered {
                    "Suspend (unregister all)"
                } else {
                    "Resume (register all)"
                },
            ),
        ]
    }

    fn status(&self) -> Vec<String> {
        let table = self
            .hotkeys
            .iter()
            .map(|e| format!("{} {}", if e.ok { "ok" } else { "--" }, e.label))
            .collect::<Vec<_>>()
            .join("   ");
        vec![
            format!(
                "fired: {}   last: {}   re-registers: {}",
                self.fired, self.last, self.reregisters
            ),
            table,
            "Expected: all 5 fire with the game focused (borderless); F8 keeps them working;"
                .to_owned(),
            "a combo taken by another app shows FAILED and the rest still work.".to_owned(),
        ]
    }

    fn on_button(&mut self, id: &str, cx: &mut ShellCx) {
        match id {
            "rereg" => {
                self.unregister_all(&mut Sink::Shell(cx));
                self.register_all(&mut Sink::Shell(cx));
                self.reregisters += 1;
            }
            "suspend" => {
                if self.registered {
                    self.unregister_all(&mut Sink::Shell(cx));
                } else {
                    self.register_all(&mut Sink::Shell(cx));
                }
            }
            _ => {}
        }
    }

    fn on_key(&mut self, key: &str, cx: &mut ShellCx) {
        match key {
            "f8" => self.on_button("rereg", cx),
            other => cx.log(format!("key: {other}")),
        }
    }

    fn on_tick(&mut self, cx: &mut ShellCx) {
        for line in self.pending.drain(..) {
            cx.show(line);
        }
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state() != HotKeyState::Pressed {
                continue;
            }
            self.fired += 1;
            self.last = self.label_for(event.id()).to_owned();
            cx.log(format!("hotkey #{}: {}", self.fired, self.last));
        }
    }
}

fn main() {
    common::run(|_cx| S4::new());
}
