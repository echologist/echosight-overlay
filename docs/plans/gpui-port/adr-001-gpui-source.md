# ADR-001: GPUI source

Status: **accepted** (2026-11-09). Revisit in ADR-002 (component library) and at every phase boundary.

## Context

Master plan task 0.5. The research report (`gpui-capability-research.md`) found that crates.io
`gpui` 0.2.2 (2025-10-22) is a year stale and lacks the platform backends, which now live in
separate zed crates (`gpui_platform` facade with `gpui_platform::application()`, plus
`gpui_windows`, `gpui_macos`, `gpui_linux`, `gpui_wgpu`), none of which are published by Zed.
Candidates:

- (a) git dependency on `zed-industries/zed` at a pinned rev for `gpui` + `gpui_platform`;
- (b) Longbridge's weekly crates.io snapshots `gpui-pre` + `gpui-pre-platform` (0.3.x);
- (c) community fork `gpui-ce` (0.3.0, 2026-10-20; not evaluated further: a single release
  of a fork with no platform crate split and no stated cadence).

Both (a) and (b) were wired into `crates/echosight-platform` (and a placeholder `gpui`
dependency in `crates/echosight-ui`) and exercised with the real S1 example
(`examples/s1_window.rs`: `WindowOptions`, `WindowKind::PopUp`, transparent background,
`window_control_area(Drag)`, raw-window-handle access, display queries).

## Measurements

Box: Linux x86_64, 10 cores, Rust 1.99.1, no display, no `-devel` packages, no sudo
(see "Build environment notes"). Clean builds use a fresh `CARGO_TARGET_DIR`; the release
profile is the workspace one (`opt-level=3`, thin LTO, `codegen-units=1`, `strip`,
`panic=abort`). Wall times are `/usr/bin/time -v` "Elapsed".

