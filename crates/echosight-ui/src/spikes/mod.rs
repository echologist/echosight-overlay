//! Bounded interactive experiments, not production application views.

use crate::components::text_input::{TextInput, TextInputEvent};
use gpui::{
    Bounds, Context, Entity, FocusHandle, Focusable, KeyDownEvent, MouseButton, Pixels,
    ScrollHandle, Window, WindowControlArea, canvas, div, prelude::*, px, rgb, rgba,
};

/// S5: input plus deliberately small select/slider/switch/scroll/modal probes.
/// No storage, domain mutations, theme runtime, or native window operations.
pub struct InputSpike {
    input: Entity<TextInput>,
    priority_focus: FocusHandle,
    slider_focus: FocusHandle,
    sounds_focus: FocusHandle,
    modal_focus: FocusHandle,
    cancel_focus: FocusHandle,
    confirm_focus: FocusHandle,
    priority: usize,
    select_open: bool,
    volume: u8,
    sounds: bool,
    slider_bounds: Option<Bounds<Pixels>>,
    dragging_slider: bool,
    scroll: ScrollHandle,
    modal: bool,
    confirmations: usize,
    last_event: String,
}

impl InputSpike {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            TextInput::new(cx)
                .placeholder("Add new task...")
                .max_length(100)
        });
        window.focus(&input.focus_handle(cx), cx);
        cx.subscribe(&input, |this, _, event, cx| {
            match event {
                TextInputEvent::Submit(text) => this.last_event = format!("Submitted: {text}"),
                TextInputEvent::Cancel => this.last_event = "Input: Escape".into(),
                TextInputEvent::Changed => {}
            }
            cx.notify();
        })
        .detach();
        Self {
            input,
            priority_focus: cx.focus_handle(),
            slider_focus: cx.focus_handle(),
            sounds_focus: cx.focus_handle(),
            modal_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
            confirm_focus: cx.focus_handle(),
            priority: 0,
            select_open: false,
            volume: 60,
            sounds: false,
            slider_bounds: None,
            dragging_slider: false,
            scroll: ScrollHandle::new(),
            modal: false,
            confirmations: 0,
            last_event: "Ready".into(),
        }
    }

    fn open_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = true;
        self.select_open = false;
        self.dragging_slider = false;
        window.focus(&self.cancel_focus, cx);
        cx.notify();
    }

    fn close_modal(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = false;
        if confirm {
            self.confirmations += 1;
        }
        self.last_event = if confirm {
            "Dialog: confirmed"
        } else {
            "Dialog: cancelled"
        }
        .into();
        window.focus(&self.modal_focus, cx);
        cx.notify();
    }

    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if key == "tab" {
            let handles = if self.modal {
                vec![self.cancel_focus.clone(), self.confirm_focus.clone()]
            } else {
                vec![
                    self.input.focus_handle(cx),
                    self.priority_focus.clone(),
                    self.slider_focus.clone(),
                    self.sounds_focus.clone(),
                    self.modal_focus.clone(),
                ]
            };
            let current = handles
                .iter()
                .position(|h| h.is_focused(window))
                .unwrap_or(0);
            let step = if event.keystroke.modifiers.shift {
                handles.len() - 1
            } else {
                1
            };
            window.focus(&handles[(current + step) % handles.len()], cx);
            self.select_open = false;
        } else if self.modal {
            match key {
                "escape" => self.close_modal(false, window, cx),
                // Danger dialogs never confirm on Enter, even when Confirm has focus.
                "enter" => {
                    if self.cancel_focus.is_focused(window) {
                        self.close_modal(false, window, cx);
                    }
                }
                "space" => self.close_modal(self.confirm_focus.is_focused(window), window, cx),
                _ => return,
            }
        } else if self.priority_focus.is_focused(window) {
            match key {
                "down" | "right" => self.priority = (self.priority + 1).min(2),
                "up" | "left" => self.priority = self.priority.saturating_sub(1),
                "home" => self.priority = 0,
                "end" => self.priority = 2,
                "enter" | "space" => self.select_open = !self.select_open,
                "escape" => self.select_open = false,
                _ => return,
            }
        } else if self.slider_focus.is_focused(window) {
            match key {
                "left" | "down" => self.volume = self.volume.saturating_sub(1),
                "right" | "up" => self.volume = (self.volume + 1).min(100),
                "home" => self.volume = 0,
                "end" => self.volume = 100,
                _ => return,
            }
        } else if self.sounds_focus.is_focused(window) && matches!(key, "space" | "enter") {
            self.sounds = !self.sounds;
        } else if self.modal_focus.is_focused(window) && matches!(key, "space" | "enter") {
            self.open_modal(window, cx);
        } else {
            return;
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn slide(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if let Some(bounds) = self.slider_bounds {
            self.volume =
                (((x - bounds.left()) / bounds.size.width).clamp(0., 1.) * 100.).round() as u8;
            cx.notify();
        }
    }

    fn control(
        id: &'static str,
        focus: &FocusHandle,
        window: &Window,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .debug_selector(|| id.into())
            .track_focus(focus)
            .px_3()
            .py_2()
            .border_1()
            .rounded_md()
            .cursor_pointer()
            .bg(rgba(0x302b4ddd))
            .border_color(if focus.is_focused(window) {
                rgb(0xa89cff)
            } else {
                rgb(0x554c77)
            })
    }
}

