// SPDX-License-Identifier: Apache-2.0
// Adapted from Zed Industries' crates/gpui/examples/input.rs at
// 4ed72e0135aaab74a43914491c915dd6cbd10c67 (Apache-2.0).
// Modified for Echosight: length limit, events, styles, single-line normalization,
// selection/IME fixes and tests. See ../../../THIRD_PARTY.md and ../../../LICENSE-APACHE.

//! Text input built directly on GPUI's [`EntityInputHandler`].
//! Single-line by default; opt-in multiline uses the same buffer, selection and IME
//! implementation with logical-line layout, bounded scrolling and XY hit testing.
//!
//! Follows zed's `crates/gpui/examples/input.rs` at the pinned rev and adds what the
//! overlay needs: a maximum length, a placeholder, a focus ring, Enter/Escape events,
//! shift+Home/End selection, double-click select-all and a cut/copy/paste that works
//! on every platform (`ctrl-` and `cmd-` bindings are both registered).
//!
//! The input stores UTF-8 offsets internally and converts at the platform boundary
//! (`UTF16Selection`, IME ranges), exactly like the upstream example. Cursor boundaries
//! use grapheme clusters (`unicode-segmentation`), so a flag emoji is one backspace.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, Hsla, KeyBinding,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window, actions, div,
    fill, point, prelude::*, px, relative, rgba, size,
};
use unicode_segmentation::UnicodeSegmentation;

actions!(
    echosight_text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        SelectHome,
        SelectEnd,
        Up,
        Down,
        SelectUp,
        SelectDown,
        DocumentHome,
        DocumentEnd,
        SelectDocumentHome,
        SelectDocumentEnd,
        Paste,
        Cut,
        Copy,
        Submit,
        Cancel,
    ]
);

/// Key context the bindings are scoped to.
pub const KEY_CONTEXT: &str = "EchosightTextInput";

/// Registers the key bindings. Call once at application start (and in tests).
pub fn init(cx: &mut App) {
    let ctx = Some(KEY_CONTEXT);
    let multiline = Some("EchosightMultiline");
    cx.bind_keys([
        KeyBinding::new("up", Up, multiline),
        KeyBinding::new("down", Down, multiline),
        KeyBinding::new("shift-up", SelectUp, multiline),
        KeyBinding::new("shift-down", SelectDown, multiline),
        KeyBinding::new("ctrl-home", DocumentHome, multiline),
        KeyBinding::new("ctrl-end", DocumentEnd, multiline),
        KeyBinding::new("ctrl-shift-home", SelectDocumentHome, multiline),
        KeyBinding::new("ctrl-shift-end", SelectDocumentEnd, multiline),
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
        KeyBinding::new("enter", Submit, ctx),
        KeyBinding::new("escape", Cancel, ctx),
        // `secondary-` needs the OS at load time; registering both spellings is simpler
        // and harmless (`cmd` is Super on Linux/Windows, where nothing else uses it).
        KeyBinding::new("ctrl-a", SelectAll, ctx),
        KeyBinding::new("ctrl-c", Copy, ctx),
        KeyBinding::new("ctrl-x", Cut, ctx),
        KeyBinding::new("ctrl-v", Paste, ctx),
        KeyBinding::new("cmd-a", SelectAll, ctx),
        KeyBinding::new("cmd-c", Copy, ctx),
        KeyBinding::new("cmd-x", Cut, ctx),
        KeyBinding::new("cmd-v", Paste, ctx),
    ]);
}

/// Events emitted by [`TextInput`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
    /// The text changed (typing, IME commit, paste, cut, delete).
    Changed,
    /// Enter was pressed; carries the current text.
    Submit(String),
    /// Escape was pressed.
    Cancel,
}

/// Colours of the input, so the theme runtime (Phase 3) can drive them.
#[derive(Debug, Clone, Copy)]
pub struct TextInputStyle {
    pub background: Hsla,
    pub border: Hsla,
    pub border_focused: Hsla,
    pub text: Hsla,
    pub placeholder: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            background: rgba(0x00000066).into(),
            border: rgba(0xa89cff55).into(),
            border_focused: rgba(0xa89cffff).into(),
            text: rgba(0xf4f1ffff).into(),
            placeholder: rgba(0xf4f1ff66).into(),
            cursor: rgba(0xa89cffff).into(),
            selection: rgba(0x7b6ff066).into(),
        }
    }
}

