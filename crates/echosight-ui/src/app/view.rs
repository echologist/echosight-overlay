use super::*;
use crate::components::text_input::TextInputStyle;
use gpui::{
    Image, ImageFormat, SharedString, WindowControlArea, div, img, px, relative, rgb, rgba,
};
use std::sync::{Arc, LazyLock};

// Bounded mapping from the frozen default theme, not a replacement design system.
// CSS, gradients, custom fonts, transparency and Theme V2 remain pending.
const SURFACE: u32 = 0x14111f;
const TEXT: u32 = 0xe8e2d2;
const GOLD: u32 = 0xd4a857;
const TEAL: u32 = 0x5fd8c8;
const MUTED: u32 = 0xb8b09a;
static EYE: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../../../../assets/icon.png").to_vec(),
    ))
});

pub(super) fn input_style() -> TextInputStyle {
    TextInputStyle {
        background: rgb(0x0b0912).into(),
        border: rgba(0xd4a85766).into(),
        border_focused: rgb(TEAL).into(),
        text: rgb(TEXT).into(),
        placeholder: rgb(MUTED).into(),
        cursor: rgb(GOLD).into(),
        selection: rgba(0x5fd8c844).into(),
    }
}

impl TaskApp {
    fn button(
        &mut self,
        key: String,
        label: impl Into<SharedString>,
        command: Command,
        enabled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let focus = self
            .controls
            .entry(key.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        if enabled {
            self.tab_order.push((focus.clone(), Some(command.clone())));
        }
        let selector = key.clone();
        div()
            .id(SharedString::from(key))
            .debug_selector(move || selector.clone())
            .track_focus(&focus)
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(if focus.is_focused(window) {
                rgb(TEAL)
            } else {
                rgba(0xd4a85766)
            })
            .text_color(if enabled { rgb(GOLD) } else { rgb(0x7a7363) })
            .when(enabled, |el| {
                el.cursor_pointer().hover(|s| s.bg(rgba(0x5fd8c822)))
            })
            .child(label.into())
            .on_click(cx.listener(move |this, _, window, cx| {
                if enabled {
                    window.focus(&focus, cx);
                    this.activate(command.clone(), window, cx);
                }
                cx.stop_propagation();
            }))
    }
}

impl Render for TaskApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tab_order.clear();
        self.rows.clear();
        let enabled = self.writable() && self.dialog.is_none();
        let mut root = div()
            .id("task-app")
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(rgb(SURFACE))
            .text_color(rgb(TEXT))
            .text_sm()
            .font_family(if cfg!(target_os = "windows") {
                "Segoe UI"
            } else {
                "Noto Sans"
            })
            .on_key_down(cx.listener(Self::key))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if event.pressed_button == Some(gpui::MouseButton::Left) {
                    this.drag_to(event.position, cx);
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| {
                    this.end_drag(event.position, cx)
                }),
            )
            .on_mouse_up_out(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.dragging = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("title")
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(img(EYE.clone()).w(px(28.)).h(px(28.)))
                    .child(div().text_color(rgb(GOLD)).child("ECHOSIGHT"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("Development task slice"),
                    ),
            );
        if enabled {
            self.tab_order.push((self.input.focus_handle(cx), None));
        }
        root = root.child(
            div()
                .flex()
                .gap_2()
                .child(div().flex_1().min_w_0().child(self.input.clone()))
                .child(self.button("add-task".into(), "Add", Command::Add, enabled, window, cx)),
        );
        root = root.child(
            div()
                .flex()
                .gap_2()
                .child(self.button("undo".into(), "Undo", Command::Undo, enabled, window, cx))
                .child(self.button("redo".into(), "Redo", Command::Redo, enabled, window, cx))
                .child(self.button(
                    "complete-next".into(),
                    "Complete next",
                    Command::Complete,
                    enabled && self.complete_next_addressable(),
                    window,
                    cx,
                ))
                .child(self.button(
                    "quit".into(),
                    "Quit",
                    Command::Quit,
                    self.dialog.is_none(),
                    window,
                    cx,
                )),
        );
        if let Some(state) = &self.state {
            let progress = state.progress();
            root = root
                .child(div().child(format!(
                    "{} / {} completed ({:.0}%)",
                    progress.completed, progress.total, progress.percentage
                )))
                .child(
                    div().h(px(4.)).bg(rgba(0xd4a85733)).child(
                        div()
                            .h_full()
                            .w(relative((progress.percentage / 100.) as f32))
                            .bg(rgb(TEAL)),
                    ),
                );
        }
        let tasks = self
            .state
            .as_ref()
            .map(|s| s.tasks().to_vec())
            .unwrap_or_default();
        let mut rows = div().id("rows-scroll").size_full().overflow_y_scroll();
        let mut flat = vec![];
        flatten(&tasks, &mut vec![], &mut flat);
        for (path, task) in flat {
            let path_key = path
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("-");
            // Unique IDs survive reorder; a structural suffix prevents duplicate
            // GPUI element collisions without claiming ID-based core precision.
            let unique = self.addressable(task.id);
            let key = if unique {
                format!("task-{:x}", task.id.0.to_bits())
            } else {
                format!("task-{:x}-{path_key}", task.id.0.to_bits())
            };
            let id = task.id;
            let row_entity = cx.entity();
            let mut row = div()
                .relative()
                .id(SharedString::from(key))
                .debug_selector({
                    let path_key = path_key.clone();
                    move || format!("row-{path_key}")
                })
                .flex()
                .flex_col()
                .gap_1()
                .py_2()
                .ml(px((path.len() - 1) as f32 * 20.))
                .child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(px(1.))
                        .bg(rgba(0xffffff0f)),
                );
            let label = format!(
                "{}{}{}",
                if task.completed { "✓ " } else { "" },
                task.text,
                if task.mode == TaskMode::Background {
                    if task.activated {
                        " [background]"
                    } else {
                        " [dormant — activation pending]"
                    }
                } else {
                    ""
                }
            );
            row = row.child(
                gpui::canvas(
                    move |bounds, _, cx| {
                        row_entity.update(cx, |this, _| {
                            this.rows.push(drag::RowGeometry { id, bounds })
                        })
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
            row = row.child(
                div()
                    .id(("grip", id.0.to_bits()))
                    .cursor_move()
                    .child(label)
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                            this.start_drag(id, event.position, cx)
                        }),
                    )
                    .when(task.completed, |el| el.text_color(rgb(MUTED))),
            );
            let allowed = enabled && unique;
            row = row.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.button(
                        format!("toggle-{path_key}"),
                        if task.completed { "Reopen" } else { "Done" },
                        Command::Toggle(task.id),
                        allowed,
                        window,
                        cx,
                    ))
                    .child(self.button(
                        format!("child-{path_key}"),
                        "+ Subtask",
                        Command::Child(task.id),
                        allowed,
                        window,
                        cx,
                    ))
                    .child(self.button(
                        format!("edit-{path_key}"),
                        "Edit",
                        Command::Edit(task.id),
                        allowed,
                        window,
                        cx,
                    ))
                    .child(self.button(
                        format!("delete-{path_key}"),
                        "Delete",
                        Command::Delete(task.id),
                        allowed && self.deletable(task.id),
                        window,
                        cx,
                    )),
            );
            if !unique {
                row = row.child(
                    div()
                        .text_xs()
                        .child("Duplicate ID or ambiguous ancestor: row actions blocked"),
                );
            }
            if let Some(intent) = self
                .dragging
                .as_ref()
                .and_then(|d| d.intent.as_ref())
                .filter(|i| i.target == task.id)
            {
                row = row.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .when(intent.above, |el| el.top_0())
                        .when(!intent.above, |el| el.bottom_0())
                        .text_xs()
                        .bg(rgb(SURFACE))
                        .text_color(rgb(TEAL))
                        .border_t_1()
                        .border_color(rgb(TEAL))
                        .child(intent.label),
                );
            }
            rows = rows.child(row);
        }
        let list_entity = cx.entity();
        root = root.child(
            div()
                .id("rows")
                .debug_selector(|| "rows".into())
                .relative()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(
                    gpui::canvas(
                        move |bounds, _, cx| {
                            list_entity.update(cx, |this, _| this.list_bounds = Some(bounds))
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(rows),
        );
        let status = if let Some(drag) = self.dragging.as_ref().filter(|d| d.active) {
            drag.intent
                .as_ref()
                .map(|i| i.label)
                .unwrap_or("Drop here to reorder (valid target required)")
                .to_owned()
        } else if self.error.is_some() {
            "Writes blocked — see error below.".into()
        } else if self.saving || !self.pending.is_empty() {
            "Saving… Do not close until Saved.".into()
        } else {
            self.status.clone()
        };
        root = root.child(
            div()
                .h(px(32.))
                .flex_shrink_0()
                .overflow_hidden()
                .text_xs()
                .text_color(rgb(MUTED))
                .child(status),
        );
        if let Some(error) = &self.error {
            root = root.child(div().text_color(rgb(0xe3804a)).child(error.clone()));
        }
        root = root.child(div().text_xs().text_color(rgb(MUTED)).child("Pending: themes/V2, templates, settings, trigger timers, sounds and overlay integration. Local undo: Ctrl+Shift+Z / Y."));
        if let Some(dialog) = self.dialog.clone() {
            if std::mem::take(&mut self.select_editor) {
                let focus = self.editor.focus_handle(cx);
                window.defer(cx, move |window, cx| {
                    if focus.is_focused(window) {
                        focus.dispatch_action(
                            &crate::components::text_input::SelectAll,
                            window,
                            cx,
                        );
                    }
                });
            }
            self.tab_order.clear();
            let delete = matches!(dialog, Dialog::Delete(_));
            let quitting = matches!(dialog, Dialog::Quit);
            let title = match dialog {
                Dialog::Child(_) => "Add Subtask",
                Dialog::Edit(..) => "Edit Task",
                Dialog::Delete(_) => "Delete Task",
                Dialog::Quit => "Quit Echosight",
            };
            let mut content = div()
                .w_full()
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(GOLD))
                .child(title);
            if quitting {
                content = content.child(if self.saving {
                    "Waiting for pending saves…"
                } else if self.error.is_some() {
                    "Storage reported an error. Unsaved changes will be lost. Quit anyway?"
                } else {
                    "Quit Echosight?"
                });
            } else if let Dialog::Delete(id) = dialog {
                if let Some(task) = self.state.as_ref().and_then(|s| s.find_task(id)) {
                    content = content.child(format!(
                        "Delete \"{}\"? You can undo this with Ctrl+Shift+Z.",
                        task.text
                    ));
                }
            } else {
                self.tab_order.push((self.editor.focus_handle(cx), None));
                content = content.child(self.editor.clone());
                if matches!(dialog, Dialog::Edit(_, true)) {
                    content = content.child(self.button(
                        "priority".into(),
                        if self.high_priority {
                            "Priority: HIGH"
                        } else {
                            "Priority: Normal"
                        },
                        Command::Priority,
                        true,
                        window,
                        cx,
                    ));
                }
            }
            if let Some(error) = self.form_error.as_ref().or(self.error.as_ref()) {
                content = content.child(
                    div()
                        .id("form-error")
                        .debug_selector(|| "form-error".into())
                        .text_color(rgb(0xe3804a))
                        .child(error.clone()),
                );
            }
            content = content.child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "cancel".into(),
                        "Cancel",
                        Command::Cancel,
                        true,
                        window,
                        cx,
                    ))
                    .child(self.button(
                        "accept".into(),
                        if quitting {
                            "Quit"
                        } else if delete {
                            "Delete"
                        } else {
                            "Save"
                        },
                        Command::Accept,
                        if quitting {
                            !self.saving
                        } else {
                            self.writable()
                        },
                        window,
                        cx,
                    )),
            );
            root = root.child(
                div()
                    .id("dialog-overlay")
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x000000bb))
                    .flex()
                    .items_center()
                    .p_4()
                    .child(content)
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            );
        }
        root
    }
}

fn flatten<'a>(tasks: &'a [Task], parent: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, &'a Task)>) {
    for (i, task) in tasks.iter().enumerate() {
        parent.push(i);
        out.push((parent.clone(), task));
        flatten(&task.children, parent, out);
        parent.pop();
    }
}
