//! Own-thread FFI only. This file is compiled solely into the S9 example.
//! No references/pointers into State are registered with Windows. TLS has no Drop
//! and stays allocated until thread exit; executable callbacks are never unloaded.
//! Every borrow ends before CallNextHookEx/DefSubclassProc/registration/unhook.
use crate::policy::*;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
    UI::{
        Input::KeyboardAndMouse::{
            GetFocus, GetKeyboardLayout, GetKeyboardState, MAPVK_VK_TO_VSC_EX, MapVirtualKeyExW,
        },
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
const SUBCLASS: usize = 0x5339;

#[derive(Clone, Copy, Default)]
pub struct Counters {
    pub consumed: u64,
    pub characters: u64,
    pub downs: u64,
    pub ups: u64,
    pub system_commands: u64,
    pub ime_messages: u64,
    pub bypass_keys: u64,
    pub max_depth: usize,
    pub focus_losses: u64,
    pub destroyed: bool,
}
struct State {
    policy: Policy,
    lease: Lease,
    observer: usize,
    observer_debt: bool,
    origin: Option<Instant>,
    counts: Counters,
    fail_snapshot: bool,
}
impl State {
    const fn new() -> Self {
        Self {
            policy: Policy::new(0),
            lease: Lease::new(),
            observer: 0,
            observer_debt: false,
            origin: None,
            counts: Counters {
                consumed: 0,
                characters: 0,
                downs: 0,
                ups: 0,
                system_commands: 0,
                ime_messages: 0,
                bypass_keys: 0,
                max_depth: 0,
                focus_losses: 0,
                destroyed: false,
            },
            fail_snapshot: false,
        }
    }
    fn now(&self) -> u64 {
        self.origin
            .map_or(0, |i| i.elapsed().as_millis().min(u64::MAX as u128) as u64)
    }
}
thread_local! { static STATE:RefCell<State> = const { RefCell::new(State::new()) }; }

/// Fail-stop on an internal invariant/panic, never unwind into Windows or fail
/// open into an action. No ordinary hook failure takes this path. No error logging
/// (especially no raw input) is done in the callback.
fn ffi<T>(f: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => std::process::abort(),
    }
}
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| {
        let Ok(mut s) = s.try_borrow_mut() else {
            std::process::abort()
        };
        f(&mut s)
    })
}
fn hwnd(value: usize) -> HWND {
    HWND(value as *mut std::ffi::c_void)
}
fn verify_window(window: HWND) -> Result<u32, &'static str> {
    let tid = unsafe { GetCurrentThreadId() };
    let pid = unsafe { GetCurrentProcessId() };
    let mut owner = 0;
    let window_tid = unsafe { GetWindowThreadProcessId(window, Some(&mut owner)) };
    if window.0.is_null() || tid == 0 || window_tid != tid || owner != pid {
        return Err("Refused: HWND is not on the current nonzero process/UI thread");
    }
    Ok(tid)
}