/// Text input entity. Render it with `cx.new(...)` and `.child(entity)`.
/// Single-line by default; call `.multiline(rows)` for a textarea.
pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    max_length: Option<usize>,
    style: TextInputStyle,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<TextLayout>,
    multiline_rows: Option<usize>,
    scroll_y: Pixels,
    reveal_cursor: bool,
    preferred_x: Option<Pixels>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
    is_selecting: bool,
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: SharedString::default(),
            placeholder: SharedString::default(),
            max_length: None,
            style: TextInputStyle::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            multiline_rows: None,
            scroll_y: px(0.),
            reveal_cursor: true,
            preferred_x: None,
            last_bounds: None,
            scroll_x: px(0.),
            is_selecting: false,
        }
    }

    /// Opt in to a multiline buffer with a fixed-height viewport.
    /// CRLF and CR become LF, as in HTMLTextAreaElement.value; LF is never a space.
    pub fn multiline(mut self, rows: usize) -> Self {
        self.multiline_rows = Some(rows.max(1));
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Maximum UTF-16 code units, matching the Electron `maxlength=100` attribute.
    /// Truncation never splits a Unicode scalar (a surrogate pair).
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn selected_range(&self) -> Range<usize> {
        self.selected_range.clone()
    }

    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked_range.clone()
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus_handle, cx);
    }

    /// Replaces the whole text (clamped to `max_length`) and puts the cursor at the end.
    pub fn set_text(
        &mut self,
        text: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.clamp(text.into(), self.content.encode_utf16().count());
        self.content = text.into();
        let end = self.content.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.emit(TextInputEvent::Changed);
        cx.notify();
    }

    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_text("", window, cx);
    }

    /// Normalize every insertion path, then truncate at a scalar boundary.
    fn clamp(&self, new_text: String, replaced_units: usize) -> String {
        let new_text = if self.multiline_rows.is_some() {
            new_text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            new_text.replace(['\r', '\n'], " ")
        };
        let Some(max) = self.max_length else {
            return new_text;
        };
        let remaining = self
            .content
            .encode_utf16()
            .count()
            .saturating_sub(replaced_units);
        let mut room = max.saturating_sub(remaining);
        new_text
            .chars()
            .take_while(|ch| {
                if ch.len_utf16() > room {
                    false
                } else {
                    room -= ch.len_utf16();
                    true
                }
            })
            .collect()
    }

    fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), window, cx);
        } else {
            self.move_to(self.selected_range.start, window, cx)
        }
    }

    fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), window, cx);
        } else {
            self.move_to(self.selected_range.end, window, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), window, cx);
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), window, cx);
    }

    fn select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, window, cx);
        self.select_to(self.content.len(), window, cx)
    }

    fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_start(), window, cx);
    }

    fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_end(), window, cx);
    }

    fn select_home(&mut self, _: &SelectHome, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.line_start(), window, cx);
    }

    fn select_end(&mut self, _: &SelectEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.line_end(), window, cx);
    }

    fn line_start(&self) -> usize {
        if self.multiline_rows.is_none() {
            return 0;
        }
        self.content[..self.cursor_offset()]
            .rfind('\n')
            .map_or(0, |i| i + 1)
    }

    fn line_end(&self) -> usize {
        if self.multiline_rows.is_none() {
            return self.content.len();
        }
        let cursor = self.cursor_offset();
        self.content[cursor..]
            .find('\n')
            .map_or(self.content.len(), |i| cursor + i)
    }

    fn vertical(&mut self, down: bool, select: bool, window: &Window, cx: &mut Context<Self>) {
        // Empty-buffer layout shapes the placeholder, not editable text. Its
        // line offsets must never become caret or selection offsets.
        if self.content.is_empty() {
            return;
        }
        let Some(layout) = self.last_layout.as_ref().filter(|l| l.text == self.content) else {
            return;
        };
        let position = layout.position(self.cursor_offset());
        let x = self.preferred_x.unwrap_or(position.x);
        let y = position.y + layout.line_height * if down { 1. } else { -1. };
        let offset = layout.index(point(x, y));
        if select {
            self.select_to(offset, window, cx);
        } else {
            self.move_to(offset, window, cx);
        }
        self.preferred_x = Some(x);
    }

    fn up(&mut self, _: &Up, w: &mut Window, cx: &mut Context<Self>) {
        self.vertical(false, false, w, cx);
    }
    fn down(&mut self, _: &Down, w: &mut Window, cx: &mut Context<Self>) {
        self.vertical(true, false, w, cx);
    }
    fn select_up(&mut self, _: &SelectUp, w: &mut Window, cx: &mut Context<Self>) {
        self.vertical(false, true, w, cx);
    }
    fn select_down(&mut self, _: &SelectDown, w: &mut Window, cx: &mut Context<Self>) {
        self.vertical(true, true, w, cx);
    }
    fn document_home(&mut self, _: &DocumentHome, w: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, w, cx);
    }
    fn document_end(&mut self, _: &DocumentEnd, w: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), w, cx);
    }
    fn select_document_home(
        &mut self,
        _: &SelectDocumentHome,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(0, w, cx);
    }
    fn select_document_end(
        &mut self,
        _: &SelectDocumentEnd,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.content.len(), w, cx);
    }

    fn scroll(&mut self, event: &gpui::ScrollEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.multiline_rows.is_none() {
            cx.propagate();
            return;
        }
        let (Some(layout), Some(bounds)) = (&self.last_layout, self.last_bounds) else {
            return;
        };
        let delta = event.delta.pixel_delta(layout.line_height);
        let max_y =
            (layout.line_height * layout.lines.len() as f32 - bounds.size.height).max(px(0.));
        let max_x = (layout
            .lines
            .iter()
            .map(|(_, l)| l.width())
            .fold(px(0.), Pixels::max)
            - bounds.size.width
            + px(2.))
        .max(px(0.));
        self.scroll_y = (self.scroll_y - delta.y).clamp(px(0.), max_y);
        self.scroll_x = (self.scroll_x - delta.x).clamp(px(0.), max_x);
        // Wheel browsing must not snap back to an unchanged caret on every paint.
        self.reveal_cursor = false;
        cx.stop_propagation();
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            self.select_to(prev, window, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            self.select_to(next, window, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline_rows.is_some() {
            self.replace_text_in_range(None, "\n", window, cx);
        } else {
            cx.emit(TextInputEvent::Submit(self.content.to_string()));
        }
    }

    fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::Cancel);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        if event.click_count >= 2 {
            self.select_all(&SelectAll, window, cx);
            return;
        }
        let index = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(index, window, cx);
        } else {
            self.move_to(index, window, cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), window, cx);
        }
    }

    fn move_to(&mut self, offset: usize, window: &Window, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if self.multiline_rows.is_none() {
            if position.y < bounds.top() {
                return 0;
            }
            if position.y > bounds.bottom() {
                return self.content.len();
            }
        }
        if line.text != self.content {
            return self.cursor_offset();
        }
        line.index(position - bounds.origin + point(self.scroll_x, self.scroll_y))
    }

    fn select_to(&mut self, offset: usize, window: &Window, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    /// Range an edit applies to: explicit UTF-16 range, else the IME marked range, else
    /// the selection.
    fn edit_range(&self, range_utf16: Option<&Range<usize>>) -> Range<usize> {
        range_utf16
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone())
    }

    fn splice(&mut self, range: &Range<usize>, new_text: &str) {
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_range = self.range_from_utf16(&range_utf16);
        self.selection_reversed = false;
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify();
    }

    fn text_length_utf16(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.content.encode_utf16().count())
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.marked_range.take().is_some() {
            cx.emit(TextInputEvent::Changed);
        }
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = self.edit_range(range_utf16.as_ref());
        let replaced_chars = self.content[range.clone()].encode_utf16().count();
        let new_text = self.clamp(new_text.to_owned(), replaced_chars);
        self.splice(&range, &new_text);
        let cursor = range.start + new_text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range.take();
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.emit(TextInputEvent::Changed);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Composition selections refer to the platform's insertion, before CRLF
        // normalization. Subtract only paired CRs before each UTF-16 endpoint.
        let new_selected_range_utf16 = new_selected_range_utf16.map(|range| {
            let normalized_offset = |offset: usize| {
                if self.multiline_rows.is_none() {
                    return offset;
                }
                let mut units = 0;
                let mut removed = 0;
                let mut previous_cr = false;
                for ch in new_text.chars() {
                    if units >= offset {
                        break;
                    }
                    if ch == '\n' && previous_cr {
                        removed += 1;
                    }
                    previous_cr = ch == '\r';
                    units += ch.len_utf16();
                }
                offset.saturating_sub(removed)
            };
            normalized_offset(range.start)..normalized_offset(range.end)
        });
        let range = self.edit_range(range_utf16.as_ref());
        let replaced_chars = self.content[range.clone()].encode_utf16().count();
        let new_text = self.clamp(new_text.to_owned(), replaced_chars);
        self.splice(&range, &new_text);
        if new_text.is_empty() {
            self.marked_range = None;
        } else {
            self.marked_range = Some(range.start..range.start + new_text.len());
        }
        let insertion_start_utf16 = self.offset_to_utf16(range.start);
        let inserted_length_utf16 = new_text.encode_utf16().count();
        self.selected_range = new_selected_range_utf16
            .map(|selected| {
                self.range_from_utf16(
                    &(insertion_start_utf16 + selected.start.min(inserted_length_utf16)
                        ..insertion_start_utf16 + selected.end.min(inserted_length_utf16)),
                )
            })
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.selection_reversed = false;
        self.reveal_cursor = true;
        self.preferred_x = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Platform candidate UI wants the first line rectangle, not a giant box
        // spanning unrelated text. The plural API exposes every selected line.
        self.selection_bounds_for_range(range_utf16, bounds, _window, _cx)
            .into_iter()
            .next()
    }

    fn selection_bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Vec<Bounds<Pixels>> {
        let Some(layout) = &self.last_layout else {
            return vec![];
        };
        if layout.text != self.content {
            return vec![];
        }
        let origin = bounds.origin - point(self.scroll_x, self.scroll_y);
        if self.content.is_empty() {
            return vec![Bounds::new(origin, size(px(0.), layout.line_height))];
        }
        layout.range_bounds(self.range_from_utf16(&range_utf16), origin)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        if self.content.is_empty() {
            return Some(0);
        }
        let last_layout = self.last_layout.as_ref()?;
        if last_layout.text != self.content {
            return None;
        }
        let utf8_index = last_layout.index(line_point + gpui::point(self.scroll_x, self.scroll_y));
        Some(self.offset_to_utf16(utf8_index))
    }
}

