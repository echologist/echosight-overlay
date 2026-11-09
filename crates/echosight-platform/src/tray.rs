//! System tray icon, ported from `src/main/tray/appTray.ts` (v2.1.1).
//!
//! Menu: `Show Echosight` / `Hide Echosight` / separator / `Exit Echosight`,
//! tooltip `Echosight`, left-click = show. Backed by the `tray-icon` crate.
//!
//! Event mapping (`tray-icon` 0.26):
//! - Menu items arrive as [`tray_icon::menu::MenuEvent`] with the item id;
//!   `show` / `hide` / `exit` call the matching callback.
//! - A left click arrives as `TrayIconEvent::Click { button: MouseButton::Left,
//!   button_state: MouseButtonState::Up, .. }` and maps to `on_show`, the same
//!   as Electron's `tray.on('click')`. `Down`, right/middle buttons,
//!   `DoubleClick`, `Enter`, `Move` and `Leave` are ignored.
//!   `with_menu_on_left_click(false)` keeps the context menu on right-click
//!   only, like Electron on Windows.
//!
//! Platform availability:
//! - Windows + macOS: `tray-icon` compiles without system libraries and is
//!   always linked; [`should_create`] still returns `false` on macOS unless the
//!   `tray-everywhere` feature is on (parity with v2.1.1: Windows only).
//! - Linux/BSD: the default `libappindicator` backend needs GTK 3 at build
//!   time, so `tray-icon` is only compiled with the `tray-everywhere` feature,
//!   which selects its pure-Rust `ksni` (StatusNotifierItem over D-Bus)
//!   backend. Without the feature [`AppTray::new`] returns
//!   [`TrayError::Unavailable`].
//!
//! Threading: on Windows `tray-icon` creates a hidden window on the calling
//! thread and dispatches its events from that thread's Win32 message loop, so
//! [`AppTray::new`] must run on the thread that pumps messages (the GPUI main
//! thread). Callbacks are invoked from inside that loop; keep them short and
//! hop to the app's executor if needed.

use std::sync::Arc;

/// Platform identifiers matching Node's `process.platform` values the
/// TypeScript code compares against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
    Other,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else {
            Platform::Other
        }
    }
}

/// `shouldCreateAppTray(platform)`: Windows only for parity; the
/// `tray-everywhere` cargo feature enables it on every platform.
pub fn should_create(platform: Platform) -> bool {
    if cfg!(feature = "tray-everywhere") {
        return true;
    }
    platform == Platform::Windows
}

pub const TOOLTIP: &str = "Echosight";
pub const MENU_SHOW: &str = "Show Echosight";
pub const MENU_HIDE: &str = "Hide Echosight";
pub const MENU_EXIT: &str = "Exit Echosight";

#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error("failed to decode tray icon: {0}")]
    Icon(#[from] image::ImageError),
    #[error("tray icon pixel buffer is invalid: {0}")]
    BadIcon(String),
    #[error("failed to build tray menu: {0}")]
    Menu(String),
    #[error("failed to create tray icon: {0}")]
    Create(String),
    #[error("tray is not available in this build ({0})")]
    Unavailable(&'static str),
}

/// RGBA8 pixels for the tray icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayIconRgba {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl TrayIconRgba {
    /// Decodes an `.ico` (the bundled `assets/tray-icon.ico`) or any other
    /// format enabled on the `image` crate, picking the largest frame.
    pub fn from_ico_bytes(bytes: &[u8]) -> Result<Self, TrayError> {
        let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Ico)?.to_rgba8();
        let (width, height) = img.dimensions();
        Ok(Self {
            rgba: img.into_raw(),
            width,
            height,
        })
    }
}

/// Callbacks mirroring `AppTrayOptions`.
pub struct TrayCallbacks {
    pub on_show: Arc<dyn Fn() + Send + Sync>,
    pub on_hide: Arc<dyn Fn() + Send + Sync>,
    pub on_exit: Arc<dyn Fn() + Send + Sync>,
}

impl TrayCallbacks {
    pub fn new<S, H, E>(on_show: S, on_hide: H, on_exit: E) -> Self
    where
        S: Fn() + Send + Sync + 'static,
        H: Fn() + Send + Sync + 'static,
        E: Fn() + Send + Sync + 'static,
    {
        Self {
            on_show: Arc::new(on_show),
            on_hide: Arc::new(on_hide),
            on_exit: Arc::new(on_exit),
        }
    }
}

/// Live tray icon; dropping it removes the icon and the global event handlers.
pub struct AppTray {
    inner: imp::Tray,
}

