//! Runnable task slice. Domain operations live in TaskState; every disk operation
//! runs off the UI thread. This is not the complete overlay/theme runtime.
mod drag;
mod persistence;
mod view;
mod window;
pub use window::window_options;

use crate::components::text_input::{TextInput, TextInputEvent};
use echosight_core::{
    storage::{DataPaths, FileKind, Storage},
    task::{
        model::{BackgroundPriority, Task, TaskEnvironment, TaskId, TaskMode},
        state::TaskState,
    },
};
use gpui::{Context, Entity, FocusHandle, Focusable, KeyDownEvent, Window, prelude::*};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
};

struct Clock;
impl TaskEnvironment for Clock {
    fn now_ms(&self) -> f64 {
        chrono::Utc::now().timestamp_millis() as f64
    }
    fn now_iso(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
    fn random(&mut self) -> f64 {
        rand::random()
    }
}

#[derive(Clone)]
enum Command {
    Add,
    Child(TaskId),
    Edit(TaskId),
    Delete(TaskId),
    Toggle(TaskId),
    Undo,
    Redo,
    Complete,
    Cancel,
    Accept,
    Priority,
    Quit,
}
#[derive(Clone)]
enum Dialog {
    Child(TaskId),
    Edit(TaskId, bool),
    Delete(TaskId),
    Quit,
}

pub struct TaskApp {
    state: Option<TaskState<Clock>>,
    storage: Option<Storage>,
    pending: VecDeque<Value>,
    saving: bool,
    error: Option<String>,
    status: String,
    input: Entity<TextInput>,
    editor: Entity<TextInput>,
    dialog: Option<Dialog>,
    high_priority: bool,
    select_editor: bool,
    form_error: Option<String>,
    restore_focus: Option<FocusHandle>,
    controls: BTreeMap<String, FocusHandle>,
    tab_order: Vec<(FocusHandle, Option<Command>)>,
    rows: Vec<drag::RowGeometry>,
    list_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    dragging: Option<drag::Drag>,
}

impl TaskApp {
    pub fn new(dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_storage(
            DataPaths::new(dir)
                .map(|paths| (Storage::new(paths.clone()), paths))
                .map_err(|e| e.to_string()),
            window,
            cx,
        )
    }

