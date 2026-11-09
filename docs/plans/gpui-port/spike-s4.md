# Spike S4: global hotkeys (`global-hotkey`) inside GPUI's run loop

Status: **awaiting Windows run**. Go/no-go spike (master plan, Phase 0).
GPUI source: zed `4ed72e0135aaab74a43914491c915dd6cbd10c67` (ADR-001). `global-hotkey` 0.10.0.

## Purpose

GPUI has no global hotkey API. Prove that `global-hotkey`'s Windows backend (hidden HWND +
`RegisterHotKey`/`WM_HOTKEY`) works when the manager is created inside `Application::run`
(main thread, where GPUI's `GetMessageW`/`DispatchMessageW` loop runs), that hotkeys fire
while the game has focus, that unregister/re-register at runtime works (hotkey recording
suspends hotkeys in the Electron app), and that a combo owned by another app fails softly.

Code: `crates/echosight-platform/examples/s4_hotkeys.rs`. The manager is created in the
`make` closure that `common::run` calls inside `Application::run`; `GlobalHotKeyEvent::receiver()`
is drained with `try_recv` on the shell's 100 ms foreground timer (UI thread never blocks).
Registered: **Ctrl+Shift+T / I / N / Z / Y**; each result is logged `<combo>: SUCCESS|FAILED`.
**F8** (or the button) unregisters all and re-registers; `Suspend`/`Resume` toggles them.

## Build

```bash
gh workflow run rust.yml -f spike=s4_hotkeys --ref gpui-port-spike-gpui
# local:
cargo build --release -p echosight-platform --example s4_hotkeys
```

## Manual test script (Windows 10/11)

1. Start `s4_hotkeys.exe` from PowerShell. Terminal and on-screen log show five lines `Ctrl+Shift+X: SUCCESS`; the status table shows `ok` for all five.
2. Panel focused: press Ctrl+Shift+T. Log: `hotkey #1: Ctrl+Shift+T`; `fired: 1`, `last: Ctrl+Shift+T`. Repeat for I, N, Z, Y (fired reaches 5).
3. Other app focused: click Notepad, press each combo once. All five fire (fired = 10) and Notepad does **not** receive them (no `T` typed, no "new window").
4. Hold: hold Ctrl+Shift+T for 2 s. Record how many events fire (auto-repeat is expected on Windows; the Electron app ignores repeats).
5. Latency: press Ctrl+Shift+N 10 times quickly; all 10 are counted, each appears in the log within ~100 ms (one tick).
6. Game, borderless-fullscreen: start the game, click into it, press all five combos. All five fire (watch the panel or the terminal). Record any that the game swallows.
7. Game, exclusive fullscreen (information only): repeat; record.
8. Re-register: press F8 (panel focused) or click `Unregister all + re-register`. Log shows 5 `unregistered` then 5 `SUCCESS`; `re-registers: 1`. Repeat step 3: all still fire.
9. Suspend/resume: click `Suspend`; status shows `--` for all; press Ctrl+Shift+T in Notepad: nothing fires and Notepad gets the key (whatever Notepad does with it). Click `Resume`; all `ok`; hotkeys fire again.
10. Conflict: start a second copy of `s4_hotkeys.exe`. In the second copy all five log `FAILED (...)` (already registered by the first process) and the second copy keeps running (no crash). Quit the second copy; press F8 in the first: all `SUCCESS` again.
11. Conflict with a real app: with the game or another tool owning e.g. Ctrl+Shift+I, start the spike; that combo logs `FAILED`, the other four `SUCCESS` and work.
12. CPU: with the panel idle, Task Manager shows < 1 % CPU for the process (the 100 ms poll must be negligible).
13. Quit cleanly; after quitting, Ctrl+Shift+T in Notepad is a plain key again (hotkeys released on exit).

## Results (fill in)

| # | Step | Result | Notes |
|---|------|--------|-------|
| 1 | 5x SUCCESS at start | awaiting Windows run | |
| 2 | fire with panel focused | awaiting Windows run | |
| 3 | fire with Notepad focused, not leaked | awaiting Windows run | |
| 4 | auto-repeat count | awaiting Windows run | |
| 5 | 10 rapid presses counted | awaiting Windows run | |
| 6 | fire with borderless game focused | awaiting Windows run | |
| 7 | exclusive fullscreen (info) | awaiting Windows run | |
| 8 | F8 re-register works | awaiting Windows run | |
| 9 | suspend/resume | awaiting Windows run | |
| 10 | second instance fails softly | awaiting Windows run | |
| 11 | partial conflict | awaiting Windows run | |
| 12 | idle CPU < 1 % | awaiting Windows run | |
| 13 | released on exit | awaiting Windows run | |

OS build: ______  Game + display mode: ______  Date: ______

## Fallback plan

- Nothing fires at all: the manager's hidden window is on the main thread but GPUI may
  filter messages; create the manager on a dedicated thread with its own
  `GetMessageW` loop (the crate only requires *a* message loop on the creating thread) and
  keep the crossbeam receiver.
- Fires but with > 100 ms latency: drain the receiver from a background task
  (`cx.background_executor().spawn`) that blocks on `recv()` and forwards through
  `AsyncApp::update`, instead of the 100 ms poll.
- Game swallows combos (step 6): same limitation as Electron (it also uses `RegisterHotKey`);
  document; the raw-input/low-level-hook route (`SetWindowsHookExW(WH_KEYBOARD_LL)`) is
  an opt-in future item, not a Phase 0 blocker.
- Hard conflicts crash or hang: wrap registration in a worker and treat `FAILED` as a
  settings warning (the Electron app already reports failures per combo).