impl AppTray {
    /// Creates the tray icon with the v2.1.1 menu. Must be called on the UI
    /// (message-loop) thread, see the module docs.
    pub fn new(icon: TrayIconRgba, callbacks: TrayCallbacks) -> Result<Self, TrayError> {
        Ok(Self {
            inner: imp::create(icon, callbacks)?,
        })
    }

    /// Backend name, for logs.
    pub fn backend(&self) -> &'static str {
        self.inner.backend()
    }
}

#[cfg(any(windows, target_os = "macos", feature = "tray-everywhere"))]
mod imp {
    use super::{MENU_EXIT, MENU_HIDE, MENU_SHOW, TOOLTIP, TrayCallbacks, TrayError, TrayIconRgba};
    use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    pub struct Tray {
        _icon: TrayIcon,
    }

    impl Tray {
        pub fn backend(&self) -> &'static str {
            if cfg!(windows) {
                "tray-icon/win32"
            } else if cfg!(target_os = "macos") {
                "tray-icon/appkit"
            } else {
                "tray-icon/ksni"
            }
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
            TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
        }
    }

    pub fn create(icon: TrayIconRgba, callbacks: TrayCallbacks) -> Result<Tray, TrayError> {
        let icon = Icon::from_rgba(icon.rgba, icon.width, icon.height)
            .map_err(|e| TrayError::BadIcon(e.to_string()))?;

        let show_id = MenuId::new("show");
        let hide_id = MenuId::new("hide");
        let exit_id = MenuId::new("exit");
        let menu = Menu::new();
        menu.append_items(&[
            &MenuItem::with_id(show_id.clone(), MENU_SHOW, true, None),
            &MenuItem::with_id(hide_id.clone(), MENU_HIDE, true, None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(exit_id.clone(), MENU_EXIT, true, None),
        ])
        .map_err(|e| TrayError::Menu(e.to_string()))?;

        let TrayCallbacks {
            on_show,
            on_hide,
            on_exit,
        } = callbacks;
        let click_show = on_show.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id = event.id();
            if *id == show_id {
                on_show();
            } else if *id == hide_id {
                on_hide();
            } else if *id == exit_id {
                on_exit();
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                click_show();
            }
        }));

        let tray = TrayIconBuilder::new()
            .with_id("echosight")
            .with_tooltip(TOOLTIP)
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|e| TrayError::Create(e.to_string()))?;
        Ok(Tray { _icon: tray })
    }
}

#[cfg(not(any(windows, target_os = "macos", feature = "tray-everywhere")))]
mod imp {
    use super::{TrayCallbacks, TrayError, TrayIconRgba};

    pub struct Tray;

    impl Tray {
        pub fn backend(&self) -> &'static str {
            "none"
        }
    }

    pub fn create(_icon: TrayIconRgba, _callbacks: TrayCallbacks) -> Result<Tray, TrayError> {
        Err(TrayError::Unavailable(
            "Linux tray needs the `tray-everywhere` feature (ksni backend)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICO: &[u8] = include_bytes!("../../../assets/tray-icon.ico");

    #[test]
    fn bundled_tray_icon_decodes_to_rgba() {
        let icon = TrayIconRgba::from_ico_bytes(ICO).expect("decode tray-icon.ico");
        eprintln!("tray-icon.ico decoded: {}x{}", icon.width, icon.height);
        assert!(icon.width >= 16 && icon.height >= 16);
        assert_eq!(
            icon.rgba.len() as u64,
            u64::from(icon.width) * u64::from(icon.height) * 4
        );
        assert!(
            icon.rgba.iter().skip(3).step_by(4).any(|&a| a > 0),
            "icon is fully transparent"
        );
    }

    #[test]
    fn should_create_matches_v2_1_1_parity() {
        let everywhere = cfg!(feature = "tray-everywhere");
        assert!(should_create(Platform::Windows));
        assert_eq!(should_create(Platform::MacOs), everywhere);
        assert_eq!(should_create(Platform::Linux), everywhere);
        assert_eq!(should_create(Platform::Other), everywhere);
    }

    #[test]
    fn menu_strings_match_app_tray_ts() {
        assert_eq!(TOOLTIP, "Echosight");
        assert_eq!(MENU_SHOW, "Show Echosight");
        assert_eq!(MENU_HIDE, "Hide Echosight");
        assert_eq!(MENU_EXIT, "Exit Echosight");
    }

    #[cfg(not(any(windows, target_os = "macos", feature = "tray-everywhere")))]
    #[test]
    fn linux_default_build_reports_unavailable() {
        let icon = TrayIconRgba::from_ico_bytes(ICO).expect("decode");
        let result = AppTray::new(icon, TrayCallbacks::new(|| {}, || {}, || {}));
        assert!(matches!(result, Err(TrayError::Unavailable(_))));
    }
}
