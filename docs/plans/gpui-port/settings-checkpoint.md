# Settings UI and accelerator checkpoint

Status: independently approved bounded development checkpoint, not full native settings/hotkey acceptance. Implementation reviewed at `be80fddf019cea062d8d496359654226c6c977c5`; subsequent publication documentation does not change its source. Electron remains shippable and unchanged. No release or cutover is implied.

## Included

- Additive Storage `load_with_origin` distinguishes missing, existing and recovered files without changing legacy load values, recovery or save-blocking rules.
- Missing/null settings persist normalized renderer defaults; raw main-process committed hotkeys remain distinct from renderer-live settings and displayed controls.
- Settings writes are immutable FIFO snapshots. Successful writes update committed mappings; failures retain live controls and are visible after the originating dialog closes and inside any newer opaque modal without replacing its draft, focus or selection. Successful retry clears the failure.
- Bounded settings/reset controls and typed recorder business-rule effects preserve shipped stop/restore, switch, duplicate, stale 200 ms callback and Save ordering quirks. These are interfaces, not native capture.
- Typed overlay registration/release summaries, delayed startup registration and tray-command/quit-readiness interfaces. Global task-command routing preserves modal drafts and revalidates stale targets and duplicate IDs.
- Explicit Electron accelerator conversion, separate from the existing main normalization. It preserves source trimming, unknown-prefix overwrite, last nonmodifier key, accumulated Shift, Plus=Shift+Equal and patched Windows Cmd/MOD_WIN semantics. Unresolved AltGr/backend mappings fail explicitly rather than silently changing the chord.

## Reproduction

Use the pinned Rust toolchain and the GPUI host prerequisites described in the port documentation. A headless Linux startup does not establish a usable native window.

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo check --locked -p echosight --all-targets --target x86_64-pc-windows-msvc
cargo check --locked -p echosight-platform --all-targets --target x86_64-pc-windows-msvc
cargo check --locked -p echosight-theme-cli --all-targets --target x86_64-pc-windows-msvc
cargo build --locked -p echosight
cargo build --locked -p echosight-platform --example s8_task_app
```

Run from an unrelated working directory with a new explicit disposable development path:

```sh
/path/to/echosight --data-dir /absolute/path/to/disposable-development-data
```

The Windows S8 harness likewise requires `--data-dir <disposable-development-directory>`. Never point either executable at the installed Electron profile: its actual identity and migration/rollback path remain unverified.

Run all nine `tools/golden/vitest.*.config.ts` suites using the existing project Node dependencies. Set `THEMES_GOLDEN_CHECK=1`, `TEMPLATES_GOLDEN_CHECK=1`, `SETTINGS_GOLDEN_CHECK=1`, `BACKGROUND_GOLDEN_CHECK=1` and `THEME_LIBRARY_GOLDEN_CHECK=1` before checking their fixtures. Template-UI and settings-UI suites verify source provenance. The older task/storage generators rewrite fixtures, so inspect their resulting diff and require unchanged bytes. Do not regenerate before provenance checks.

## Verified evidence

- Parent fresh, unseeded combined target: **341 Rust passed, zero failed, one existing ignored**, including 57 named added probes across storage/settings/overlay/accelerators/correction tests.
- **152 TypeScript tests passed** across all nine source-backed suites; every tracked file retained identical bytes after execution.
- Formatting, workspace all-target Clippy, Windows app/platform/CLI cross-checks, actual Linux application/S8 links and accelerator provenance check passed.
- Executable without arguments exited 2; license output exited 0. Initial and repeat headless starts reached the window-open log and ended under deliberate six-second timeouts. Only `settings.json` was created; reopening preserved its bytes and modification time. These are bounded startup/persistence checks, not Windows or visual acceptance.
- Independent final UI review: pristine UI 127 passes, six correction regressions and three additional scratch probes for long-error scrolling and failure/success/FIFO readiness. Platform 38 passes. Accelerator/storage/oracle blobs match previously reviewed topics.
- Independent accelerator review: 154 source-derived rows, 406 raw/main-normalized pairs, nine upstream-case converter observations and 2,709 helper observations; additional 6,916 actual Electron acceptance comparisons and 20,748 target identity/error comparisons passed. Acceptance and source-derived chord identity are separate from OS registration.

## Native gates still open

- Lossless native recorder input, DOM/layout/dead-key/AltGr translation and consumption; physical key release, default actions, IME/composition and UTF-16 surrogate handling.
- Native registration ownership/debt, conflicts, repeat parity and OS dispatch. The underlying library's Windows repeat suppression is not resolved by the parser.
- Production tray/bin/OS wiring and native show/hide/click-through/game/focus behavior.
- Theme selection/v2 rendering, transparency/audio/position effects and all 11-theme visual acceptance.
- Actual Windows interaction, clipboard/IME/DPI/accessibility and installed-profile migration/rollback.

The isolated S9 recorder experiment is **absent** from this checkpoint. Its reviewed correction removes unsafe manual message retrieval and proves a genuine pending-prefix timeout in a headless probe, but decoding and state reconciliation still block production integration. A separately built normal Windows diagnostic executable must not be confused with the optional GPUI test harness, which occupies the same example binary slot.