/// Logical lines retain their byte offsets, including empty and trailing lines.
/// GPUI owns glyph shaping/hit testing; buffer editing remains in TextInput.
#[derive(Clone)]
struct TextLayout {
    text: SharedString,
    lines: Vec<(usize, ShapedLine)>,
    line_height: Pixels,
}

impl TextLayout {
    fn row_for_offset(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|(start, _)| *start <= offset)
            .saturating_sub(1)
    }

    fn position(&self, offset: usize) -> Point<Pixels> {
        let row = self.row_for_offset(offset);
        let (start, line) = &self.lines[row];
        point(
            line.x_for_index(offset.saturating_sub(*start).min(line.len())),
            self.line_height * row as f32,
        )
    }

    fn index(&self, position: Point<Pixels>) -> usize {
        let row =
            ((position.y / self.line_height).floor().max(0.) as usize).min(self.lines.len() - 1);
        let (start, line) = &self.lines[row];
        start + line.closest_index_for_x(position.x)
    }

    fn range_bounds(&self, range: Range<usize>, origin: Point<Pixels>) -> Vec<Bounds<Pixels>> {
        let first = self.row_for_offset(range.start);
        let last = self.row_for_offset(range.end);
        (first..=last)
            .filter_map(|row| {
                let (start, line) = &self.lines[row];
                // A selection ending at the start of a line does not select that line.
                if !range.is_empty() && *start == range.end {
                    return None;
                }
                let left = line.x_for_index(range.start.saturating_sub(*start).min(line.len()));
                let mut right = line.x_for_index(range.end.saturating_sub(*start).min(line.len()));
                // Give the selected newline its own visible cell, including empty lines.
                if range.end > start + line.len() {
                    right += px(4.);
                }
                Some(Bounds::new(
                    origin + point(left, self.line_height * row as f32),
                    size((right - left).max(px(0.)), self.line_height),
                ))
            })
            .collect()
    }
}

