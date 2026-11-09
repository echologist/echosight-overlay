# Spike S6: supporting crates next to GPUI (audio, game monitor, single instance, tray)

Branch: merged into `gpui-port` (developed on `gpui-port-spike-support`). Crate: `echosight-platform`.

Status: **passed on Windows (2026-11-09, Milan, second build)**. Single instance acquired
(`Local\Echosight.SingleInstance` + `\\.\pipe\echosight-reveal`) and the process scan ran
(`matching processes: none`). Audio and tray did **not** run: the first build resolved
`complete.wav` and `tray-icon.ico` through the build machine's `CARGO_MANIFEST_DIR`
(`D:\a\echosight-overlay\...`, os error 3 on the test PC). Fixed by embedding both assets with
`include_bytes!`. Re-run of the rebuilt exe (2026-11-09, Milan): **audio and tray work**, so S6 passes.
Observation: playback at `volume 0.5` was perceived as very loud. Perceived loudness of rodio's
linear gain versus Chromium's `HTMLAudioElement.volume` at the same number is unverified; Phase 4
must A/B the GPUI build against Electron 2.1.1 at the default 60 % and pick a calibration factor
(or a perceptual curve) if they differ. Lesson: spike binaries never read repo paths at runtime.

## Purpose

Prove, before Phase 4, that the four non-window platform features of v2.1.1 can
be implemented with maintained Rust crates that coexist with GPUI's main-thread
message loop, and pin the exact crates, feature names and threading model:

| Feature | v2.1.1 source | Rust module | Crate |
|---|---|---|---|
| Theme sounds (4 events, volume rule, wav/ogg/mp3/m4a) | `src/renderer/features/themes/themeSounds.ts` | `src/audio.rs` | `rodio` 0.23.0 (`playback`, `wav`, `vorbis`, `mp3`, `mp4`) |
| Game monitor (3 s poll, `pathofexile` match, Started/Stopped/StillRunning) | `src/main/game/poe2Monitor.ts` | `src/game_monitor.rs` | `sysinfo` 0.40.1 (`system` only) |
| Single instance + reveal | `src/main/lifecycle/singleInstance.ts`, inventory section 6 | `src/single_instance.rs` | std (Unix socket) / `windows` 0.62.2 (named mutex + named pipe) |
| Windows tray (Show / Hide / Exit, tooltip, left-click = show) | `src/main/tray/appTray.ts` | `src/tray.rs` | `tray-icon` 0.26.1 (+ `image` 0.25.10 `ico` for the icon) |

Master plan references: section 2.2 bullets Sounds / Game monitor / Lifecycle /
Windows tray, Phase 4 module list, spike row S6.

## What was built

- `audio::SoundPlayer`: one worker thread owns the `rodio::MixerDeviceSink`
  (opened lazily on the first `play`, retried at most every 10 s after a
  failure), a per-`(theme id, event)` decoded cache (`play_event`,
  `clear_cache`), and `play(bytes, volume)` for one-off playback. Every
  failure is `log::warn!`, nothing panics or returns an error to the UI.
  `audio::effective_volume` and `audio::sounds_allowed` are the exact TS rules.
- `game_monitor::GameMonitor`: background thread, `recv_timeout` based stop,
  `EdgeDetector` with the exact poe2Monitor transitions, `ProcessSource` trait
  for fakes, `SysinfoSource` using
  `ProcessRefreshKind::nothing().without_tasks().with_exe(UpdateKind::OnlyIfNotSet)`
  and matching `is_path_of_exile_process` on the process name **and** the exe
  file name. `list_matching_processes()` for diagnostics.
- `single_instance::SingleInstance::acquire(app_id, on_reveal)`: primary gets a
  guard and the callback; secondary signals the primary and gets
  `AlreadyRunning`. Windows: `Local\Echosight.SingleInstance` mutex +
  `\\.\pipe\echosight-reveal` listener thread (`ConnectNamedPipe` loop, woken
  on drop). Unix: `$XDG_RUNTIME_DIR/echosight.sock` (fallback temp dir), stale
  sockets are unlinked. All `unsafe` is in the Windows backend with `// SAFETY:`.
- `tray::AppTray::new(icon_rgba, callbacks)`: `tray-icon` menu `Show Echosight`
  / `Hide Echosight` / separator / `Exit Echosight`, tooltip `Echosight`,
  `TrayIconEvent::Click { button: Left, button_state: Up }` maps to `on_show`,
  context menu on right click only. `tray::should_create(platform)` is Windows
  only; cargo feature `tray-everywhere` flips it and, on Linux, compiles
  `tray-icon`'s pure-Rust `ksni` backend (default build on Linux links no
  tray at all, so no GTK/libappindicator is needed).
- `examples/s6_support.rs`: console binary exercising all four for 15 s.

