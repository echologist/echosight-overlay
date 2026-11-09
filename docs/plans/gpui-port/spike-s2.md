# Spike S2: runtime click-through while GPUI keeps rendering

Status: **awaiting Windows run**. Go/no-go spike (master plan, Phase 0).
GPUI source: zed `4ed72e0135aaab74a43914491c915dd6cbd10c67` (ADR-001).

## Purpose

GPUI has no mouse-passthrough API on Windows. Prove that toggling `WS_EX_TRANSPARENT`
(with or without `WS_EX_LAYERED`) on the raw `HWND` makes clicks reach the game underneath
while the GPUI window keeps rendering with alpha, and that toggling back restores input
without stealing focus.

Code: `crates/echosight-platform/examples/s2_click_through.rs`,
`OverlayWindowHandle::set_click_through[_with]` in `src/overlay/{mod,windows}.rs`
(`SetWindowLongPtrW(GWL_EXSTYLE)` + `SetWindowPos(SWP_FRAMECHANGED)`).

Why two modes: GPUI creates its windows with `WS_EX_NOREDIRECTIONBITMAP` and renders via a
DirectComposition swap chain. Whether adding `WS_EX_LAYERED` (what Electron uses) breaks
that path is unverified, so the spike runs either way:

| `ECHOSIGHT_SPIKE_LAYERED` | styles toggled |
|---|---|
| `1` (default) | `WS_EX_TRANSPARENT \| WS_EX_LAYERED` |
| `0` | `WS_EX_TRANSPARENT` only |

Because a click-through panel cannot be clicked, click-through switches itself **off after
8 s** (countdown in the status line); F9 also toggles it while the window has keyboard focus.

## Build

```bash
gh workflow run rust.yml -f spike=s2_click_through --ref gpui-port-spike-gpui
# local:
cargo build --release -p echosight-platform --example s2_click_through
```

## Manual test script (Windows 10/11)

Run the whole script twice: once as `s2_click_through.exe` (layered) and once as
`$env:ECHOSIGHT_SPIKE_LAYERED="0"; .\s2_click_through.exe` (transparent only).

1. Start the exe from PowerShell; confirm the S1 basics (top-right, transparent, counting). Status shows `mode: TransparentLayered` or `TransparentOnly`.
2. Open Notepad behind the panel so that part of its text area is under the panel body.
3. Click `Click-through: OFF` (or press F9). Log: `set_click_through(true, ...): ok`. The button label changes to ON, `auto-off in:` counts down.
4. While ON: the panel still renders (renders/ticks increase) with the same alpha as before. **Record any change**: black background, flicker, missing panel, frozen counters.
5. While ON: click inside the panel over Notepad's text area. The click must land in Notepad (caret moves / Notepad activates). Type a few letters; they appear in Notepad.
6. While ON: hover over the button; it must **not** highlight (no hover state).
7. Wait for auto-off. Log: `auto-off timer fired`, `set_click_through(false, ...): ok`. Notepad keeps focus (the panel must not steal it on toggle-off).
8. Click the panel's button again: it works; hover highlights again; log shows the toggle.
9. Keyboard-focused rapid toggling: click the panel once (focus), then press F9 twenty times, about once per second. Every press logs `ok`; the toggles counter reaches 20 + earlier toggles; no flicker, no black frames, counters never stall.
10. Game, borderless-fullscreen: start the game, place the panel over it, enable click-through (button). Click through the panel onto the game: the game reacts (e.g. character moves / menu opens) and the panel stays on top and rendering. Wait for auto-off; the game keeps focus.
11. Re-assert after game click: with click-through OFF, click the panel's button once; the panel takes focus; click the game again; panel stays on top.
12. Quit cleanly.

## Results (fill in)

| # | Step | Layered (`=1`) | Transparent only (`=0`) | Notes |
|---|------|----------------|--------------------------|-------|
| 3 | toggle on ok | awaiting Windows run | awaiting Windows run | |
| 4 | rendering/alpha intact while ON | awaiting Windows run | awaiting Windows run | |
| 5 | clicks reach Notepad | awaiting Windows run | awaiting Windows run | |
| 6 | no hover while ON | awaiting Windows run | awaiting Windows run | |
| 7 | auto-off, no focus steal | awaiting Windows run | awaiting Windows run | |
| 8 | input restored | awaiting Windows run | awaiting Windows run | |
| 9 | 20 toggles, no flicker | awaiting Windows run | awaiting Windows run | |
| 10 | clicks reach the game | awaiting Windows run | awaiting Windows run | |
| 11 | stays topmost after game click | awaiting Windows run | awaiting Windows run | |

OS build: ______  GPU/driver: ______  Date: ______

## Fallback plan

- Layered mode breaks rendering but transparent-only works: ship transparent-only
  (`ClickThroughMode::TransparentOnly` becomes the default).
- Neither passes clicks: (1) try `SetWindowRgn`/`SetLayeredWindowAttributes` combos, (2)
  implement passthrough with a `WM_NCHITTEST` subclass returning `HTTRANSPARENT`
  (`SetWindowSubclass` on the HWND; GPUI's own `WM_NCHITTEST` handler runs first, so this
  needs testing), (3) contribute `PlatformWindow::set_mouse_passthrough` upstream to GPUI
  (small, self-contained; pin a fork rev until merged), as the master plan's go/no-go rule
  says.
- Toggle-off steals focus: add `SWP_NOACTIVATE` is already set; next try
  `SetWindowPos(HWND_TOPMOST)` without `SWP_FRAMECHANGED` and, if needed, restore the
  previous foreground window with `SetForegroundWindow(previous)`.
