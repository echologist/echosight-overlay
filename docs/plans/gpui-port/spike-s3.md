# Spike S3: hide / show / move / topmost / focus without recreating the window

Status: **awaiting Windows run**. Go/no-go spike (master plan, Phase 0).
GPUI source: zed `4ed72e0135aaab74a43914491c915dd6cbd10c67` (ADR-001).

## Purpose

GPUI's `Window` has no per-window `hide`/`show` and no runtime `set_position`. Prove that
Win32 calls on the raw `HWND` do the job while GPUI keeps its state, keeps rendering after
re-show, and does not drop input or log errors. This covers the Electron behaviours
`hideOverlay`/`showInactive`, `resetPosition` and `focus`.

Code: `crates/echosight-platform/examples/s3_show_hide_move.rs`;
`OverlayWindowHandle::{hide, show, set_position, assert_topmost, focus}` in
`src/overlay/windows.rs`:

| op | Win32 |
|----|-------|
| hide | `ShowWindow(hwnd, SW_HIDE)` |
| show | `ShowWindow(hwnd, SW_SHOWNOACTIVATE)` then `SetWindowPos(HWND_TOPMOST, SWP_NOMOVE\|SWP_NOSIZE\|SWP_NOACTIVATE)` |
| set_position | `SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE\|SWP_NOACTIVATE)` (physical px) |
| assert_topmost | `SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE\|SWP_NOSIZE\|SWP_NOACTIVATE)` |
| focus | `SetForegroundWindow(hwnd)` |

Keys: **F10** hide, **F11** show, **F12** move to the top-right anchor, **F7** focus.
Buttons: `Hide 3 s, then show`, `Move to top-right`, `Re-assert topmost`, `Focus`.
`ECHOSIGHT_SPIKE_CYCLE=1`: 2 s after start the window hides/shows itself 5 times (2 s apart).

Fallback design (documented in the example header): close the GPUI window and reopen it
with fresh `WindowOptions`; entity state survives, the GPU surface does not.

## Build

```bash
gh workflow run rust.yml -f spike=s3_show_hide_move --ref gpui-port-spike-gpui
# local:
cargo build --release -p echosight-platform --example s3_show_hide_move
```

## Manual test script (Windows 10/11)

1. Start `s3_show_hide_move.exe`; confirm S1 basics. Note the `renders:` value.
2. Button `Hide 3 s, then show`: the window disappears immediately (no fade, no leftover), the taskbar shows nothing, and after 3 s it reappears at the same place. Log: `hide: ok`, `show (no activate) + topmost: ok`. `hides`/`shows` read 1/1 and the log lines from before the hide are still there (state intact).
3. Focus check on re-show: before step 2 click into Notepad; after the window reappears Notepad still has the caret (re-show must not activate the panel).
4. Keyboard path: click the panel, press F10 (hidden), wait 2 s, press F11. Because the hidden window may lose keyboard focus, F11 may not arrive; if so use the auto cycle (step 5) and record this.
5. Auto cycle: quit, start with `$env:ECHOSIGHT_SPIKE_CYCLE="1"`. The window hides and re-shows 5 times, 2 s apart. Expect 5 `hide: ok` + 5 `show ... ok`, `auto-cycle finished`, counters keep increasing between cycles, no `ERROR`/`WARN` from gpui in the terminal.
6. Stress: start with the cycle env 10 times in a row (50 cycles total) or leave one run and press the Hide button 50 times; no leak of GPU memory visible in Task Manager (> +50 MB), no stall.
7. Move: drag the window to the middle of the screen, press F12 (or the button). The window snaps to the top-right anchor (20 px inset, same spot as at start). Log shows `set_position(<physical px>) [logical x,y]: ok`.
8. Move with DPI scaling (if the display is not 100 %): after F12 the window must still be flush with the inset; record the scale (`scale:` in the counters line) and whether the position is off by the scale factor.
9. Topmost: open a maximised Explorer, click it so it is active, then click `Re-assert topmost` first (it is already on top; this is a no-op check) and, with a borderless game running, click the game and then press F11 in the panel: the panel stays above the game.
10. Focus: click the game, then press F7 **in the game** (nothing should happen: F7 is a panel key). Now click the panel's `Focus` button; the log shows `focus: ok` or `focus: FAILED (...)` (Windows may refuse `SetForegroundWindow` when another process is in the foreground; record which).
11. Input after re-show: after any re-show, hover and click the buttons; they work; drag by the header works; resize works.
12. Quit cleanly.

## Results (fill in)

| # | Step | Result | Notes |
|---|------|--------|-------|
| 2 | hide/show, state intact | awaiting Windows run | |
| 3 | re-show does not activate | awaiting Windows run | |
| 4 | F10/F11 keyboard path | awaiting Windows run | |
| 5 | auto cycle 5x clean | awaiting Windows run | |
| 6 | 50 cycles, no leak/stall | awaiting Windows run | |
| 7 | F12 snaps to anchor | awaiting Windows run | |
| 8 | DPI correct | awaiting Windows run | |
| 9 | topmost after game click | awaiting Windows run | |
| 10 | focus result | awaiting Windows run | |
| 11 | input after re-show | awaiting Windows run | |

OS build: ______  GPU/driver: ______  DPI scale: ______  Date: ______

## Fallback plan

- Hide/show breaks rendering (black panel, stalled counters) or GPUI logs swap-chain
  errors: use close + reopen (`window.remove_window()` then `cx.open_window` with the same
  options); measure the flash. Alternative: keep the window but move it off-screen
  (`set_position(-10000, -10000)`) for "hide", which keeps the surface alive.
- `SetWindowPos` ignored for a `PopUp` window: call it with `SWP_FRAMECHANGED`, or fall
  back to close + reopen with new `window_bounds`.
- DPI offset (step 8): convert with the display's own scale (`PlatformDisplay` has none
  exposed; use `GetDpiForWindow` via `windows` crate) instead of `window.scale_factor()`.
- `SetForegroundWindow` refused: `AllowSetForegroundWindow`/`AttachThreadInput` dance or
  simulate an `Alt` key press (the Electron app only needs focus after a user hotkey, where
  Windows grants it); record whether `focus` after a global hotkey works in S4 + S3 together.
- If all of hide/show/move needs upstream work: contribute `PlatformWindow::{set_bounds,
  hide, show}` to GPUI and pin a fork rev (master plan go/no-go rule).