## How to run

### Linux (and macOS)

```sh
cargo test -p echosight-platform                      # 23 tests, headless
cargo clippy -p echosight-platform --all-targets -- -D warnings
cargo run -p echosight-platform --example s6_support  # terminal 1
cargo run -p echosight-platform --example s6_support  # terminal 2, while 1 runs: exits 3
S6_SKIP_AUDIO=1 cargo run -p echosight-platform --example s6_support   # clean CPU figure
cargo run -p echosight-platform --features tray-everywhere --example s6_support  # ksni tray
```

Linux build prerequisite: `cpal` needs `pkg-config` to find ALSA
(`libasound2-dev` on Ubuntu, already in `rust.yml`; `alsa-lib-devel` on Fedora).

### Windows

CI build of the example (no Windows box needed for the build):

```sh
gh workflow run rust.yml -f spike=s6_support --ref gpui-port-spike-support
```

Download the `spike-s6_support-windows` artifact, unzip, then run the manual
checks below from a `cmd`/PowerShell in the repo root (the example locates
`data/` and `assets/` relative to its compile-time manifest dir, so run it from
a checkout; or copy `data/themes/winamp-classic/complete.wav` and
`assets/tray-icon.ico` next to the same relative paths).

Local Windows build: `cargo run -p echosight-platform --example s6_support`.

## Manual checks

1. Start `s6_support`. Expect `single instance acquired (Local\Echosight.SingleInstance + \\.\pipe\echosight-reveal)` on Windows or `(.../echosight.sock)` on Linux.
2. Within ~1 s the Winamp `complete.wav` plays at half volume (Windows/macOS with a device). Without a device the line `[WARN] ... sound playback failed: failed to open the default audio output` appears instead; the process must keep running.
3. `matching processes: none` is printed every 3 s, five times.
4. With Path of Exile (1 or 2, any launcher) already running before start: the first scan prints its process name(s) and `game monitor: Started` appears once, then `game monitor: StillRunning` every 3 s.
5. Start the game during the run: `Started` once, then `StillRunning`. Close it: `Stopped` once, then nothing.
6. Start a second `s6_support` while the first runs: the second prints `another instance is already running; signalled it, exiting with 3` and exits with code 3 (`echo %ERRORLEVEL%` / `echo $?`); the first prints `reveal requested`.
7. Windows: a tray icon with the Echosight eye appears, tooltip `Echosight` on hover. Right click shows `Show Echosight`, `Hide Echosight`, a separator, `Exit Echosight`.
8. Windows: left click prints `tray: show`; menu items print `tray: show` / `tray: hide`; `Exit Echosight` prints `tray: exit` and ends the run early.
9. After 15 s the process prints `done after 15.0 s` and exits 0; on Linux the line includes the process CPU time.
10. Start a third copy after the first exited: it becomes the primary again (lock released on drop).

## Results

| # | Check | Linux (this box: Fedora 44, no sound server, headless, Rust 1.99.1) | Windows |
|---|---|---|---|
| 1 | single instance acquired | observed: `single instance acquired (/run/user/1001/echosight.sock)` | awaiting Windows run |
| 2 | sound plays / warns without device | observed: `[WARN] echosight_platform::audio: sound playback failed: failed to open the default audio output: Error opening the stream with the OS`; process kept running. Note: the ALSA open took 14 s and burned ~7 s CPU on this box (pipewire ALSA plugin with no server); the worker thread kept the main loop unaffected | awaiting Windows run |
| 3 | process scan every 3 s | observed: 5 x `matching processes: none` | awaiting Windows run |
| 4, 5 | Started / StillRunning / Stopped with a real game | not possible here (no game). Edge logic proven by `thread_emits_started_still_running_stopped_once` and `edge_detector_matches_poe2_monitor` | awaiting Windows run |
| 6 | second instance exits 3, first prints reveal | observed: second printed `another instance is already running; signalled it, exiting with 3`, `B exit code: 3`; first printed `reveal requested` between scans. Also covered by `second_process_fails_and_reveals_primary` (child process via `current_exe`) | awaiting Windows run |
| 7, 8 | tray | default build: `tray skipped on Linux (v2.1.1 parity: Windows only)`. With `--features tray-everywhere`: `tray created (tray-icon/ksni)` on the session bus (no visible host to click headless) | awaiting Windows run |
| 9 | exit + CPU | observed with `S6_SKIP_AUDIO=1`: `done after 15.0 s; process CPU time 0.14 s (0.9% of one core)` (debug build, monitor + one extra full scan every 3 s). Micro-benchmark: 5 sysinfo refreshes of 456 processes = 30 ms CPU, i.e. ~0.2% at 3 s | awaiting Windows run |
| 10 | re-acquire after exit | covered by `second_acquire_in_same_process_fails_and_reveals_primary` (drop then acquire again) and by running the example repeatedly | awaiting Windows run |
| build | `cargo check -p echosight-platform --all-targets --target x86_64-pc-windows-msvc` | passes on Linux (cross-check, no linker); clippy `-D warnings` for that target also clean | real Windows build awaiting CI dispatch |