    fn with_storage(
        storage: Result<(Storage, DataPaths), String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            TextInput::new(cx)
                .placeholder("Add new task...")
                .max_length(100)
                .style(view::input_style())
        });
        let editor = Self::new_editor(String::new(), window, cx);
        window.focus(&input.focus_handle(cx), cx);
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let _ = weak.update(cx, |this, cx| this.activate(Command::Quit, window, cx));
            false
        });
        cx.subscribe_in(&input, window, |this, _, event, window, cx| {
            if matches!(event, TextInputEvent::Submit(_)) && this.dialog.is_none() {
                this.activate(Command::Add, window, cx);
            } else if matches!(event, TextInputEvent::Cancel) {
                this.dragging = None;
                cx.notify();
            }
        })
        .detach();
        let mut this = Self {
            state: None,
            storage: None,
            pending: VecDeque::new(),
            saving: false,
            error: None,
            status: "Loading tasks… Editing is disabled until load succeeds.".into(),
            input,
            editor,
            dialog: None,
            high_priority: false,
            select_editor: false,
            form_error: None,
            restore_focus: None,
            controls: BTreeMap::new(),
            tab_order: vec![],
            rows: vec![],
            list_bounds: None,
            dragging: None,
        };
        match storage {
            Err(e) => this.error = Some(e),
            Ok((storage, paths)) => {
                this.storage = Some(storage.clone());
                let task = cx
                    .background_executor()
                    .spawn(async move { persistence::load(&storage, &paths) });
                cx.spawn(async move |this, cx| {
                    let result = task.await;
                    let _ = this.update(cx, |this, cx| {
                        match result {
                            Ok(data) => {
                                this.state = Some(TaskState::new(
                                    data.tasks,
                                    data.current_template,
                                    data.snapshots,
                                    Clock,
                                ));
                                this.status =
                                    "Ready. Changes save to the explicit development directory."
                                        .into();
                            }
                            Err(error) => this.error = Some(error),
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        this
    }

    fn new_editor(text: String, window: &mut Window, cx: &mut Context<Self>) -> Entity<TextInput> {
        let editor = cx.new(|cx| {
            let mut input = TextInput::new(cx)
                .placeholder("Task name")
                .style(view::input_style());
            // Preserve loaded names before enabling the normal insertion limit.
            input.set_text(text, window, cx);
            input.max_length(100)
        });
        cx.subscribe_in(&editor, window, |this, _, event, window, cx| match event {
            TextInputEvent::Submit(_) => this.activate(Command::Accept, window, cx),
            TextInputEvent::Cancel => this.close_dialog(window, cx),
            _ => {}
        })
        .detach();
        editor
    }

    fn deletable(&self, id: TaskId) -> bool {
        fn subtree_ok(app: &TaskApp, task: &Task) -> bool {
            app.addressable(task.id) && task.children.iter().all(|child| subtree_ok(app, child))
        }
        self.addressable(id)
            && self
                .state
                .as_ref()
                .and_then(|s| s.find_task(id))
                .is_some_and(|task| subtree_ok(self, task))
    }

    fn writable(&self) -> bool {
        self.state.is_some() && self.error.is_none()
    }

    /// One in-flight save, FIFO snapshots, no debounce. A failed write stops the
    /// entire funnel; neither later snapshots nor subsequent edits overwrite it.
    fn save(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.state.as_mut() {
            self.pending.push_back(
                serde_json::to_value(state.record_auto_save_snapshot()).expect("finite task state"),
            );
            self.pump(cx);
        }
        cx.notify();
    }
    fn pump(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.error.is_some() {
            return;
        }
        let Some(value) = self.pending.pop_front() else {
            return;
        };
        let storage = self.storage.clone().expect("successful load has storage");
        self.saving = true;
        let task = cx.background_executor().spawn(async move {
            storage
                .save(FileKind::Tasks, &value)
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(()) => {
                        this.status = "Saved".into();
                        this.pump(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!(
                            "Save failed; changes are NOT saved. Further edits blocked. {error}"
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // The core API intentionally retains legacy ID-based lookup. Never pretend a
    // duplicate-ID row can be addressed perfectly: deny ambiguous ancestry rather
    // than editing a different task or passing an unsafe destination to the core.
    fn addressable(&self, id: TaskId) -> bool {
        let Some(state) = &self.state else {
            return false;
        };
        fn count(tasks: &[Task], id: TaskId) -> usize {
            tasks
                .iter()
                .map(|t| usize::from(t.id == id) + count(&t.children, id))
                .sum()
        }
        fn walk(tasks: &[Task], all: &[Task], id: TaskId, ancestors_ok: bool) -> bool {
            tasks.iter().any(|t| {
                let ok = ancestors_ok && count(all, t.id) == 1;
                (t.id == id && ok) || walk(&t.children, all, id, ok)
            })
        }
        walk(state.tasks(), state.tasks(), id, true)
    }

    fn complete_next_addressable(&self) -> bool {
        let Some(state) = &self.state else {
            return false;
        };
        // Complete next selects structurally, but resolves parents by ID. Be
        // conservative across the whole tree rather than duplicate core selection.
        fn all_addressable(app: &TaskApp, tasks: &[Task]) -> bool {
            tasks
                .iter()
                .all(|task| app.addressable(task.id) && all_addressable(app, &task.children))
        }
        all_addressable(self, state.tasks())
    }

    fn activate(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(command, Command::Cancel) {
            self.close_dialog(window, cx);
            return;
        }
        if matches!(command, Command::Quit) && self.dialog.is_none() {
            self.restore_focus = window.focused(cx);
            self.dialog = Some(Dialog::Quit);
            self.dragging = None;
            let focus = self
                .controls
                .entry("cancel".into())
                .or_insert_with(|| cx.focus_handle())
                .clone();
            window.focus(&focus, cx);
            cx.notify();
            return;
        }
        if matches!(command, Command::Accept) && matches!(self.dialog, Some(Dialog::Quit)) {
            if !self.saving {
                cx.quit();
            }
            return;
        }
        if !self.writable() {
            return;
        }
        if self.dialog.is_some() && !matches!(command, Command::Accept | Command::Priority) {
            return;
        }
        let id = match command {
            Command::Child(id) | Command::Edit(id) | Command::Delete(id) | Command::Toggle(id) => {
                Some(id)
            }
            _ => None,
        };
        if id.is_some_and(|id| !self.addressable(id))
            || matches!(command, Command::Delete(id) if !self.deletable(id))
            || (matches!(command, Command::Complete) && !self.complete_next_addressable())
        {
            self.status =
                "Duplicate task IDs: this row or its ancestor is ambiguous; action blocked.".into();
            cx.notify();
            return;
        }
        let changed = match command {
            Command::Add => {
                let text =
                    echosight_core::jscompat::js_trim(self.input.read(cx).text()).to_string();
                if text.is_empty() {
                    return;
                }
                let changed = self.state.as_mut().unwrap().add_task(text, None).is_some();
                self.input.update(cx, |input, cx| input.clear(window, cx));
                changed
            }
            Command::Child(id) | Command::Edit(id) | Command::Delete(id) => {
                let task = self.state.as_ref().unwrap().find_task(id).unwrap();
                let text = if matches!(command, Command::Edit(_)) {
                    task.text.clone()
                } else {
                    String::new()
                };
                self.high_priority = task
                    .background_options
                    .as_ref()
                    .is_some_and(|o| o.priority == BackgroundPriority::High);
                self.dialog = Some(match command {
                    Command::Child(_) => Dialog::Child(id),
                    Command::Edit(_) => Dialog::Edit(id, task.mode == TaskMode::Background),
                    _ => Dialog::Delete(id),
                });
                self.dragging = None;
                self.form_error = None;
                self.restore_focus = window.focused(cx);
                self.editor = Self::new_editor(text, window, cx);
                let focus = if matches!(self.dialog, Some(Dialog::Delete(_))) {
                    self.controls
                        .entry("cancel".into())
                        .or_insert_with(|| cx.focus_handle())
                        .clone()
                } else {
                    self.editor.focus_handle(cx)
                };
                window.focus(&focus, cx);
                self.select_editor = matches!(self.dialog, Some(Dialog::Edit(..)));
                cx.notify();
                false
            }
            Command::Toggle(id) => self.state.as_mut().unwrap().toggle_task(id).is_some(),
            Command::Undo => self.state.as_mut().unwrap().undo().restored,
            Command::Redo => self.state.as_mut().unwrap().redo().restored,
            Command::Complete => self.state.as_mut().unwrap().complete_next_task().is_some(),
            Command::Priority => {
                self.high_priority = !self.high_priority;
                cx.notify();
                false
            }
            Command::Accept => {
                let Some(dialog) = self.dialog.clone() else {
                    return;
                };
                if matches!(dialog, Dialog::Delete(id) if !self.deletable(id)) {
                    self.status =
                        "Duplicate task IDs: deletion subtree is ambiguous; action blocked.".into();
                    cx.notify();
                    return;
                }
                let text =
                    echosight_core::jscompat::js_trim(self.editor.read(cx).text()).to_owned();
                if !matches!(dialog, Dialog::Delete(_)) && text.is_empty() {
                    self.form_error = Some("Please enter a task name!".into());
                    cx.notify();
                    return;
                }
                let deleting = matches!(dialog, Dialog::Delete(_));
                let changed = match dialog {
                    Dialog::Child(id) => self
                        .state
                        .as_mut()
                        .unwrap()
                        .add_task(text, Some(id))
                        .is_some(),
                    Dialog::Edit(id, background) => self.state.as_mut().unwrap().edit_task(
                        id,
                        &text,
                        background.then_some(if self.high_priority {
                            BackgroundPriority::High
                        } else {
                            BackgroundPriority::Normal
                        }),
                    ),
                    Dialog::Delete(id) => self.state.as_mut().unwrap().delete_task(id).removed,
                    Dialog::Quit => false,
                };
                if deleting {
                    self.restore_focus = Some(self.input.focus_handle(cx));
                }
                self.close_dialog(window, cx);
                changed
            }
            Command::Cancel | Command::Quit => false,
        };
        if changed {
            self.save(cx);
        }
    }

    fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        self.form_error = None;
        window.focus(
            &self
                .restore_focus
                .take()
                .unwrap_or_else(|| self.input.focus_handle(cx)),
            cx,
        );
        cx.notify();
    }

    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if key == "tab" && !self.tab_order.is_empty() {
            let current = self
                .tab_order
                .iter()
                .position(|(h, _)| h.is_focused(window));
            let len = self.tab_order.len();
            let next = match (current, event.keystroke.modifiers.shift) {
                (Some(i), true) => (i + len - 1) % len,
                (Some(i), false) => (i + 1) % len,
                (None, true) => len - 1,
                (None, false) => 0,
            };
            window.focus(&self.tab_order[next].0, cx);
        } else if key == "escape" && self.dragging.is_some() {
            self.dragging = None;
        } else if key == "escape" && self.dialog.is_some() {
            self.close_dialog(window, cx);
        } else if self.dialog.is_none()
            && (event.keystroke.modifiers.control || event.keystroke.modifiers.platform)
            && event.keystroke.modifiers.shift
            && matches!(key, "z" | "y")
        {
            self.activate(
                if key == "z" {
                    Command::Undo
                } else {
                    Command::Redo
                },
                window,
                cx,
            );
        } else if matches!(key, "space" | "enter") {
            let command = self
                .tab_order
                .iter()
                .find(|(h, _)| h.is_focused(window))
                .and_then(|(_, c)| c.clone());
            if let Some(command) = command {
                if key != "enter"
                    || !matches!(self.dialog, Some(Dialog::Delete(_) | Dialog::Quit))
                    || matches!(command, Command::Cancel)
                {
                    self.activate(command, window, cx);
                }
            } else {
                return;
            }
        } else {
            return;
        }
        cx.stop_propagation();
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