pub fn install(window: HWND) -> Result<(), String> {
    let tid = verify_window(window)?;
    if with(|s| s.lease.handle.is_some() || s.observer != 0) {
        return Err("Existing installation/cleanup debt must be resolved first".into());
    }
    with(|s| {
        s.policy = Policy::new(0);
        s.origin = Some(Instant::now());
    });
    // Narrow destruction/focus observer. It never consumes keydown or pretends
    // that a WNDPROC can undo TranslateMessage. No GPUI/App pointers as refdata.
    if !unsafe { SetWindowSubclass(window, Some(observer), SUBCLASS, 0) }.as_bool() {
        return Err("SetWindowSubclass failed; no recording enabled".into());
    }
    with(|s| s.observer = window.0 as usize);
    // Revalidate directly before installation. Nonzero tid: never a global hook.
    if let Err(error) = verify_window(window) {
        let clean = shutdown(false);
        return Err(format!("{error}; revalidation cleanup clean={clean}"));
    }
    match unsafe { SetWindowsHookExW(WH_GETMESSAGE, Some(get_message), None, tid) } {
        Ok(hook) => {
            with(|s| {
                s.lease.installed(hook.0 as usize);
                s.policy.target = window.0 as usize;
            });
            Ok(())
        }
        Err(e) => {
            let removed =
                unsafe { RemoveWindowSubclass(window, Some(observer), SUBCLASS) }.as_bool();
            with(|s| {
                if removed {
                    s.observer = 0;
                }
                s.observer_debt = !removed;
            });
            Err(format!(
                "SetWindowsHookExW failed: {e}; observer debt={}",
                !removed
            ))
        }
    }
}
fn classify(message: u32) -> Kind {
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN => Kind::Down,
        WM_KEYUP | WM_SYSKEYUP => Kind::Up,
        WM_CHAR | WM_SYSCHAR | WM_DEADCHAR | WM_SYSDEADCHAR => Kind::Character,
        _ => Kind::Other,
    }
}
// SAFETY: called only with the OS-owned valid MSG for HC_ACTION/PM_REMOVE.
// Never return it, store it, or hold a Rust reference across a chained callback.
unsafe fn neutralize(ptr: *mut MSG) {
    unsafe {
        (*ptr).message = WM_NULL;
        (*ptr).wParam = WPARAM(0);
        (*ptr).lParam = LPARAM(0);
    }
}
unsafe extern "system" fn get_message(code: i32, remove: WPARAM, data: LPARAM) -> LRESULT {
    // Negative and non-removing cases: no dereference, depth, decode or counters.
    if code != HC_ACTION as i32 || remove.0 != PM_REMOVE.0 as usize {
        return unsafe { CallNextHookEx(None, code, remove, data) };
    }
    ffi(|| {
        if data.0 == 0 {
            return unsafe { CallNextHookEx(None, code, remove, data) };
        }
        let ptr = data.0 as *mut MSG;
        let copied = unsafe { *ptr };
        let kind = classify(copied.message);
        let consumed = with(|s| {
            s.lease.enter();
            s.counts.max_depth = s.counts.max_depth.max(s.lease.depth);
            let decision = s.policy.decide(
                code,
                true,
                Input {
                    hwnd: copied.hwnd.0 as usize,
                    kind,
                    key: Physical::from_raw(copied.wParam.0 as u32, copied.lParam.0 as u32),
                    repeat: (copied.lParam.0 as u32) & (1 << 30) != 0,
                },
                s.now(),
            );
            if decision.consume {
                // Consume BEFORE snapshot/decode even on an OS failure or full FIFO.
                unsafe { neutralize(ptr) };
                s.counts.consumed = s.counts.consumed.saturating_add(1);
                match kind {
                    Kind::Character => s.counts.characters = s.counts.characters.saturating_add(1),
                    Kind::Down => s.counts.downs = s.counts.downs.saturating_add(1),
                    Kind::Up => s.counts.ups = s.counts.ups.saturating_add(1),
                    _ => {}
                }
            }
            if decision.capture {
                let mut raw = Raw {
                    message: copied.message,
                    vk: copied.wParam.0 as u32,
                    bits: copied.lParam.0 as u32,
                    time: copied.time,
                    ..Raw::default()
                };
                // These queries don't pump. This is the queue-associated state,
                // not GetAsyncKeyState, and not a promise of Chromium DOM parity.
                raw.status =
                    if s.fail_snapshot || unsafe { GetKeyboardState(&mut raw.keyboard) }.is_err() {
                        SnapshotStatus::Failed
                    } else {
                        SnapshotStatus::Ok
                    };
                raw.hkl = unsafe { GetKeyboardLayout(0) }.0 as usize;
                if raw.hkl == 0 && raw.status == SnapshotStatus::Ok {
                    raw.status = SnapshotStatus::MissingLayout;
                }
                s.policy.push(decision, raw);
            }
            decision.consume
        });
        // Nested retrieval can capture here; outer record is already enqueued.
        // No State/MSG Rust borrow spans this call. Preserve downstream return.
        let result = chain_reassert(
            consumed,
            || unsafe { neutralize(ptr) },
            || unsafe { CallNextHookEx(None, code, remove, data) },
        );
        with(|s| s.lease.leave());
        result
    })
}
unsafe extern "system" fn observer(
    window: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    ffi(|| {
        with(|s| {
            if s.observer == window.0 as usize {
                if msg == WM_NCDESTROY {
                    // Nonqueued: synchronous invalidation BEFORE forwarding, even
                    // if close bypassed the GPUI should-close callback entirely.
                    s.policy.destroy();
                    s.counts.destroyed = true;
                } else if msg == WM_KILLFOCUS {
                    s.policy.focus_lost();
                    s.counts.focus_losses = s.counts.focus_losses.saturating_add(1);
                } else if s.policy.armed {
                    match msg {
                        WM_SYSCOMMAND => {
                            s.counts.system_commands = s.counts.system_commands.saturating_add(1)
                        }
                        WM_IME_STARTCOMPOSITION | WM_IME_COMPOSITION | WM_IME_ENDCOMPOSITION => {
                            s.counts.ime_messages = s.counts.ime_messages.saturating_add(1)
                        }
                        WM_KEYDOWN | WM_SYSKEYDOWN => {
                            s.counts.bypass_keys = s.counts.bypass_keys.saturating_add(1)
                        }
                        _ => {}
                    }
                }
            }
        });
        if msg == WM_NCDESTROY {
            // All callback storage is TLS, not the window. Destruction permanently
            // invalidates observer identity even if removal reports false.
            let _ = unsafe { RemoveWindowSubclass(window, Some(observer), SUBCLASS) };
            with(|s| {
                s.observer = 0;
                s.observer_debt = false;
            });
        }
        unsafe { DefSubclassProc(window, msg, w, l) }
    })
}