Test run (Linux, `cargo test -p echosight-platform`): 23 passed, 0 failed, among
them `decodes_wav_fixture`, `decodes_ogg_vorbis_fixture`, `decodes_mp3_fixture`,
`decodes_m4a_aac_fixture` (0.15 s mono 22.05 kHz tones under `tests/fixtures/`,
generated with ffmpeg), `decodes_bundled_winamp_complete_wav`,
`effective_volume_matches_theme_sounds_ts` (0.6 / 0.6 / 0.4 / 1.0 / 0.0 plus
NaN and infinity fallbacks), `bundled_tray_icon_decodes_to_rgba`
(`tray-icon.ico decoded: 256x256`), `sysinfo_lists_processes_on_this_machine`
(`matching processes: []`, 456 processes seen). The `play_without_device...`
test takes ~14 s on this box for the reason in row 2.

## Decisions

- **rodio 0.23 API**: `Sink` is now `Player`, `OutputStream` is `MixerDeviceSink`,
  built with `DeviceSinkBuilder::open_default_sink()`; `Decoder::new(Cursor<Arc<[u8]>>)`
  decodes from memory and sniffs the container. Decoded clips are kept as
  `Arc<[f32]>` + channels + rate and replayed through `SamplesBuffer`, so the
  cache holds PCM, not file bytes.
- **Device on a worker thread, never the UI thread**: the 14 s blocking open
  observed here would freeze GPUI if done inline. Opening fails fast on normal
  machines but the design must not depend on that.
- **sysinfo**: keep one `System` alive, refresh with names + exe only, no tasks.
  Matching exe file names in addition to names covers truncated `comm` names on
  Linux (15 chars) and renamed launchers.
- **single instance**: hand-rolled (the `single-instance` crate is unmaintained).
  App id is a parameter so tests use unique ids and never collide with a real
  running Echosight.
- **tray on Linux**: default build does not link `tray-icon` at all (the GTK
  backend needs system libs); `tray-everywhere` selects `ksni` which compiled
  here with no system packages and registered on the session bus. macOS links
  `tray-icon` unconditionally (no system deps) but `should_create` stays false
  for parity.

## Risks and integration plan

1. **Win32 message loop ownership.** `tray-icon` (and `global-hotkey`, spike S3)
   create hidden windows on the thread that calls them and deliver events
   through that thread's message pump via `WM_*` messages; `muda` menus are
   the same. GPUI owns the main thread's `GetMessageW`/`DispatchMessageW` loop.
   Plan: create `AppTray` and the hotkey manager **inside `Application::run`'s
   startup closure on the main thread**; GPUI's loop dispatches their
   hidden-window messages without any extra pump because `DispatchMessageW`
   routes by `HWND`. Callbacks run on the main thread inside that dispatch, so
   they must only post to GPUI (`cx.spawn` / channel) and return. Verify in
   Phase 4 with a GPUI window + tray in one process; if GPUI ever filters
   messages by its own HWNDs (it does not today) fall back to `set_event_handler`
   plus a GPUI timer that drains `TrayIconEvent::receiver()`.
2. **Callback thread affinity.** On Windows tray callbacks arrive on the main
   thread; on macOS on the main thread too; with ksni on a background thread.
   Treat them as "unknown thread" and forward through a channel.
3. **Audio device churn / none present.** First open can block (seen: 14 s)
   and fail; headphones unplugged mid-session drops the stream. The worker
   backs off 10 s and retries on the next play. Consider a watchdog that
   recreates the sink on `cpal` stream errors (rodio logs them today).
4. **sysinfo privileges.** `exe()` can be `None` for processes of other users
   (permission denied on `/proc/<pid>/exe`, protected processes on Windows);
   name matching still works, which is what v2.1.1 relied on.
5. **Named pipe hijack / stale state.** `PIPE_REJECT_REMOTE_CLIENTS` and a
   per-session `Local\` mutex are used; a crashed primary releases both
   automatically (kernel objects). On Unix a crashed primary leaves a socket
   file; `acquire` detects `ECONNREFUSED` and unlinks it.
6. **Linux CI time.** `cargo test` includes one real device open attempt
   (14 s here, instant on CI runners without ALSA devices).
7. **Lockfile merge.** This branch touches `Cargo.lock` and
   `crates/echosight-platform/Cargo.toml` (new deps only) and appends `pub mod`
   lines at the end of `src/lib.rs`, so merging with the GPUI sibling branch
   should be mechanical; regenerate the lockfile after merge.
