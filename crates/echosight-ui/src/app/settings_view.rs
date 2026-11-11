//! Settings controls reuse the bounded task-dialog shell and its palette.
//! Theme application/audio/position and native recorder input remain separate ports.
use super::*;
use echosight_core::settings::{HotkeyAction, renderer_defaults};
use gpui::{SharedString, div, px, relative, rgb};

impl TaskApp {
    pub(super) fn settings_fields(
        &mut self,
        reset: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        if reset {
            let defaults = renderer_defaults(settings::platform());
            let labels = HotkeyAction::ALL
                .map(|a| {
                    echosight_core::settings::format_hotkey_for_display(
                        defaults.hotkeys.get(a),
                        settings::platform(),
                    )
                })
                .join(", ");
            return div().child(format!("Reset hotkeys to default values ({labels})?"));
        }
        let mut fields = div().flex().flex_col().gap_3()
            .child("Appearance")
            .child(self.settings_range(0, window, cx))
            .child(format!("Theme: {}", self.settings.live.theme))
            .child("Theme selection, transparency rendering, audio and position integration pending.")
            .child("Theme Sounds:")
            .child(self.button("theme-sounds".into(),
                if self.settings.live.sounds.enabled { "☑ Enable optional theme sounds" } else { "☐ Enable optional theme sounds" },
                Command::Settings(SettingsCommand::SoundEnabled), true, window, cx))
            .child(self.settings_range(1, window, cx))
            .child("Hotkeys")
            .child("Native recorder input pending. Record controls currently exercise the typed adapter seam only.");
        for action in HotkeyAction::ALL {
            let key = format!("record-row-{}", action.as_str());
            let focus = self
                .controls
                .entry(key.clone())
                .or_insert_with(|| cx.focus_handle())
                .clone();
            let recording = self.settings.recorder.action() == Some(action);
            fields = fields.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!("{}:", action.label()))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from(key.clone()))
                                    .debug_selector(move || key.clone())
                                    .track_focus(&focus)
                                    .flex_1()
                                    .min_w_0()
                                    .border_1()
                                    .border_color(rgb(view::GOLD))
                                    .p_2()
                                    .child(self.settings.displayed.get(action).to_owned()),
                            )
                            .child(self.button(
                                format!("record-{}", action.as_str()),
                                if recording { "Cancel" } else { "Record" },
                                Command::Settings(SettingsCommand::Record(action)),
                                true,
                                window,
                                cx,
                            )),
                    ),
            );
        }
        fields = fields
            .child("Avoid single letter keys (like Shift+I) to prevent typing conflicts!")
            .child("Good examples: Cmd/Ctrl+F1, Alt+Q, Cmd/Ctrl+Shift+T")
            .child("Changes take effect as soon as settings are saved.")
            .child(self.button(
                "reset-hotkeys".into(),
                "Reset to Defaults",
                Command::Settings(SettingsCommand::Reset),
                true,
                window,
                cx,
            ));
        fields
    }
    fn settings_range(
        &mut self,
        index: usize,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let value = if index == 0 {
            self.settings.live.transparency
        } else {
            self.settings.live.sounds.volume
        };
        let key = if index == 0 {
            "transparency"
        } else {
            "sound-volume"
        };
        let focus = self
            .controls
            .entry(key.into())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        self.tab_order.push((focus.clone(), None));
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(if index == 0 {
                format!("Transparency: {value}% visible")
            } else {
                format!("{value}% volume")
            })
            .child(
                div()
                    .id(key)
                    .debug_selector(move || key.into())
                    .track_focus(&focus)
                    .relative()
                    .h(px(22.))
                    .w_full()
                    .border_1()
                    .border_color(rgb(view::GOLD))
                    .cursor_pointer()
                    .child(
                        div()
                            .h_full()
                            .w(relative(f32::from(value) / 100.))
                            .bg(rgb(view::GOLD)),
                    )
                    .child(
                        gpui::canvas(
                            move |bounds, _, cx| {
                                entity.update(cx, |s, _| {
                                    s.settings.range_bounds[index] = Some(bounds)
                                });
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |s, event: &gpui::MouseDownEvent, w, cx| {
                            w.focus(&focus, cx);
                            s.settings_range_pointer(index, event.position, w, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_move(cx.listener(move |s, event: &gpui::MouseMoveEvent, w, cx| {
                        if event.pressed_button == Some(gpui::MouseButton::Left) {
                            s.settings_range_pointer(index, event.position, w, cx);
                        }
                    })),
            )
            .child(if index == 0 {
                "More Transparent — More Solid"
            } else {
                "Muted — Louder"
            })
    }
    fn settings_range_pointer(
        &mut self,
        index: usize,
        point: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = self.settings.range_bounds[index] else {
            return;
        };
        if bounds.size.width <= px(0.) {
            return;
        }
        let value = (((point.x - bounds.left()) / bounds.size.width) * 100.)
            .round()
            .clamp(0., 100.) as u8;
        self.settings_command(
            if index == 0 {
                SettingsCommand::Transparency(value)
            } else {
                SettingsCommand::Volume(value)
            },
            window,
            cx,
        );
    }
    pub(super) fn settings_range_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !matches!(self.dialog, Some(Dialog::Settings)) {
            return false;
        }
        let key = event.keystroke.key.as_str();
        if !matches!(key, "left" | "right" | "up" | "down" | "home" | "end") {
            return false;
        }
        for (index, name) in ["transparency", "sound-volume"].into_iter().enumerate() {
            if self
                .controls
                .get(name)
                .is_some_and(|h| h.is_focused(window))
            {
                let old = if index == 0 {
                    self.settings.live.transparency
                } else {
                    self.settings.live.sounds.volume
                };
                let value = match key {
                    "home" => 0,
                    "end" => 100,
                    "left" | "down" => old.saturating_sub(1),
                    _ => old.saturating_add(1).min(100),
                };
                self.settings_command(
                    if index == 0 {
                        SettingsCommand::Transparency(value)
                    } else {
                        SettingsCommand::Volume(value)
                    },
                    window,
                    cx,
                );
                cx.stop_propagation();
                return true;
            }
        }
        false
    }
}