| | (a) zed git pin | (b) `gpui-pre` |
|---|---|---|
| Exact source | `rev = 4ed72e0135aaab74a43914491c915dd6cbd10c67` (zed main, committed 2026-11-09 16:24 UTC) | `gpui-pre = "=0.3.12"`, `gpui-pre-platform = "=0.3.12"` (published 2026-11-09, "snapshot of zed@d2a4fb6" = 2026-11-08 20:08 UTC) |
| Resolves | yes | yes |
| Resolve/fetch cost | `cargo fetch` 2 min 06 s cold; `~/.cargo/git/db/zed-*` 410 MB + 99 MB checkout | `cargo fetch` 7 s cold; registry crates only |
| Extra workspace config | `[patch.crates-io]` mirroring zed's pins for `async-task`, `calloop`, `khronos-egl` (patches do not propagate from a git dependency) | none |
| Linux debug build (`cargo build -p echosight-platform --examples`) | ok | ok |
| Linux clean release `--example s1_window` | **2 min 51 s** wall (1336 s user), 449 crates compiled | **2 min 35 s** wall (1218 s user), 445 crates compiled |
| Release binary (Linux, stripped) | **20.0 MB** (20 001 656 B) | **20.3 MB** (20 294 488 B) |
| `cargo tree -p echosight-platform -e normal \| wc -l` | 1135 lines; 553 unique packages (Linux), 328 (Windows target) | 1126 lines; 548 unique (Linux), 333 (Windows) |
| `cargo check --examples --target x86_64-pc-windows-msvc` | ok (43 s incremental over Linux cache) | ok (43 s) |
| `cargo clippy --all-targets -D warnings` (Linux and Windows target) | clean | not run separately (same code compiled) |
| Code changes needed between (a) and (b) | **none**: identical API at these revisions | |
| Renderer deps | wgpu **git** `cddb2f5d` (zed's pin: includes the Vulkan surface-drop fix wgpu#10230 and WebGL resize fix wgpu#10348 that 30.0.1 lacks), zed's `async-task` fork, zed's `calloop`/`khronos-egl` forks | wgpu **30.0.1** from crates.io, upstream `async-task` 4.7.1, `calloop` 0.14.5, `khronos-egl` 6.0.0 |
| Crate names in the graph | `gpui`, `gpui_platform`, `gpui_linux`, `gpui_windows`, ... | `gpui-pre`, `gpui-pre-platform`, `gpui-pre-linux`, `gpui-pre-windows`, `gpui-pre-wgpu`, `zed-font-kit`, `zed-xim`, ... (renamed on publish; used as `gpui` via `package =`) |

API notes at both revisions (identical): `gpui_platform::application()` exists and replaces
`Application::new()`; `WindowOptions` fields are `window_bounds`, `titlebar`, `focus`,
`show`, `kind`, `is_movable`, `app_owns_titlebar_drag`, `inactive_frame_interval`,
`low_power_frame_interval`, `thermal_frame_interval`, `is_resizable`, `is_minimizable`,
`display_id`, `window_background`, `app_id`, `window_min_size`, `window_decorations`,
`icon`, `tabbing_identifier`, `use_display_p3` (`Default` provided).
`Window` has an inherent `window_handle()` returning GPUI's `AnyWindowHandle`; the raw handle
needs the fully qualified trait call `raw_window_handle::HasWindowHandle::window_handle(window)`.
`raw-window-handle` must be requested with `features = ["std"]` so `HandleError: Error`
(on Linux another crate enables it, on the Windows target nothing does).
`gpui_platform::application()` uses `cfg_select!`, so the toolchain must be recent (1.99.1 ok).

## Decision

**(a): git dependency on zed at an exact 40-hex rev**, currently
`4ed72e0135aaab74a43914491c915dd6cbd10c67` (2026-11-09), with features
`gpui = ["font-kit", "x11", "windows-manifest"]`, `gpui_platform = ["x11"]` (Wayland is added
in spike S7 once the policy in ADR-004 exists), plus the mirrored `[patch.crates-io]` entries.

Rationale:

1. The two options are equivalent in build time (16 s apart), binary size (0.3 MB) and API;
   (b) is "smoother" only at dependency-resolution time (7 s vs 2 min once per machine, and
   500 MB of git cache). That does not outweigh the points below.
2. (a) is what Zed itself builds and ships: the exact wgpu rev and forks Zed tests on Windows.
   (b) silently swaps wgpu for the crates.io release Zed deliberately avoids.
3. The master plan's go/no-go fallback for S2/S3 is to contribute `PlatformWindow` methods
   upstream and pin a fork rev until merged. That is a one-line change with (a)
   (`git = "<fork>", rev = ...`) and impossible with (b) without waiting for Longbridge.
4. (a) depends on Zed only; (b) adds a third party (Longbridge) between us and the source,
   with renamed crates that make upstream diffs and issue reports harder to map.
5. Upgrade control is identical: both are exact pins; we bump deliberately once per phase and
   keep `rust-toolchain.toml` in step (zed's own pin is 1.98.1; we use 1.99.1 and it builds).

## Consequences and follow-ups

- Bump procedure: change the two `rev` values and the three `[patch.crates-io]` revs
  together (copy them from zed's `Cargo.toml` at the new rev), re-run the three gates
  (Linux build, Windows check, clippy), rebuild the spike binaries.
- **gpui-component / gpui-kit (ADR-002, spike S5)**: it pins `gpui = { package = "gpui-pre",
  version = "=0.3.12" }`. A binary cannot contain both `gpui` (git) and `gpui-pre`
  (crates.io): every `Window`, `App` and element type would be duplicated. Adopting
  gpui-component therefore means either switching this ADR to (b) and tracking Longbridge's
  cadence (their snapshot wins, as the master plan already anticipates), or verifying in S5
  that `[patch.crates-io] gpui-pre = { git = "<zed>", package = "gpui", rev = ... }` lets
  gpui-component build against our pin (unverified; depends on how far the snapshot's crate
  renames leak into gpui-component's code). Do **not** add gpui-component before S5.
- CI: the Linux job needs `libfontconfig-dev`, `libxkbcommon(-x11)-dev`, `libx11-xcb-dev`
  (already in `rust.yml`); Windows needs MSVC + Windows SDK (for `rc.exe`, used by
  `embed-resource` through the `windows-manifest` feature); no CMake in this graph.

## Build environment notes (this Linux box; not needed on CI)

- First Linux build failed with `yeslogic-fontconfig-sys`: `Package fontconfig was not
  found in the pkg-config search path` (no `-devel` packages, no sudo). Fixed locally with
  `~/.local/gpui-shim` (unversioned `.so` symlinks to the installed runtime libraries and
  stub `.pc` files for fontconfig, freetype2, xkbcommon(-x11), x11(-xcb), xcb, wayland-*,
  vulkan, egl, openssl) plus `PKG_CONFIG_PATH` and `-L` in `RUSTFLAGS`
  (`~/.local/gpui-shim/env.sh`). The same trick as the pre-existing alsa shim.
- The Windows cross `cargo check` first failed in gpui's build script:
  `called Result::unwrap() on an Err value: NotAttempted("llvm-rc")` (`windows-manifest`
  feature, which `gpui_platform` forces on for Windows targets). No `llvm-rc`/`windres` is
  installed and `rustup component llvm-tools` does not ship it. Fixed locally with a shim
  `llvm-rc` that execs `zig rc` (resinator, from the `ziglang` pip wheel, rc.exe-compatible;
  the shim maps `/no-preprocess` to `/:no-preprocess`) and `CC_x86_64_pc_windows_msvc=gcc`
  for `embed-resource`'s preprocessing step. The compiled resource is a genuine
  `Microsoft Visual C binary resource file`; it is never linked on Linux.
- Headless run: the Linux examples start under GPUI's headless platform (no display),
  log `no primary display reported; assuming 1920x1080`, open the window object and run the
  tick loop; the raw handle is unavailable there, so native ops log
  `FAILED (no native handle)`. That exercised the shell, timers and spike logic end to end
  without a GPU; it says nothing about Windows behaviour (spike docs are "awaiting Windows run").
