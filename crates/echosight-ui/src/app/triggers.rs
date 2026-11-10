use super::*;
use echosight_core::task::background::trigger_rows;
use gpui::{div, rgb};

pub(super) struct ModalFrame {
    pub dialog: Dialog,
    pub editor: Entity<TextInput>,
    pub high_priority: bool,
    pub restore_focus: Option<FocusHandle>,
}

impl TaskApp {
    pub(super) fn open_triggers(
        &mut self,
        id: TaskId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.state() else {
            return;
        };
        let Some(task) = state.find_task(id) else {
            return;
        };
        self.trigger_rows = trigger_rows(task, state.tasks());
        self.dialog = Some(Dialog::Triggers(id));
        self.dragging = None;
        self.form_error = None;
        self.editor =
            Self::new_editor_with_placeholder(String::new(), "New background task...", window, cx);
        self.high_priority = self.new_background_high_priority;
        window.focus(&self.editor.focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn add_background_from_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(Dialog::Triggers(id)) = self.dialog else {
            return;
        };
        let text = echosight_core::jscompat::js_trim(self.editor.read(cx).text()).to_owned();
        if text.is_empty() {
            self.form_error = Some("Please enter a background task description.".into());
            cx.notify();
            return;
        }
        self.dispatch_background(BackgroundCommand::AddBackground {
            text,
            // The shipped trigger form has no expiry input. New tasks default
            // to no expiry; loaded expiry values survive dormant priority edits.
            options: serde_json::json!({ "priority": if self.high_priority { "high" } else { "normal" }, "expiresAfterMinutes": null }),
        }, cx);
        self.dispatch_background(BackgroundCommand::Save, cx);
        // Source reopens the configuration, resetting selection from saved
        // triggers and clearing only the description, NOT the priority checkbox.
        self.open_triggers(id, window, cx);
    }

    pub(super) fn trigger_fields(&mut self, window: &Window, cx: &mut Context<Self>) -> gpui::Div {
        let mut fields = div()
            .flex()
            .flex_col()
            .gap_3()
            .child("When this task is completed, the selected background tasks will activate.");
        if self.trigger_rows.is_empty() {
            fields = fields.child("No background tasks yet. Create one below.");
        }
        for (index, row) in self.trigger_rows.clone().into_iter().enumerate() {
            let allowed = self.addressable(row.id);
            let mut choice = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(self.button(
                    format!("trigger-choice-{index}"),
                    format!("{} {}", if row.checked { "☑" } else { "☐" }, row.text),
                    Command::TriggerChoice(index),
                    allowed,
                    window,
                    cx,
                ))
                .child(div().text_xs().child(row.status_badge));
            if let Some(badge) = row.priority_badge {
                choice = choice.child(div().text_xs().text_color(rgb(view::GOLD)).child(badge));
            }
            choice = choice.child(self.button(
                format!("trigger-edit-{index}"),
                "Edit",
                Command::Edit(row.id),
                allowed,
                window,
                cx,
            ));
            fields = fields.child(choice);
        }
        self.tab_order.push((self.editor.focus_handle(cx), None));
        fields
            .child(
                div()
                    .id("new-background-input")
                    .debug_selector(|| "new-background-input".into())
                    .child(self.editor.clone()),
            )
            .child(self.button(
                "add-background".into(),
                "+ Add",
                Command::AddBackground,
                true,
                window,
                cx,
            ))
            .child(self.button(
                "priority".into(),
                if self.high_priority {
                    "☑ High Priority"
                } else {
                    "☐ High Priority"
                },
                Command::Priority,
                true,
                window,
                cx,
            ))
    }
}
