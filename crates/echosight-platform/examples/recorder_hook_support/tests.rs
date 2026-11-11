use super::*;
#[test]
fn successful_unhook_during_nested_callback_is_not_unhooked_twice() {
    let mut l = Lease::new();
    l.installed(42);
    l.enter();
    assert!(l.needs_unhook());
    l.unhook_result(true);
    assert!(!l.needs_unhook());
    assert_eq!(l.handle, Some(42));
    assert!(!l.debt);
    l.leave();
    assert_eq!(l.handle, None);
}
#[test]
fn raw_shape_exposes_not_normalizes_the_native_bits() {
    let raw = Raw {
        vk: 0x12,
        bits: 0xe1380003,
        ..Raw::default()
    };
    assert_eq!(
        raw.shape(),
        RawShape {
            scan: 0x38,
            extended: true,
            repeat_count: 3,
            previously_down: true,
            transition_up: true,
            alt_context: true,
            location: Location::Right
        }
    );
}
#[test]
fn raw_location_distinguishes_modifier_sides_and_numpad() {
    for (vk, bits, location) in [
        (0x10, 0x002a0001, Location::Left),
        (0x10, 0x00360001, Location::Right),
        (0x11, 0x001d0001, Location::Left),
        (0x11, 0x011d0001, Location::Right),
        (0x0d, 0x011c0001, Location::Numpad),
        (0x60, 0x00520001, Location::Numpad),
        (0x41, 0x001e0001, Location::Standard),
        (0xe7, 1, Location::Unknown),
    ] {
        assert_eq!(
            Raw {
                vk,
                bits,
                ..Raw::default()
            }
            .shape()
            .location,
            location
        );
    }
}
#[test]
fn chain_neutralizes_before_and_after_preserves_return_and_allows_nested_capture() {
    use std::cell::{Cell, RefCell};
    let p = RefCell::new(begun(&[]));
    let msg = Cell::new(0x100);
    capture(&mut p.borrow_mut(), Raw::default());
    let result = chain_reassert(
        true,
        || msg.set(0),
        || {
            assert_eq!(msg.get(), 0);
            capture(&mut p.borrow_mut(), Raw::default());
            msg.set(0x104);
            -73
        },
    );
    assert_eq!(result, -73);
    assert_eq!(msg.get(), 0);
    assert_eq!(p.borrow_mut().drain().records[1].unwrap().sequence, 2);
}
#[test]
fn untargeted_chain_keeps_downstream_mutations_and_result() {
    use std::cell::Cell;
    let msg = Cell::new(0x12);
    let result = chain_reassert(
        false,
        || panic!("untargeted mutation"),
        || {
            msg.set(0x312);
            17
        },
    );
    assert_eq!(msg.get(), 0x312);
    assert_eq!(result, 17);
}
const OWN: usize = 99;
const ALT: Physical = Physical(0x38);
const A: Physical = Physical(0x1e);
fn input(kind: Kind, key: Physical) -> Input {
    Input {
        hwnd: OWN,
        kind,
        key,
        repeat: false,
    }
}
fn begun(preheld: &[Physical]) -> Policy {
    let mut p = Policy::new(OWN);
    p.boundary(Command::Begin, 0, preheld);
    p
}
fn take(p: &mut Policy, kind: Kind, key: Physical, now: u64) -> Decision {
    p.decide(0, true, input(kind, key), now)
}
fn capture(p: &mut Policy, raw: Raw) {
    let d = take(p, Kind::Down, A, 1);
    p.push(d, raw);
}

