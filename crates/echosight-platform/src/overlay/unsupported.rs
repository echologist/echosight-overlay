//! Stub used on every platform other than Windows. X11/Wayland come in spike S7.

use raw_window_handle::RawWindowHandle;

use crate::PlatformError;
use crate::overlay::ClickThroughMode;

/// Opaque placeholder; the real handle kinds are added per platform in S7.
#[derive(Debug, Clone, Copy)]
pub struct NativeHandle;

impl NativeHandle {
    pub(crate) fn from_raw(raw: RawWindowHandle) -> Result<Self, PlatformError> {
        log::info!("overlay: raw window handle is {raw:?}; native ops are stubs here");
        Ok(Self)
    }
}

fn unsupported(op: &'static str) -> Result<(), PlatformError> {
    log::warn!("overlay: {op} is unsupported on this platform");
    Err(PlatformError::Unsupported { op })
}

pub(crate) fn set_click_through(
    _h: NativeHandle,
    enabled: bool,
    mode: ClickThroughMode,
) -> Result<(), PlatformError> {
    log::warn!("overlay: set_click_through({enabled}, {mode:?}) requested");
    unsupported("set_click_through")
}

pub(crate) fn hide(_h: NativeHandle) -> Result<(), PlatformError> {
    unsupported("hide")
}

pub(crate) fn show(_h: NativeHandle) -> Result<(), PlatformError> {
    unsupported("show")
}

pub(crate) fn set_position(_h: NativeHandle, x: i32, y: i32) -> Result<(), PlatformError> {
    log::warn!("overlay: set_position({x}, {y}) requested");
    unsupported("set_position")
}

pub(crate) fn assert_topmost(_h: NativeHandle) -> Result<(), PlatformError> {
    unsupported("assert_topmost")
}

pub(crate) fn focus(_h: NativeHandle) -> Result<(), PlatformError> {
    unsupported("focus")
}