impl Render for InputSpike {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .id("s5-root")
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(rgba(0x1e1b2eee))
            .font_family(if cfg!(target_os = "windows") {
                "Segoe UI"
            } else if cfg!(target_os = "macos") {
                ".SystemUIFont"
            } else {
                "Noto Sans"
            })
            .text_color(rgb(0xf4f1ff))
            .text_sm()
            .on_key_down(cx.listener(Self::key))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging_slider && event.pressed_button == Some(MouseButton::Left) {
                    this.slide(event.position.x, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging_slider = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging_slider = false),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .window_control_area(WindowControlArea::Drag)
                            .flex_1()
                            .child("Echosight S5 · hand-rolled input"),
                    )
                    .child(
                        div()
                            .id("quit")
                            .cursor_pointer()
                            .child("Quit")
                            .on_click(|_, _, cx| cx.quit()),
                    ),
            );

        // Remove background controls from the dispatch tree while modal: no pointer
        // activation or Tab escape, and no hidden editor receiving IME text.
        if self.modal {
            return root.child(div().flex().flex_col().gap_3().p_4().border_1().border_color(rgb(0xa89cff))
                .child("Delete task? (probe only)")
                .child("Escape cancels. Tab is trapped. Confirm requires click or Space, never Enter.")
                .child(Self::control("cancel", &self.cancel_focus, window).child("Cancel")
                    .on_click(cx.listener(|this, _, window, cx| this.close_modal(false, window, cx))))
                .child(Self::control("confirm", &self.confirm_focus, window).child("Confirm")
                    .on_click(cx.listener(|this, _, window, cx| this.close_modal(true, window, cx)))));
        }

        root = root
            .child("Tab / Shift+Tab move focus. Arrow keys edit select/slider.")
            .child(self.input.clone())
            .child(format!(
                "UTF-16: {}/100 · {}",
                self.input.read(cx).text().encode_utf16().count(),
                self.last_event
            ))
            .child(
                Self::control("priority", &self.priority_focus, window)
                    .child(format!(
                        "Priority: {} ▾",
                        ["Low", "Medium", "High"][self.priority]
                    ))
                    .on_click(cx.listener(|this, _, window, cx| {
                        window.focus(&this.priority_focus, cx);
                        this.select_open = !this.select_open;
                        cx.notify();
                    })),
            );
        if self.select_open {
            root = root.child(
                div().flex().gap_2().children(
                    [
                        ("priority-low", "Low"),
                        ("priority-medium", "Medium"),
                        ("priority-high", "High"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (id, label))| {
                        div()
                            .id(id)
                            .debug_selector(|| id.into())
                            .cursor_pointer()
                            .p_2()
                            .bg(rgb(0x554c77))
                            .child(label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.priority = index;
                                this.select_open = false;
                                window.focus(&this.priority_focus, cx);
                                cx.notify();
                            }))
                    }),
                ),
            );
        }
        let entity = cx.entity();
        root.child(format!(
            "Volume: {} · click/drag bar, or arrows/Home/End",
            self.volume
        ))
        .child(
            Self::control("slider", &self.slider_focus, window)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                        window.focus(&this.slider_focus, cx);
                        this.dragging_slider = true;
                        this.slide(event.position.x, cx);
                    }),
                )
                .child(
                    div()
                        .relative()
                        .h(px(16.))
                        .w_full()
                        .bg(rgb(0x554c77))
                        .child(
                            div()
                                .absolute()
                                .h_full()
                                .w(gpui::relative(self.volume as f32 / 100.))
                                .bg(rgb(0xa89cff)),
                        )
                        .child(
                            canvas(
                                move |bounds, _, cx| {
                                    entity.update(cx, |this, _| this.slider_bounds = Some(bounds))
                                },
                                |_, _, _, _| {},
                            )
                            .size_full(),
                        ),
                ),
        )
        .child(
            Self::control("sounds", &self.sounds_focus, window)
                .child(if self.sounds {
                    "[ON] Play sounds (state only)"
                } else {
                    "[OFF] Play sounds (state only)"
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.sounds_focus, cx);
                    this.sounds = !this.sounds;
                    cx.notify();
                })),
        )
        .child(
            div()
                .id("rows")
                .debug_selector(|| "rows".into())
                .h(px(150.))
                .flex_shrink_0()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .children((1..=50).map(|i| div().h(px(28.)).child(format!("Scroll row {i}")))),
        )
        .child(
            Self::control("open-modal", &self.modal_focus, window)
                .child(format!(
                    "Open danger dialog · confirmed {} times",
                    self.confirmations
                ))
                .on_click(cx.listener(|this, _, window, cx| this.open_modal(window, cx))),
        )
    }
}

#[cfg(test)]
mod s5_tests;
