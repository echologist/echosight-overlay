# Spike S1: frameless, transparent, topmost, resizable overlay window

Status: **awaiting Windows run**. Go/no-go spike (master plan, Phase 0).
GPUI source: zed `4ed72e0135aaab74a43914491c915dd6cbd10c67` (ADR-001).

## Purpose

Prove that `WindowOptions` alone gives us the Electron overlay window on Windows:
frameless, per-pixel transparent, above a borderless-fullscreen game, no taskbar button,
user-resizable with a 450x600 floor, draggable by its header, opened top-right of the
primary display's work area inset 20 px (`echosight_core::window::placement::default_bounds`).

Code: `crates/echosight-platform/examples/s1_window.rs` (+ `examples/common/mod.rs`).
Options used: `titlebar: None`, `kind: WindowKind::PopUp`,
`window_background: WindowBackgroundAppearance::Transparent`, `is_resizable: true`,
`window_min_size: 450x600`, `window_decorations: Some(Client)`,
`window_bounds: Windowed(default_bounds(primary_display.visible_bounds()))`; header div
has `.window_control_area(WindowControlArea::Drag)`.

## Build

CI (produces `spike-s1_window-windows` artifact with `s1_window.exe`):

```bash
gh workflow run rust.yml -f spike=s1_window --ref gpui-port-spike-gpui
gh run watch   # then: gh run download -n spike-s1_window-windows
```

Local (any OS with the toolchain; on Windows needs MSVC Build Tools + Windows SDK):

```bash
cargo build --release -p echosight-platform --example s1_window
# -> target/release/examples/s1_window(.exe)
```

Run from a terminal to see the log (`RUST_LOG=debug` for more). Env knobs:
`ECHOSIGHT_SPIKE_FOCUS=0` opens the window without taking focus.

## Manual test script (Windows 10/11)

1. Run `s1_window.exe` from PowerShell. Expect log lines `work area Rect {...}; opening 500x700 at X,Y` and `window opened`, no `ERROR` lines.
2. Position: the panel's top-right corner is 20 px from the right edge and 20 px below the top of the primary monitor's work area (not under the taskbar). Compare with the log's `X,Y`.
3. Frame: no title bar, no OS border, no shadow frame around the rounded corners.
4. Transparency: the desktop wallpaper is visible through the panel body (dark purple tint) and fully visible outside the rounded corners. Move a window with text behind the panel; it must show through.
5. Counters: the `renders:` and `ticks:` numbers in the panel increase continuously (ticks about 10 per second). Leave it 60 s; both keep increasing.
6. Taskbar: no button for the spike appears on the taskbar; Alt+Tab does not list it (tool window).
7. Drag: press and drag on the header strip (title + Quit button row). The window moves. Dragging on the body does **not** move it.
8. Resize: drag each of the 4 edges and 4 corners. The window resizes; it refuses to go below 450x600 (watch `bounds:` in the counters line). **Research unknown**: `PopUp` has no `WS_THICKFRAME`; record whether edge resize works at all.
9. Click: press `Click me` 5 times; the label counts up and the log shows `button clicked`. Hover highlights the button.
10. Keys: with the panel focused press `a`, `F5`; the log shows `key: a`, `key: f5`.
11. Explorer: open an Explorer window and maximise it; the panel stays on top.
12. Game, borderless: start Path of Exile 2 (or any game) in **borderless/windowed fullscreen**. The panel stays visible on top while the game has focus. Click the game, type in it: the panel remains on top and keeps counting.
13. Game, exclusive fullscreen (information only): switch the game to exclusive fullscreen; note whether the panel disappears (expected, same as Electron).
14. Multi-monitor (if available): the window opens on the primary display; drag it to the second display; it keeps rendering and resizing there.
15. Quit: press `Quit`; the process exits with code 0 and no panic in the terminal.

## Results (fill in)

| # | Step | Result (pass/fail) | Notes / screenshot |
|---|------|--------------------|--------------------|
| 1 | starts, no errors | awaiting Windows run | |
| 2 | top-right inset 20 px | awaiting Windows run | |
| 3 | frameless | awaiting Windows run | |
| 4 | per-pixel alpha | awaiting Windows run | |
| 5 | keeps rendering | awaiting Windows run | |
| 6 | no taskbar / Alt+Tab | awaiting Windows run | |
| 7 | header drag only | awaiting Windows run | |
| 8 | edge resize + min size | awaiting Windows run | |
| 9 | click + hover | awaiting Windows run | |
| 10 | key events | awaiting Windows run | |
| 11 | above Explorer | awaiting Windows run | |
| 12 | above borderless game | awaiting Windows run | |
| 13 | exclusive fullscreen (info) | awaiting Windows run | |
| 14 | multi-monitor | awaiting Windows run | |
| 15 | clean quit | awaiting Windows run | |

OS build: ______  GPU/driver: ______  DPI scale: ______  Date: ______

## Fallback plan

- Resize (step 8) fails: open with `WindowKind::Normal` (gets `WS_THICKFRAME`) and apply
  topmost + tool-window ourselves through the raw handle (`SetWindowPos(HWND_TOPMOST)`,
  `WS_EX_TOOLWINDOW` via `SetWindowLongPtrW`); the helpers for that already exist in
  `src/overlay/windows.rs`. Second fallback: implement resize ourselves with a hit-test
  border (`window.start_window_resize` is Linux-only, so this would be a Win32 `WM_NCHITTEST`
  subclass or a manual `SetWindowPos` on drag).
- Transparency (step 4) fails: check `GPUI_DISABLE_DIRECT_COMPOSITION` is **not** set and
  the GPU driver supports DirectComposition; otherwise GPUI has no alpha path on Windows and
  the port needs an upstream fix (blocker for the whole plan).
- Topmost over a borderless game (step 12) fails: `assert_topmost()` on a 1 s timer (what
  the Electron app effectively relies on via `setAlwaysOnTop('screen-saver')`).
- Drag (step 7) fails: handle `on_mouse_down` + `SetWindowPos` ourselves (drag loop).