pub fn drain() -> Batch {
    with(|s| s.policy.drain())
}
/// Only a foreground handler calls this; never the hook. No message pumping.
pub fn boundary(command: Command) -> Result<Batch, String> {
    let mut held = [Physical(0); 256];
    let mut len = 0;
    if matches!(command, Command::Begin | Command::Switch) {
        let target = with(|s| s.policy.target);
        verify_window(hwnd(target))?;
        if unsafe { GetFocus() } != hwnd(target) {
            return Err("Begin requires focus on the exact probe HWND".into());
        }
        let mut keys = [0; 256];
        unsafe { GetKeyboardState(&mut keys) }
            .map_err(|_| "SnapshotFailed: preheld state; Begin refused")?;
        let layout = unsafe { GetKeyboardLayout(0) };
        if layout.0.is_null() {
            return Err("SnapshotFailed: missing preheld HKL; Begin refused".into());
        }
        for (vk, state) in keys.iter().enumerate().skip(8) {
            if state & 128 == 0 || matches!(vk, 0x10..=0x12) {
                continue;
            }
            // Specific L/R modifier VKs avoid generic modifier duplicates.
            let scan = unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_VSC_EX, Some(layout)) };
            let bits = ((scan & 255) << 16) | if scan & 0xff00 == 0xe000 { 1 << 24 } else { 0 };
            held[len] = Physical::from_raw(vk as u32, bits);
            len += 1;
        }
    }
    Ok(with(|s| s.policy.boundary(command, s.now(), &held[..len])))
}
pub fn status() -> (bool, u64, Counters, bool, bool) {
    with(|s| {
        (
            s.policy.armed,
            s.policy.arm,
            s.counts,
            s.lease.debt,
            s.observer_debt,
        )
    })
}
pub fn fail_snapshot(enabled: bool) {
    with(|s| s.fail_snapshot = enabled);
}
/// Clear target FIRST. Retained HHOOK is real ownership debt, not "cleaned up".
/// inject_failure skips the OS unhook to leave a genuinely installed INACTIVE hook;
/// it does not pretend an OS call failed. Retry performs the real unhook.
pub fn shutdown(inject_failure: bool) -> bool {
    let (handle, observer) = with(|s| {
        s.policy.destroy();
        (
            if s.lease.needs_unhook() {
                s.lease.handle
            } else {
                None
            },
            s.observer,
        )
    });
    if let Some(handle) = handle {
        let success =
            !inject_failure && unsafe { UnhookWindowsHookEx(HHOOK(handle as *mut _)) }.is_ok();
        with(|s| s.lease.unhook_result(success));
    }
    if observer != 0 {
        let success =
            unsafe { RemoveWindowSubclass(hwnd(observer), Some(self::observer), SUBCLASS) }
                .as_bool();
        with(|s| {
            if success {
                s.observer = 0;
            }
            s.observer_debt = !success;
        });
    }
    with(|s| s.lease.handle.is_none() && s.observer == 0)
}
/// Explicit synthetic static characters; no drain and no other HWND targeted.
/// Call immediately before Begin to test a pre-existing queued character backlog.
pub fn prequeue_characters() -> Result<(), String> {
    let target = with(|s| s.policy.target);
    verify_window(hwnd(target))?;
    for message in [WM_CHAR, WM_SYSCHAR, WM_DEADCHAR, WM_SYSDEADCHAR] {
        unsafe { PostMessageW(Some(hwnd(target)), message, WPARAM(0x78), LPARAM(1)) }
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
/// Deliberate sent-message negative control. This is NOT a recorder capture.
pub fn sent_negative_control() -> Result<(), String> {
    let target = with(|s| s.policy.target);
    verify_window(hwnd(target))?;
    unsafe {
        SendMessageW(
            hwnd(target),
            WM_KEYDOWN,
            Some(WPARAM(0x41)),
            Some(LPARAM(0x001e0001)),
        )
    };
    Ok(())
}
