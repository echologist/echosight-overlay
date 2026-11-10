use super::*;
use gpui::{ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext};

fn open(cx: &mut TestAppContext) -> (Entity<TextInput>, &mut VisualTestContext) {
    cx.update(init);
    let (input, cx) = cx.add_window_view(|_, cx| TextInput::new(cx).multiline(4));
    cx.simulate_resize(size(px(240.), px(160.)));
    input.update_in(cx, |input, window, cx| input.focus(window, cx));
    cx.run_until_parked();
    (input, cx)
}

fn open_placeholder<'a>(
    cx: &'a mut TestAppContext,
    placeholder: &'static str,
) -> (Entity<TextInput>, &'a mut VisualTestContext) {
    cx.update(init);
    let (input, cx) =
        cx.add_window_view(|_, cx| TextInput::new(cx).multiline(8).placeholder(placeholder));
    input.update_in(cx, |input, window, cx| input.focus(window, cx));
    cx.run_until_parked();
    (input, cx)
}

fn assert_empty_caret(input: &Entity<TextInput>, cx: &mut VisualTestContext) {
    input.update_in(cx, |input, window, cx| {
        assert_eq!(input.text(), "");
        assert_eq!(input.selected_range(), 0..0);
        assert_eq!(input.cursor_offset(), 0);
        assert!(!input.selection_reversed);
        assert_eq!(
            input.selected_text_range(false, window, cx).unwrap().range,
            0..0
        );
    });
}

#[gpui::test]
fn empty_placeholder_down_backspace_does_not_edit_display_text(cx: &mut TestAppContext) {
    let (input, cx) = open_placeholder(cx, "JSON\nshare code");
    // Keep the original crash sequence uninterrupted: invalid offsets must not
    // reach Backspace's replacement range, even before a test asserts on them.
    cx.simulate_keystrokes("down backspace");
    assert_empty_caret(&input, cx);
}

#[gpui::test]
fn empty_placeholder_select_down_backspace_does_not_edit_display_text(cx: &mut TestAppContext) {
    let (input, cx) = open_placeholder(cx, "JSON\nshare code");
    cx.simulate_keystrokes("shift-down backspace");
    assert_empty_caret(&input, cx);
}

#[gpui::test]
fn empty_placeholder_arrows_keep_zero_selection_and_allow_typing(cx: &mut TestAppContext) {
    for placeholder in ["", "😀界", "JSON\nshare code", "😀\né\n\n🦀", "\n\n"] {
        let (input, cx) = open_placeholder(cx, placeholder);
        for key in ["up", "down", "shift-up", "shift-down", "down", "down"] {
            cx.simulate_keystrokes(key);
            assert_empty_caret(&input, cx);
            cx.simulate_input("🦀");
            input.read_with(cx, |input, _| {
                assert_eq!(input.text(), "🦀");
                let range = input.selected_range();
                assert_eq!(range, "🦀".len().."🦀".len());
                assert!(input.text().is_char_boundary(range.start));
                assert!(input.text().is_char_boundary(range.end));
            });
            cx.simulate_keystrokes("backspace");
            assert_empty_caret(&input, cx);
        }
        cx.simulate_keystrokes("up down shift-up shift-down backspace delete");
        assert_empty_caret(&input, cx);
    }
}

#[gpui::test]
fn multiline_placeholder_does_not_change_nonempty_navigation(cx: &mut TestAppContext) {
    let (input, cx) = open_placeholder(cx, "😀\nshare code\n");
    cx.simulate_input("abcd\nx\nabcd\n");
    cx.simulate_keystrokes("up home right right right up up");
    assert_eq!(input.read_with(cx, |i, _| i.selected_range()), 3..3);
    cx.simulate_keystrokes("shift-down shift-down");
    assert_eq!(input.read_with(cx, |i, _| i.selected_range()), 3..10);
    cx.simulate_keystrokes("right shift-up shift-up");
    input.read_with(cx, |i, _| {
        assert_eq!(i.selected_range(), 3..10);
        assert!(i.selection_reversed);
    });
    cx.simulate_keystrokes("backspace");
    assert_eq!(input.read_with(cx, |i, _| i.text().to_owned()), "abcd\n");
    cx.simulate_keystrokes("ctrl-a backspace");
    assert_empty_caret(&input, cx);
    cx.simulate_keystrokes("down shift-down up shift-up");
    assert_empty_caret(&input, cx);
    cx.simulate_input("é");
    cx.simulate_keystrokes("backspace");
    assert_empty_caret(&input, cx);
}