/// The custom element that shapes, paints and registers the input handler.
struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    layout: TextLayout,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height =
            (window.line_height() * self.input.read(cx).multiline_rows.unwrap_or(1) as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let colors = input.style;
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), colors.placeholder)
        } else {
            (content, colors.text)
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let mut start = 0;
        let lines = display_text
            .split('\n')
            .map(|text| {
                let end = start + text.len();
                let run = TextRun {
                    len: text.len(),
                    font: style.font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let runs = if let Some(mark) = input.marked_range.as_ref() {
                    let from = mark.start.clamp(start, end) - start;
                    let to = mark.end.clamp(start, end) - start;
                    vec![
                        TextRun {
                            len: from,
                            ..run.clone()
                        },
                        TextRun {
                            len: to - from,
                            underline: Some(UnderlineStyle {
                                color: Some(text_color),
                                thickness: px(1.),
                                wavy: false,
                            }),
                            ..run.clone()
                        },
                        TextRun {
                            len: text.len() - to,
                            ..run
                        },
                    ]
                    .into_iter()
                    .filter(|r| r.len > 0)
                    .collect()
                } else {
                    vec![run]
                };
                let line =
                    window
                        .text_system()
                        .shape_line(text.to_owned().into(), font_size, &runs, None);
                let row = (start, line);
                start = end + 1;
                row
            })
            .collect();
        let layout = TextLayout {
            text: input.content.clone(),
            lines,
            line_height,
        };
        let cursor_pos = if input.content.is_empty() {
            point(px(0.), px(0.))
        } else {
            layout.position(cursor)
        };
        let visible_width = (bounds.size.width - px(2.)).max(px(0.));
        let mut scroll_x = input.scroll_x;
        let mut scroll_y = input.scroll_y;
        if input.reveal_cursor || input.last_bounds != Some(bounds) {
            scroll_x = scroll_x
                .min(cursor_pos.x)
                .max(cursor_pos.x - visible_width)
                .max(px(0.));
            scroll_y = scroll_y
                .min(cursor_pos.y)
                .max(cursor_pos.y + line_height - bounds.size.height)
                .max(px(0.));
        }
        let max_y = (line_height * layout.lines.len() as f32 - bounds.size.height).max(px(0.));
        scroll_y = scroll_y.min(max_y);
        let origin = bounds.origin - point(scroll_x, scroll_y);
        let selection = if selected_range.is_empty() {
            vec![]
        } else {
            layout
                .range_bounds(selected_range, origin)
                .into_iter()
                .map(|b| fill(b, colors.selection))
                .collect()
        };
        let cursor = input.selected_range.is_empty().then(|| {
            fill(
                Bounds::new(origin + cursor_pos, size(px(2.), line_height)),
                colors.cursor,
            )
        });
        self.input.update(cx, |input, _| {
            input.scroll_x = scroll_x;
            input.scroll_y = scroll_y;
            input.reveal_cursor = false;
        });
        PrepaintState {
            layout,
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let input = self.input.read(cx);
        let origin = bounds.origin - point(input.scroll_x, input.scroll_y);
        let layout = &prepaint.layout;
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for selection in prepaint.selection.drain(..) {
                window.paint_quad(selection);
            }
            for (row, (_, line)) in layout.lines.iter().enumerate() {
                let position = origin + point(px(0.), layout.line_height * row as f32);
                if position.y + layout.line_height <= bounds.top() || position.y >= bounds.bottom()
                {
                    continue;
                }
                if let Err(e) = line.paint(
                    position,
                    layout.line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                ) {
                    log::error!("text input paint failed: {e:#}");
                }
            }
            if focus_handle.is_focused(window)
                && let Some(cursor) = prepaint.cursor.take()
            {
                window.paint_quad(cursor);
            }
        });
        self.input.update(cx, |input, _| {
            input.last_layout = Some(prepaint.layout.clone());
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let colors = self.style;
        div()
            .flex()
            .key_context(if self.multiline_rows.is_some() {
                "EchosightTextInput EchosightMultiline"
            } else {
                KEY_CONTEXT
            })
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::document_home))
            .on_action(cx.listener(Self::document_end))
            .on_action(cx.listener(Self::select_document_home))
            .on_action(cx.listener(Self::select_document_end))
            .on_scroll(cx.listener(Self::scroll))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::cancel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(colors.background)
            .border_1()
            .border_color(if focused {
                colors.border_focused
            } else {
                colors.border
            })
            .text_color(colors.text)
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
#[path = "multiline_tests.rs"]
mod multiline_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    fn open(cx: &mut TestAppContext) -> (Entity<TextInput>, &mut VisualTestContext) {
        cx.update(init);
        let (input, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(cx)
                .placeholder("Add new task...")
                .max_length(100)
        });
        input.update_in(cx, |input, window, cx| input.focus(window, cx));
        cx.run_until_parked();
        (input, cx)
    }

    #[gpui::test]
    fn long_input_keeps_caret_visible_and_home_restores_origin(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_resize(size(px(240.), px(80.)));
        cx.run_until_parked();
        cx.simulate_input(&"W".repeat(100));
        input.update_in(cx, |input, window, cx| {
            let bounds = input.last_bounds.unwrap();
            let caret = input
                .bounds_for_range(100..100, bounds, window, cx)
                .unwrap();
            assert!(caret.left() >= bounds.left() && caret.right() < bounds.right());
        });
        cx.simulate_keystrokes("home");
        input.update_in(cx, |input, window, cx| {
            let bounds = input.last_bounds.unwrap();
            let caret = input.bounds_for_range(0..0, bounds, window, cx).unwrap();
            assert_eq!(caret.left(), bounds.left());
        });
    }

    #[gpui::test]
    fn keystrokes_insert_in_the_middle(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_keystrokes("a b left c");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "acb"
        );
    }

    #[gpui::test]
    fn max_length_counts_utf16_and_set_text_replaces_all(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input(&"a".repeat(98));
        cx.simulate_input("😀b");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().encode_utf16().count()),
            100
        );
        input.update_in(cx, |input, window, cx| {
            input.set_text("replacement", window, cx);
            assert_eq!(input.text(), "replacement");
            input.set_text("😀".repeat(60), window, cx);
            assert_eq!(input.text().encode_utf16().count(), 100);
            assert_eq!(input.text().chars().count(), 50);
            input.clear(window, cx);
            assert_eq!(input.text(), "");
        });
    }

    #[gpui::test]
    fn ime_utf16_ranges_clamp_at_scalar_and_limit_boundaries(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        input.update_in(cx, |input, window, cx| {
            input.set_text("a😀z", window, cx);
            let mut actual = None;
            assert_eq!(
                input
                    .text_for_range(1..3, &mut actual, window, cx)
                    .as_deref(),
                Some("😀")
            );
            assert_eq!(actual, Some(1..3));
            input.set_selected_text_range(2..999, window, cx);
            assert_eq!(
                input.selected_text_range(false, window, cx).unwrap().range,
                3..4
            );
            input.replace_text_in_range(Some(1..3), "é", window, cx);
            assert_eq!(input.text(), "aéz");
            input.set_text("a".repeat(98), window, cx);
            input.replace_and_mark_text_in_range(None, "😀x", Some(3..3), window, cx);
            assert_eq!(input.text_length_utf16(window, cx), Some(100));
            assert_eq!(input.marked_text_range(window, cx), Some(98..100));
            assert_eq!(
                input.selected_text_range(false, window, cx).unwrap().range,
                100..100
            );
            input.replace_text_in_range(None, "好", window, cx);
            assert_eq!(input.text_length_utf16(window, cx), Some(99));
            assert!(input.text().ends_with('好'));
            assert_eq!(input.marked_range(), None);
            input.replace_and_mark_text_in_range(None, "x", None, window, cx);
            input.unmark_text(window, cx);
            assert_eq!(input.marked_range(), None);
        });
    }

    #[gpui::test]
    fn all_insertion_paths_are_single_line(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        input.update_in(cx, |input, window, cx| {
            input.set_text("a\nb\r", window, cx);
            assert_eq!(input.text(), "a b ");
            input.replace_text_in_range(None, "c\nd", window, cx);
            assert_eq!(input.text(), "a b c d");
        });
    }

    #[gpui::test]
    fn click_refocuses_after_blur(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        let bounds = input.read_with(cx, |input, _| input.last_bounds.unwrap());
        cx.update(|window, cx| window.blur(cx));
        cx.simulate_click(bounds.center(), Default::default());
        cx.simulate_keystrokes("a");
        assert_eq!(input.read_with(cx, |input, _| input.text().to_owned()), "a");
    }

    #[gpui::test]
    fn combining_and_zwj_graphemes_delete_as_units(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("e\u{301}👩‍💻");
        cx.simulate_keystrokes("backspace");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "e\u{301}"
        );
        cx.simulate_keystrokes("home delete");
        assert_eq!(input.read_with(cx, |input, _| input.text().to_owned()), "");
    }

    #[gpui::test]
    fn command_clipboard_bindings_are_registered(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("é😀");
        cx.simulate_keystrokes("cmd-a cmd-x cmd-v");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "é😀"
        );
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("é😀")
        );
    }

    #[gpui::test]
    fn typing_inserts_at_cursor(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("héllo");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "héllo"
        );
        assert_eq!(input.read_with(cx, |input, _| input.selected_range()), 6..6);
    }

    #[gpui::test]
    fn backspace_removes_grapheme_and_delete_removes_forward(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("ab🇳🇱c");
        cx.simulate_keystrokes("backspace backspace");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "ab"
        );
        cx.simulate_keystrokes("home delete");
        assert_eq!(input.read_with(cx, |input, _| input.text().to_owned()), "b");
        cx.simulate_keystrokes("backspace");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "b",
            "backspace at start is a no-op"
        );
    }

    #[gpui::test]
    fn shift_arrows_and_home_end_select(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("hello");
        cx.simulate_keystrokes("shift-left shift-left");
        assert_eq!(input.read_with(cx, |input, _| input.selected_range()), 3..5);
        cx.simulate_keystrokes("left");
        assert_eq!(input.read_with(cx, |input, _| input.selected_range()), 3..3);
        cx.simulate_keystrokes("shift-home");
        assert_eq!(input.read_with(cx, |input, _| input.selected_range()), 0..3);
        cx.simulate_keystrokes("end shift-home");
        assert_eq!(input.read_with(cx, |input, _| input.selected_range()), 0..5);
        cx.simulate_input("x");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "x",
            "typing replaces the selection"
        );
    }

    #[gpui::test]
    fn select_all_copy_cut_paste_use_the_clipboard(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("task one");
        cx.simulate_keystrokes("ctrl-a ctrl-c");
        assert_eq!(
            cx.read_from_clipboard().and_then(|i| i.text()).as_deref(),
            Some("task one")
        );
        cx.simulate_keystrokes("end ctrl-v");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "task onetask one"
        );
        cx.simulate_keystrokes("ctrl-a ctrl-x");
        assert_eq!(input.read_with(cx, |input, _| input.text().to_owned()), "");
        cx.write_to_clipboard(ClipboardItem::new_string("multi\nline\r\n".to_owned()));
        cx.simulate_keystrokes("ctrl-v");
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "multi line  "
        );
    }

    #[gpui::test]
    fn max_length_clamps_typing_and_paste(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input(&"a".repeat(98));
        cx.simulate_input("bcd");
        assert_eq!(
            input
                .read_with(cx, |input, _| input.text().to_owned())
                .chars()
                .count(),
            100
        );
        assert!(
            input
                .read_with(cx, |input, _| input.text().to_owned())
                .ends_with("bc")
        );
        cx.simulate_keystrokes("ctrl-a");
        cx.write_to_clipboard(ClipboardItem::new_string("é".repeat(150)));
        cx.simulate_keystrokes("ctrl-v");
        assert_eq!(
            input
                .read_with(cx, |input, _| input.text().to_owned())
                .chars()
                .count(),
            100
        );
        assert!(
            input
                .read_with(cx, |input, _| input.text().to_owned())
                .chars()
                .all(|c| c == 'é')
        );
    }

    #[gpui::test]
    fn enter_emits_submit_and_escape_emits_cancel(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|_, cx| {
            cx.subscribe(&input, move |_, event, _| {
                sink.borrow_mut().push(event.clone())
            })
            .detach();
        });
        cx.simulate_input("buy milk");
        cx.simulate_keystrokes("enter escape");
        let events = events.borrow();
        assert!(events.contains(&TextInputEvent::Submit("buy milk".to_owned())));
        assert!(events.contains(&TextInputEvent::Cancel));
        assert_eq!(
            events
                .iter()
                .filter(|e| **e == TextInputEvent::Changed)
                .count(),
            8
        );
    }

    #[gpui::test]
    fn empty_input_exposes_caret_bounds_for_ime(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        input.update_in(cx, |input, window, cx| {
            let bounds = input.last_bounds.unwrap();
            let caret = input
                .bounds_for_range(0..0, bounds, window, cx)
                .expect("empty input has a caret");
            assert_eq!(caret.left(), bounds.left());
            assert_eq!(
                input.character_index_for_point(bounds.center(), window, cx),
                Some(0)
            );
        });
    }

    #[gpui::test]
    fn unmark_notifies_committed_composition_once(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        let changes = std::rc::Rc::new(std::cell::Cell::new(0));
        let sink = changes.clone();
        cx.update(|_, cx| {
            cx.subscribe(&input, move |_, event, _| {
                if *event == TextInputEvent::Changed {
                    sink.set(sink.get() + 1);
                }
            })
            .detach();
        });
        input.update_in(cx, |input, window, cx| {
            input.replace_and_mark_text_in_range(None, "你", None, window, cx);
            input.unmark_text(window, cx);
            input.unmark_text(window, cx);
        });
        cx.run_until_parked();
        assert_eq!(changes.get(), 1);
        assert_eq!(
            input.read_with(cx, |input, _| input.text().to_owned()),
            "你"
        );
    }

    #[gpui::test]
    fn ime_preedit_marks_then_commits(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("a");
        input.update_in(cx, |input, window, cx| {
            input.replace_and_mark_text_in_range(None, "ni", Some(0..2), window, cx);
            assert_eq!(input.text(), "ani");
            assert_eq!(input.marked_range(), Some(1..3));
            input.replace_and_mark_text_in_range(None, "你", Some(1..1), window, cx);
            assert_eq!(input.text(), "a你");
            assert_eq!(input.marked_range(), Some(1..4));
            input.replace_text_in_range(None, "你好", window, cx);
            assert_eq!(input.text(), "a你好");
            assert_eq!(input.marked_range(), None);
            assert_eq!(input.selected_range(), 7..7);
            assert_eq!(input.text_length_utf16(window, cx), Some(3));
        });
    }

    #[gpui::test]
    fn mouse_click_moves_cursor_and_drag_selects(cx: &mut TestAppContext) {
        let (input, cx) = open(cx);
        cx.simulate_input("hello world");
        let bounds = input
            .read_with(cx, |input, _| input.last_bounds)
            .expect("input painted after typing");
        let y = bounds.center().y;
        cx.simulate_mouse_down(
            point(bounds.left(), y),
            MouseButton::Left,
            Default::default(),
        );
        cx.simulate_mouse_move(
            point(bounds.right(), y),
            MouseButton::Left,
            Default::default(),
        );
        cx.simulate_mouse_up(
            point(bounds.right(), y),
            MouseButton::Left,
            Default::default(),
        );
        cx.run_until_parked();
        assert_eq!(
            input.read_with(cx, |input, _| input.selected_range()),
            0..11
        );
        cx.simulate_click(point(bounds.left(), y), Default::default());
        cx.run_until_parked();
        assert_eq!(input.read_with(cx, |input, _| input.selected_range()), 0..0);
    }
}
