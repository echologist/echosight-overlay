//! Per-window operations GPUI does not expose through its public API.
//!
//! GPUI (zed main, see ADR-001) has no `set_position`, no per-window `hide`/`show`, and no
//! mouse passthrough on Windows/macOS/X11. `Window` implements
//! `raw_window_handle::HasWindowHandle`, so we take the native handle once and call the
//! platform directly. The platform-specific code lives in [`windows`] (the only module in
//! this crate allowed to contain `unsafe`); every other OS gets the [`unsupported`] stub
//! until spike S7 adds X11/Wayland.

use crate::PlatformError;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
use self::windows as imp;

#[cfg(not(windows))]
pub mod unsupported;
#[cfg(not(windows))]
use self::unsupported as imp;

/// Which extended styles are used for click-through on Windows.
///
/// GPUI creates its windows with `WS_EX_NOREDIRECTIONBITMAP` and renders through a
/// DirectComposition swap chain. Whether `WS_EX_LAYERED` interacts badly with that path is
/// unverified (research report, feature 4), so spike S2 tests both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClickThroughMode {
    /// `WS_EX_TRANSPARENT | WS_EX_LAYERED` (what Electron's `setIgnoreMouseEvents` uses).
    #[default]
    TransparentLayered,
    /// `WS_EX_TRANSPARENT` only.
    TransparentOnly,
}

impl ClickThroughMode {
    /// Reads `ECHOSIGHT_SPIKE_LAYERED` (`0` = transparent only, anything else = layered).
    pub fn from_env() -> Self {
        match std::env::var("ECHOSIGHT_SPIKE_LAYERED").as_deref() {
            Ok("0") => Self::TransparentOnly,
            _ => Self::TransparentLayered,
        }
    }
}

/// A `Send + Sync` snapshot of a GPUI window's native handle.
///
/// On Windows this is the `HWND`; it stays valid for as long as the GPUI window is open.
/// Callers must not use a handle after the window was closed (the OS will report an
/// error rather than crash, but the HWND value could in theory be reused).
#[derive(Debug, Clone, Copy)]
pub struct OverlayWindowHandle {
    inner: imp::NativeHandle,
}

impl OverlayWindowHandle {
    /// Captures the native handle of `window`.
    pub fn from_window(window: &gpui::Window) -> Result<Self, PlatformError> {
        // Fully qualified: `Window` also has an inherent `window_handle()` that returns
        // GPUI's own `AnyWindowHandle`.
        let handle = raw_window_handle::HasWindowHandle::window_handle(window)?;
        let inner = imp::NativeHandle::from_raw(handle.as_raw())?;
        Ok(Self { inner })
    }

    /// Lets mouse input fall through to whatever is underneath (uses
    /// [`ClickThroughMode::default`]).
    pub fn set_click_through(&self, enabled: bool) -> Result<(), PlatformError> {
        self.set_click_through_with(enabled, ClickThroughMode::default())
    }

    /// [`Self::set_click_through`] with an explicit [`ClickThroughMode`].
    pub fn set_click_through_with(
        &self,
        enabled: bool,
        mode: ClickThroughMode,
    ) -> Result<(), PlatformError> {
        imp::set_click_through(self.inner, enabled, mode)
    }

    /// Hides the window without closing it (state and GPU resources are kept).
    pub fn hide(&self) -> Result<(), PlatformError> {
        imp::hide(self.inner)
    }

    /// Shows the window again **without** activating it and re-asserts topmost.
    pub fn show(&self) -> Result<(), PlatformError> {
        imp::show(self.inner)
    }

    /// Moves the window's top-left corner to `(x, y)` in physical screen pixels; size and
    /// activation state are untouched.
    pub fn set_position(&self, x: i32, y: i32) -> Result<(), PlatformError> {
        imp::set_position(self.inner, x, y)
    }

    /// Re-asserts the always-on-top z-order (needed after some games/explorer steal it).
    pub fn assert_topmost(&self) -> Result<(), PlatformError> {
        imp::assert_topmost(self.inner)
    }

    /// Gives the window keyboard focus (foreground).
    pub fn focus(&self) -> Result<(), PlatformError> {
        imp::focus(self.inner)
    }
}