#[gpui::test]
fn multiline_navigation_preserves_column_and_selects_across_lines(cx: &mut TestAppContext) {
    let (input, cx) = open(cx);
    input.update_in(cx, |input, w, cx| input.set_text("abcd\nx\nabcd\n", w, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("up home right right right up up");
    assert_eq!(input.read_with(cx, |i, _| i.selected_range()), 3..3);
    cx.simulate_keystrokes("shift-down shift-down");
    assert_eq!(input.read_with(cx, |i, _| i.selected_range()), 3..10);
    cx.simulate_keystrokes("ctrl-c");
    assert_eq!(
        cx.read_from_clipboard().unwrap().text().unwrap(),
        "d\nx\nabc"
    );
    cx.simulate_keystrokes("right home shift-end");
    assert_eq!(input.read_with(cx, |i, _| i.selected_range()), 7..11);
    cx.simulate_keystrokes("end delete");
    assert_eq!(
        input.read_with(cx, |i, _| i.text().to_owned()),
        "abcd\nx\nabcd"
    );
}

#[gpui::test]
fn multiline_scroll_is_bounded_and_xy_caret_tracks_scrolled_lines(cx: &mut TestAppContext) {
    let (input, cx) = open(cx);
    input.update_in(cx, |i, w, cx| {
        i.set_text("first\nsecond\nthird\nfourth\nfifth\nsixth\n", w, cx)
    });
    cx.run_until_parked();
    let (bounds, height) = input.read_with(cx, |i, _| {
        assert!(i.scroll_y > px(0.));
        (
            i.last_bounds.unwrap(),
            i.last_layout.as_ref().unwrap().line_height,
        )
    });
    cx.simulate_event(ScrollEvent {
        position: bounds.center(),
        delta: ScrollDelta::Lines(point(0., 100.)),
        ..Default::default()
    });
    cx.run_until_parked();
    assert_eq!(input.read_with(cx, |i, _| i.scroll_y), px(0.));
    let second = point(bounds.left(), bounds.top() + height * 1.5);
    cx.simulate_click(second, Default::default());
    assert_eq!(input.read_with(cx, |i, _| i.selected_range()), 6..6);
    cx.simulate_mouse_down(second, MouseButton::Left, Default::default());
    let fourth = point(bounds.left(), bounds.top() + height * 3.5);
    cx.simulate_mouse_move(fourth, MouseButton::Left, Default::default());
    cx.simulate_mouse_up(fourth, MouseButton::Left, Default::default());
    cx.simulate_keystrokes("ctrl-c");
    assert_eq!(
        cx.read_from_clipboard().unwrap().text().unwrap(),
        "second\nthird\n"
    );
    cx.simulate_event(ScrollEvent {
        position: bounds.center(),
        delta: ScrollDelta::Lines(point(0., -100.)),
        ..Default::default()
    });
    cx.run_until_parked();
    input.update_in(cx, |i, w, cx| {
        assert_eq!(i.scroll_y, height * 3.);
        assert_eq!(
            i.character_index_for_point(point(bounds.left(), bounds.top() + height * 0.5), w, cx),
            Some(19)
        );
    });
    cx.simulate_keystrokes("left home");
    input.update_in(cx, |i, w, cx| {
        let range = i.selected_text_range(false, w, cx).unwrap().range;
        let caret = i.bounds_for_range(range, bounds, w, cx).unwrap();
        assert!(caret.top() >= bounds.top() && caret.bottom() <= bounds.bottom());
    });
}

#[gpui::test]
fn multiline_ime_non_bmp_crossline_ranges_and_geometry(cx: &mut TestAppContext) {
    let (input, cx) = open(cx);
    input.update_in(cx, |i, w, cx| {
        i.set_text("a😀\nb🦀\nc", w, cx);
        i.set_selected_text_range(1..7, w, cx);
        let mut actual = None;
        assert_eq!(
            i.text_for_range(1..7, &mut actual, w, cx).as_deref(),
            Some("😀\nb🦀")
        );
        assert_eq!(actual, Some(1..7));
        i.replace_and_mark_text_in_range(None, "界\n😀", Some(2..4), w, cx);
        assert_eq!(i.text(), "a界\n😀\nc");
        assert_eq!(i.marked_text_range(w, cx), Some(1..5));
        assert_eq!(i.selected_text_range(false, w, cx).unwrap().range, 3..5);
    });
    cx.run_until_parked();
    input.update_in(cx, |i, w, cx| {
        let bounds = i.last_bounds.unwrap();
        let height = i.last_layout.as_ref().unwrap().line_height;
        let rects = i.selection_bounds_for_range(1..5, bounds, w, cx);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[1].top() - rects[0].top(), height);
        assert_eq!(i.bounds_for_range(1..5, bounds, w, cx), Some(rects[0]));
        assert_eq!(
            i.character_index_for_point(point(rects[1].left(), rects[1].center().y), w, cx),
            Some(3)
        );
        i.replace_text_in_range(None, "好\n🦀", w, cx);
        assert_eq!(i.text(), "a好\n🦀\nc");
        assert_eq!(i.marked_range(), None);
        assert_eq!(i.selected_text_range(false, w, cx).unwrap().range, 5..5);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("shift-left ctrl-x ctrl-v");
    assert_eq!(
        input.read_with(cx, |i, _| i.text().to_owned()),
        "a好\n🦀\nc"
    );
}

#[gpui::test]
fn multiline_all_input_paths_keep_newlines_and_trailing_empty_lines(cx: &mut TestAppContext) {
    let (input, cx) = open(cx);
    input.update_in(cx, |i, w, cx| i.set_text("a\r\nb\rc\n", w, cx));
    cx.run_until_parked();
    assert_eq!(input.read_with(cx, |i, _| i.text().to_owned()), "a\nb\nc\n");
    input.read_with(cx, |i, _| {
        let layout = i.last_layout.as_ref().unwrap();
        assert_eq!(
            layout
                .lines
                .iter()
                .map(|(s, l)| (*s, l.text.as_ref()))
                .collect::<Vec<_>>(),
            vec![(0, "a"), (2, "b"), (4, "c"), (6, "")]
        );
        assert_eq!(i.last_bounds.unwrap().size.height, layout.line_height * 4.);
    });
    cx.write_to_clipboard(ClipboardItem::new_string("😀\r\n\rZ".into()));
    cx.simulate_keystrokes("ctrl-v enter");
    assert_eq!(
        input.read_with(cx, |i, _| i.text().to_owned()),
        "a\nb\nc\n😀\n\nZ\n"
    );
    cx.simulate_keystrokes("ctrl-a ctrl-x ctrl-v");
    assert_eq!(
        input.read_with(cx, |i, _| i.text().to_owned()),
        "a\nb\nc\n😀\n\nZ\n"
    );
    input.update_in(cx, |i, w, cx| {
        i.replace_and_mark_text_in_range(Some(0..1), "x\ny", Some(3..3), w, cx);
        i.unmark_text(w, cx);
        assert!(i.text().starts_with("x\ny\nb\n"));
    });
}

#[gpui::test]
fn multiline_ime_crlf_selection_uses_normalized_utf16_offsets(cx: &mut TestAppContext) {
    let (input, cx) = open(cx);
    input.update_in(cx, |i, w, cx| {
        i.replace_and_mark_text_in_range(None, "x\r\n😀z", Some(3..5), w, cx);
        assert_eq!(i.text(), "x\n😀z");
        assert_eq!(i.selected_text_range(false, w, cx).unwrap().range, 2..4);
    });
}

#[gpui::test]
fn multiline_long_line_horizontal_scroll_and_buffer_endpoints(cx: &mut TestAppContext) {
    let (input, cx) = open(cx);
    let text = format!("\n{}😀\n\n", "W".repeat(3000));
    input.update_in(cx, |i, w, cx| i.set_text(text.clone(), w, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("up up end");
    input.update_in(cx, |i, w, cx| {
        assert!(i.scroll_x > px(0.));
        assert_eq!(i.text(), text);
        let bounds = i.last_bounds.unwrap();
        let caret = i.bounds_for_range(3003..3003, bounds, w, cx).unwrap();
        assert!(caret.left() >= bounds.left() && caret.right() < bounds.right());
    });
    cx.simulate_keystrokes("ctrl-home backspace ctrl-end delete");
    assert_eq!(input.read_with(cx, |i, _| i.text().to_owned()), text);
    cx.simulate_keystrokes("ctrl-a ctrl-c");
    assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), text);
}
