//! UI-thread boundary only. No native hotkey manager or input adapter lives here.
use super::*;
use echosight_core::settings::{HotkeyAction, HotkeySettings};
use std::{rc::Rc, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionStatus {
    Active,
    Inactive(String),
    ConversionAborted(String),
    NotAttempted(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterSummary {
    pub actions: [(HotkeyAction, ActionStatus); 5],
    pub retained: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReleaseSummary {
    pub released: Vec<String>,
    pub retained: Vec<(String, String)>,
}
#[derive(Clone, Copy, Debug)]
pub enum TrayCommand {
    Show,
    Hide,
    Exit,
}
pub struct OverlayConfig {
    pub ports: Rc<dyn OverlayPorts>,
    pub register_delay: Duration,
}
pub trait OverlayPorts {
    fn register_hotkeys(&self, keys: &HotkeySettings) -> RegisterSummary;
    fn suspend_hotkeys(&self) -> ReleaseSummary;
    fn shutdown(&self) -> ReleaseSummary;
    fn is_visible(&self) -> Option<bool>;
    /// Show and assert topmost. Preserve the current click-through mode.
    fn show(&self) -> Result<(), String>;
    fn hide(&self) -> Result<(), String>;
    fn focus(&self) -> Result<(), String>;
    fn set_click_through(&self, enabled: bool) -> Result<(), String>;
}
pub(super) struct OverlayState {
    pub ports: Option<Rc<dyn OverlayPorts>>,
    pub delay_elapsed: bool,
    pub initial_registered: bool,
    pub interactive: bool,
    pub manually_hidden: bool,
    pub registration: Option<RegisterSummary>,
    pub release: Option<ReleaseSummary>,
    pub quit_requested: bool,
    pub shutdown: bool,
}
impl Default for OverlayState {
    fn default() -> Self {
        Self {
            ports: None,
            delay_elapsed: false,
            initial_registered: false,
            interactive: true,
            manually_hidden: false,
            registration: None,
            release: None,
            quit_requested: false,
            shutdown: false,
        }
    }
}
impl TaskApp {
    pub fn new_with_overlay(
        dir: PathBuf,
        config: OverlayConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self::new(dir, window, cx);
        this.overlay.ports = Some(config.ports);
        let timer = cx.background_executor().timer(config.register_delay);
        cx.spawn(async move |this, cx| {
            timer.await;
            let _ = this.update(cx, |this, cx| {
                this.overlay.delay_elapsed = true;
                this.maybe_register_initial();
                cx.notify();
            });
        })
        .detach();
        this
    }
    pub(super) fn maybe_register_initial(&mut self) {
        if !self.settings.loading
            && self.overlay.delay_elapsed
            && !self.overlay.initial_registered
            && !self.overlay.shutdown
        {
            self.overlay.initial_registered = true;
            self.register_committed();
        }
    }
    pub(super) fn register_committed(&mut self) {
        if !self.overlay.shutdown
            && let Some(ports) = &self.overlay.ports
        {
            let summary = ports.register_hotkeys(&self.settings.committed);
            eprintln!("hotkey registration: {summary:?}");
            self.overlay.registration = Some(summary);
        }
    }
    pub fn global_hotkey(
        &mut self,
        action: HotkeyAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay.shutdown {
            return;
        }
        let Some(ports) = self.overlay.ports.clone() else {
            return;
        };
        if action == HotkeyAction::ToggleVisibility {
            match ports.is_visible() {
                Some(true) => self.tray_command(TrayCommand::Hide, window, cx),
                Some(false) => self.tray_command(TrayCommand::Show, window, cx),
                None => self.status = "Overlay visibility unavailable".into(),
            }
        } else if ports.is_visible() == Some(true) {
            match action {
                HotkeyAction::ToggleInteractive => {
                    self.overlay.interactive = !self.overlay.interactive;
                    if let Err(error) = ports.set_click_through(!self.overlay.interactive) {
                        self.status = error;
                    }
                }
                HotkeyAction::CompleteNextTask => {
                    self.run_task_action(TaskAction::Complete, TaskActionOrigin::Global, cx)
                }
                HotkeyAction::UndoLastAction => {
                    self.run_task_action(TaskAction::Undo, TaskActionOrigin::Global, cx)
                }
                HotkeyAction::RedoLastAction => {
                    self.run_task_action(TaskAction::Redo, TaskActionOrigin::Global, cx)
                }
                HotkeyAction::ToggleVisibility => unreachable!(),
            }
        }
        cx.notify();
    }
    pub fn tray_command(
        &mut self,
        command: TrayCommand,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay.shutdown {
            return;
        }
        if matches!(command, TrayCommand::Exit) {
            self.request_quit(cx);
            return;
        }
        let Some(ports) = &self.overlay.ports else {
            return;
        };
        let result = match command {
            TrayCommand::Show => {
                self.overlay.manually_hidden = false;
                ports.show().and_then(|()| ports.focus())
            }
            TrayCommand::Hide => {
                self.overlay.manually_hidden = true;
                ports.hide()
            }
            TrayCommand::Exit => unreachable!(),
        };
        if let Err(error) = result {
            self.status = error;
        }
        cx.notify();
    }
    pub(super) fn quit_ready(&self) -> bool {
        !self.task_loading
            && !self.settings.loading
            && self.templates_load_settled()
            && !self.saving
            && (self.pending.is_empty() || self.error.is_some())
            && !self.settings.saving
            && self.settings.pending.is_empty()
            && self.template_writes_settled()
    }
    pub(super) fn request_quit(&mut self, cx: &mut Context<Self>) {
        if self.overlay.quit_requested {
            return;
        }
        self.overlay.quit_requested = true;
        if self.finish_quit(cx) {
            return;
        }
        // Readiness polling only: no fabricated visibility or native events.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
                match this.update(cx, |this, cx| this.finish_quit(cx)) {
                    Ok(false) => {}
                    _ => break,
                }
            }
        })
        .detach();
        cx.notify();
    }
    fn finish_quit(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.quit_ready() {
            return false;
        }
        if !self.overlay.shutdown {
            self.overlay.shutdown = true;
            if let Some(ports) = &self.overlay.ports {
                let summary = ports.shutdown();
                eprintln!("hotkey shutdown: {summary:?}");
                self.overlay.release = Some(summary);
            }
            cx.quit();
        }
        true
    }
}