#[test]
fn explicit_begin_is_required() {
    let mut p = Policy::new(OWN);
    assert!(!take(&mut p, Kind::Down, A, 0).consume);
    p.boundary(Command::Begin, 0, &[]);
    assert!(take(&mut p, Kind::Down, A, 1).capture);
}
#[test]
fn negative_and_nonaction_codes_and_noremove_have_no_side_effects() {
    let mut p = begun(&[]);
    for (code, remove) in [(-1, true), (1, true), (0, false), (0, false)] {
        let d = p.decide(code, remove, input(Kind::Down, A), 1);
        assert_eq!(d, Decision::default());
        p.push(d, Raw::default());
    }
    assert!(p.drain().records.iter().all(Option::is_none));
    p.boundary(Command::Stop, 2, &[]);
    assert!(!take(&mut p, Kind::Up, A, 3).consume);
    p.boundary(Command::Begin, 4, &[]);
    capture(&mut p, Raw::default());
    assert_eq!(p.drain().records[0].unwrap().sequence, 1);
}
#[test]
fn unrelated_hwnd_thread_quit_and_hotkey_are_untouched() {
    let mut p = begun(&[]);
    for hwnd in [0, 100] {
        for kind in [Kind::Down, Kind::Up, Kind::Character, Kind::Other] {
            assert_eq!(
                p.decide(
                    0,
                    true,
                    Input {
                        hwnd,
                        ..input(kind, A)
                    },
                    1
                ),
                Decision::default()
            );
        }
    }
    assert_eq!(take(&mut p, Kind::Other, A, 1), Decision::default());
    assert!(take(&mut p, Kind::Down, A, 2).consume);
}
#[test]
fn failed_snapshot_is_still_consumed_and_copied() {
    let mut p = begun(&[]);
    let d = take(&mut p, Kind::Down, A, 1);
    assert!(d.consume);
    p.push(
        d,
        Raw {
            status: SnapshotStatus::Failed,
            ..Raw::default()
        },
    );
    assert_eq!(
        p.drain().records[0].unwrap().raw.status,
        SnapshotStatus::Failed
    );
    assert!(p.armed);
}
#[test]
fn raw_snapshot_preserves_bits_modifiers_layout_and_time() {
    let mut p = begun(&[]);
    let mut raw = Raw {
        message: 0x104,
        vk: 0xa5,
        bits: 0xe1380003,
        time: 42,
        hkl: 123,
        status: SnapshotStatus::Ok,
        ..Raw::default()
    };
    raw.keyboard[0xa5] = 128;
    raw.keyboard[0xa2] = 128;
    raw.keyboard[0x14] = 1;
    capture(&mut p, raw);
    raw.keyboard.fill(0);
    let record = p.drain().records[0].unwrap();
    assert_eq!(record.raw.bits, 0xe1380003);
    assert_eq!(record.raw.vk, 0xa5);
    assert_eq!(record.raw.time, 42);
    assert_eq!(record.raw.hkl, 123);
    assert_eq!(record.raw.keyboard[0xa5], 128);
    assert_eq!(record.raw.keyboard[0xa2], 128);
    assert_eq!(record.raw.keyboard[0x14], 1);
}
#[test]
fn physical_scan_extended_and_zero_scan_vk_are_distinct() {
    assert_ne!(
        Physical::from_raw(0x12, 0x00380001),
        Physical::from_raw(0x12, 0x01380001)
    );
    assert_eq!(Physical::from_raw(0x12, 0x00380001), ALT);
    assert_ne!(Physical::from_raw(65, 0), Physical::from_raw(66, 0));
}
#[test]
fn nested_chain_capture_is_fifo_not_stack_order() {
    let mut p = begun(&[]);
    capture(
        &mut p,
        Raw {
            vk: 65,
            ..Raw::default()
        },
    );
    // Outer callback has committed and dropped its borrow before chaining; nested retrieval.
    capture(
        &mut p,
        Raw {
            vk: 66,
            ..Raw::default()
        },
    );
    let b = p.drain();
    assert_eq!(b.records[0].unwrap().raw.vk, 65);
    assert_eq!(b.records[1].unwrap().raw.vk, 66);
    assert_eq!(b.records[1].unwrap().sequence, 2);
}
#[test]
fn bounded_overflow_consumes_reports_loss_and_keeps_order() {
    let mut p = begun(&[]);
    for _ in 0..CAPACITY + 3 {
        let d = take(&mut p, Kind::Down, A, 1);
        assert!(d.consume);
        p.push(d, Raw::default());
    }
    let b = p.drain();
    assert_eq!(b.records.iter().flatten().count(), CAPACITY);
    assert_eq!(
        b.loss,
        Some(Loss {
            first_sequence: 65,
            arm: 1,
            count: 3
        })
    );
    capture(&mut p, Raw::default());
    assert_eq!(p.drain().records[0].unwrap().sequence, 68);
}
#[test]
fn all_foreground_boundaries_drain_before_state_change() {
    for command in [
        Command::Begin,
        Command::Switch,
        Command::Save,
        Command::Cancel,
        Command::Stop,
        Command::DetachedStop,
        Command::Global,
        Command::Close,
        Command::FocusLost,
    ] {
        let mut p = begun(&[]);
        capture(&mut p, Raw::default());
        let old = p.arm;
        let batch = p.boundary(command, 2, &[]);
        assert_eq!(batch.records[0].unwrap().arm, old);
        assert!(p.drain().records.iter().all(Option::is_none));
    }
}
#[test]
fn switch_does_not_apply_old_records_to_new_arm() {
    let mut p = begun(&[]);
    capture(&mut p, Raw::default());
    let b = p.boundary(Command::Switch, 2, &[]);
    capture(&mut p, Raw::default());
    assert_eq!(b.records[0].unwrap().arm, 1);
    assert_eq!(p.drain().records[0].unwrap().arm, 2);
}
#[test]
fn old_detached_stop_stops_current_arm_without_generation_filter() {
    let mut p = begun(&[]);
    p.boundary(Command::Switch, 100, &[]);
    assert!(p.armed);
    p.boundary(Command::DetachedStop, 200, &[]);
    assert!(!p.armed);
    assert!(!take(&mut p, Kind::Down, A, 201).capture);
}
#[test]
fn preheld_alt_release_survives_old_stop_with_no_consumed_down() {
    let mut p = begun(&[ALT]);
    p.boundary(Command::DetachedStop, 200, &[]);
    let d = take(&mut p, Kind::Up, ALT, 9000);
    assert!(d.consume);
    assert!(!d.capture);
    assert!(!take(&mut p, Kind::Up, ALT, 9001).consume);
}
#[test]
fn matching_consumed_release_only_no_blanket_keyup_swallow() {
    let mut p = begun(&[]);
    assert!(!take(&mut p, Kind::Up, ALT, 1).consume);
    assert!(take(&mut p, Kind::Down, A, 2).consume);
    p.boundary(Command::Stop, 3, &[]);
    assert!(!take(&mut p, Kind::Up, ALT, 4).consume);
    assert!(take(&mut p, Kind::Up, A, 5).consume);
}
#[test]
fn repeat_down_after_stop_protects_same_physical_cycle_without_capture() {
    let mut p = begun(&[ALT]);
    p.boundary(Command::Stop, 2, &[]);
    let d = p.decide(
        0,
        true,
        Input {
            repeat: true,
            ..input(Kind::Down, ALT)
        },
        3,
    );
    assert!(d.consume);
    assert!(!d.capture);
    assert!(take(&mut p, Kind::Up, ALT, 4).consume);
}
#[test]
fn unrelated_new_press_release_cycle_is_never_swallowed_after_stop() {
    let mut p = begun(&[ALT]);
    p.boundary(Command::Stop, 2, &[]);
    assert!(!take(&mut p, Kind::Down, ALT, 3).consume);
    assert!(!take(&mut p, Kind::Up, ALT, 4).consume);
}
#[test]
fn focus_boundary_clears_preheld_tail_and_later_cycles() {
    let mut p = begun(&[ALT]);
    p.focus_lost();
    assert!(!p.armed);
    assert!(!take(&mut p, Kind::Up, ALT, 1).consume);
    assert!(!take(&mut p, Kind::Character, A, 1).consume);
}
#[test]
fn destruction_clears_hwnd_before_reuse_but_retains_copied_records() {
    let mut p = begun(&[ALT]);
    capture(&mut p, Raw::default());
    p.destroy();
    assert_eq!(p.target, 0);
    assert!(!take(&mut p, Kind::Down, A, 1).consume);
    assert!(!take(&mut p, Kind::Up, ALT, 1).consume);
    assert!(p.drain().records[0].is_some());
    p.boundary(Command::Begin, 2, &[]);
    assert!(!p.armed);
}
#[test]
fn prequeued_characters_quarantined_while_armed_and_bounded_tail() {
    let mut p = begun(&[]);
    assert!(take(&mut p, Kind::Character, A, 1).consume);
    p.boundary(Command::Stop, 10, &[]);
    for _ in 0..TAIL_CHARS {
        assert!(take(&mut p, Kind::Character, A, 11).consume);
    }
    assert!(!take(&mut p, Kind::Character, A, 12).consume);
}
#[test]
fn char_tail_expires_without_other_messages_and_new_down_ends_it() {
    let mut p = begun(&[]);
    p.boundary(Command::Stop, 10, &[]);
    assert!(!take(&mut p, Kind::Character, A, 10 + TAIL_MS).consume);
    p.boundary(Command::Begin, 100, &[]);
    p.boundary(Command::Stop, 101, &[]);
    assert!(!take(&mut p, Kind::Down, A, 102).consume);
    assert!(!take(&mut p, Kind::Character, A, 102).consume);
}
#[test]
fn repeated_stops_do_not_extend_character_quarantine() {
    let mut p = begun(&[]);
    p.boundary(Command::Stop, 10, &[]);
    p.boundary(Command::DetachedStop, 49, &[]);
    assert!(!take(&mut p, Kind::Character, A, 60).consume);
}
#[test]
fn unhook_failure_retains_debt_and_success_reclaims_only_at_depth_zero() {
    let mut l = Lease::default();
    l.installed(42);
    l.enter();
    l.enter();
    l.unhook_result(false);
    assert_eq!(l.handle, Some(42));
    assert!(l.debt);
    l.leave();
    l.unhook_result(true);
    assert_eq!(l.handle, Some(42));
    l.leave();
    assert_eq!(l.handle, None);
    assert!(!l.debt);
}
#[test]
fn failed_unhook_can_be_retried_when_inactive() {
    let mut l = Lease::default();
    l.installed(42);
    l.unhook_result(false);
    assert!(l.debt);
    assert_eq!(l.handle, Some(42));
    l.unhook_result(true);
    assert_eq!(l.handle, None);
    assert!(!l.debt);
}
