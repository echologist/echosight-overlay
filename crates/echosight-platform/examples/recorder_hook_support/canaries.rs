//! Shared canary bindings: keymap tests are not Windows timeout/pump evidence.
use gpui::{KeyBinding, actions};

actions!(s9, [Canary, ShortPrefixCanary, LongChordCanary]);

pub fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("f10", Canary, Some("S9")),
        KeyBinding::new("alt", Canary, Some("S9")),
        KeyBinding::new("ctrl-k", ShortPrefixCanary, Some("S9")),
        KeyBinding::new("ctrl-k ctrl-c", LongChordCanary, Some("S9")),
    ]
}

// Public Window observation only; no focus or state reconciliation.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn has_live_ctrl_k_timeout(window: &gpui::Window, cx: &gpui::App) -> bool {
    window.pending_input().is_some_and(|pending| {
        let is_ctrl_k = matches!(pending.keystrokes(), [key]
        if key.key == "k" && key.modifiers == gpui::Modifiers {
            control: true,
            ..Default::default()
        });
        is_ctrl_k
            && pending
                .timeout()
                .is_some_and(|t| !t.is_paused() && !t.remaining(cx).is_zero())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{KeyContext, Keymap, Keystroke};

    #[test]
    fn shorter_canary_is_bound_while_longer_chord_is_pending() {
        let map = Keymap::new(bindings());
        let (matched, pending) = map.bindings_for_input(
            &[Keystroke::parse("ctrl-k").unwrap()],
            &[KeyContext::parse("S9").unwrap()],
        );
        assert!(pending, "the longer binding must leave Ctrl+K ambiguous");
        assert_eq!(
            matched.len(),
            1,
            "a prefix alone does not establish a timeout"
        );
        assert!(matched[0].action().as_any().is::<ShortPrefixCanary>());
    }

    #[test]
    fn longer_chord_has_its_own_distinct_canary() {
        let map = Keymap::new(bindings());
        let (matched, pending) = map.bindings_for_input(
            &[
                Keystroke::parse("ctrl-k").unwrap(),
                Keystroke::parse("ctrl-c").unwrap(),
            ],
            &[KeyContext::parse("S9").unwrap()],
        );
        assert!(!pending);
        assert_eq!(matched.len(), 1);
        assert!(matched[0].action().as_any().is::<LongChordCanary>());
    }

    #[test]
    fn spike_does_not_own_a_message_retrieval_loop() {
        // Architectural removal guard, not a Win32 message-delivery simulation.
        let native = include_str!("native.rs");
        for api in [
            "PeekMessageW(",
            "PeekMessageA(",
            "GetMessageW(",
            "GetMessageA(",
        ] {
            assert!(
                !native.contains(api),
                "S9 must leave queue retrieval to GPUI: {api}"
            );
        }
        let probe = include_str!("probe.rs");
        assert!(!probe.contains("native::retrieval_probe"));
        assert!(!probe.contains("\"retrieval\""));
    }
}
