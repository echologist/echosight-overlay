//! Win32 implementation of the overlay window operations.
//!
//! This is the only module in `echosight-platform` that may contain `unsafe`. Every FFI
//! call is wrapped so that callers get a [`PlatformError`] instead of a panic.
//!
//! Notes on the calls (spike S2/S3; see `docs/plans/gpui-port/spike-s2.md`, `spike-s3.md`):
//! * Click-through toggles `WS_EX_TRANSPARENT` (optionally with `WS_EX_LAYERED`) on the
//!   extended style. GPUI creates the window with `WS_EX_NOREDIRECTIONBITMAP` and renders
//!   via DirectComposition, so the layered variant is the one to watch.
//! * `SetWindowLongPtrW` returns the previous value, which may legitimately be 0, so the
//!   documented protocol is `SetLastError(0)` first and `GetLastError()` afterwards.
//! * `ShowWindow`'s return value is the previous visibility, not a success flag.

use std::io;

use raw_window_handle::RawWindowHandle;
use windows::Win32::Foundation::{GetLastError, HWND, SetLastError, WIN32_ERROR};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, HWND_TOPMOST, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

use crate::PlatformError;
use crate::overlay::ClickThroughMode;

/// The `HWND` stored as an integer so the handle is `Send + Sync`.
#[derive(Debug, Clone, Copy)]
pub struct NativeHandle(isize);

impl NativeHandle {
    pub(crate) fn from_raw(raw: RawWindowHandle) -> Result<Self, PlatformError> {
        match raw {
            RawWindowHandle::Win32(h) => Ok(Self(h.hwnd.get())),
            other => Err(PlatformError::UnexpectedHandleKind(kind_name(&other))),
        }
    }

    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut core::ffi::c_void)
    }
}

fn kind_name(raw: &RawWindowHandle) -> &'static str {
    match raw {
        RawWindowHandle::WinRt(_) => "WinRt",
        RawWindowHandle::Xlib(_) => "Xlib",
        RawWindowHandle::Xcb(_) => "Xcb",
        RawWindowHandle::Wayland(_) => "Wayland",
        RawWindowHandle::AppKit(_) => "AppKit",
        _ => "other",
    }
}

fn os_error(op: &'static str, source: windows::core::Error) -> PlatformError {
    PlatformError::Os {
        op,
        source: io::Error::from(source),
    }
}

fn last_error(op: &'static str) -> PlatformError {
    // SAFETY: `GetLastError` has no preconditions; it reads thread-local state.
    let code = unsafe { GetLastError() };
    PlatformError::Os {
        op,
        source: io::Error::from_raw_os_error(code.0 as i32),
    }
}

fn get_ex_style(hwnd: HWND) -> Result<WINDOW_EX_STYLE, PlatformError> {
    // SAFETY: `SetLastError`/`GetLastError` touch thread-local state only.
    // `GetWindowLongPtrW` takes any HWND and returns 0 for an invalid one; the handle comes
    // from GPUI's live window, and an invalid handle only yields an error, not UB.
    let value = unsafe {
        SetLastError(WIN32_ERROR(0));
        GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
    };
    // SAFETY: see above.
    if value == 0 && unsafe { GetLastError() }.0 != 0 {
        return Err(last_error("GetWindowLongPtrW(GWL_EXSTYLE)"));
    }
    Ok(WINDOW_EX_STYLE(value as u32))
}

fn set_ex_style(hwnd: HWND, style: WINDOW_EX_STYLE) -> Result<(), PlatformError> {
    // SAFETY: same reasoning as `get_ex_style`; `SetWindowLongPtrW` only changes window
    // bookkeeping owned by user32 and reports failure via the last-error value.
    let previous = unsafe {
        SetLastError(WIN32_ERROR(0));
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style.0 as isize)
    };
    // SAFETY: see above.
    if previous == 0 && unsafe { GetLastError() }.0 != 0 {
        return Err(last_error("SetWindowLongPtrW(GWL_EXSTYLE)"));
    }
    // Make the style change take effect without moving, resizing or activating.
    // SAFETY: plain Win32 call with a live HWND and no out-pointers.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    }
    .map_err(|e| os_error("SetWindowPos(SWP_FRAMECHANGED)", e))
}

pub(crate) fn set_click_through(
    h: NativeHandle,
    enabled: bool,
    mode: ClickThroughMode,
) -> Result<(), PlatformError> {
    let hwnd = h.hwnd();
    let bits = match mode {
        ClickThroughMode::TransparentLayered => WS_EX_TRANSPARENT | WS_EX_LAYERED,
        ClickThroughMode::TransparentOnly => WS_EX_TRANSPARENT,
    };
    let current = get_ex_style(hwnd)?;
    let next = if enabled {
        current | bits
    } else {
        // Always clear both so switching modes at runtime cannot leave WS_EX_LAYERED behind.
        WINDOW_EX_STYLE(current.0 & !(WS_EX_TRANSPARENT | WS_EX_LAYERED).0)
    };
    if next == current {
        return Ok(());
    }
    log::debug!(
        "overlay: ex-style {:#x} -> {:#x} (click_through={enabled}, {mode:?})",
        current.0,
        next.0
    );
    set_ex_style(hwnd, next)
}

pub(crate) fn hide(h: NativeHandle) -> Result<(), PlatformError> {
    // SAFETY: `ShowWindow` has no preconditions beyond a window handle; its BOOL result is
    // the previous visibility, so it is deliberately ignored.
    let _was_visible = unsafe { ShowWindow(h.hwnd(), SW_HIDE) };
    Ok(())
}

pub(crate) fn show(h: NativeHandle) -> Result<(), PlatformError> {
    // SAFETY: as in `hide`. `SW_SHOWNOACTIVATE` shows without taking focus from the game.
    let _was_visible = unsafe { ShowWindow(h.hwnd(), SW_SHOWNOACTIVATE) };
    assert_topmost(h)
}

pub(crate) fn set_position(h: NativeHandle, x: i32, y: i32) -> Result<(), PlatformError> {
    // SAFETY: plain Win32 call with a live HWND and no out-pointers.
    unsafe {
        SetWindowPos(
            h.hwnd(),
            Some(HWND_TOPMOST),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .map_err(|e| os_error("SetWindowPos(HWND_TOPMOST, x, y)", e))
}

pub(crate) fn assert_topmost(h: NativeHandle) -> Result<(), PlatformError> {
    // SAFETY: plain Win32 call with a live HWND and no out-pointers.
    unsafe {
        SetWindowPos(
            h.hwnd(),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .map_err(|e| os_error("SetWindowPos(HWND_TOPMOST)", e))
}

pub(crate) fn focus(h: NativeHandle) -> Result<(), PlatformError> {
    // SAFETY: plain Win32 call with a live HWND. Windows may refuse to change the
    // foreground window (foreground lock); that is reported as an error, not a crash.
    let ok = unsafe { SetForegroundWindow(h.hwnd()) };
    if ok.as_bool() {
        Ok(())
    } else {
        Err(PlatformError::Os {
            op: "SetForegroundWindow",
            source: io::Error::other("the system refused to bring the window to the foreground"),
        })
    }
}
