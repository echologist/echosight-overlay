# Runnable task application and remaining feature integration

Status: local integration rehearsal on `gpui-port-feature-integration`; independent review pending. **Not the finished application.** Base: `98af9e5` (PR #16 on `gpui-port`).

## User-visible feature status

- Task list/subtasks: runnable native GPUI window with recursive rows, add/edit/delete/completion/progress and local undo/redo. Real dispatched input tests; native desktop appearance/input not accepted yet.
- Drag/drop: wired pointer reorder/nest/promote, intent placeholder, strict 50px snapping and invalid-target guards. Scroll-aware viewport tests pass. Offscreen autoscroll, keyboard dragging and native visual testing remain pending.
- Saving: actual off-UI-thread FIFO writes, no debounce, explicit development-directory argument, successful reload tests and failure gates. No implicit access to legacy user data. Not yet full application startup/migration/lifecycle parity.
- Templates: core normalization/save/load/import/export/community behavior implemented and golden-tested; dialogs, native pickers, timer/history integration and application persistence not wired yet.
- Settings/hotkeys: core normalization/migration/defaults/accelerator/display/recording effects implemented and golden-tested; settings modal, preview/cancel behavior and OS global registration not wired yet.
- Background tasks: core trigger/activation/expiry/undo-rescheduling effects implemented and golden-tested; actual GPUI wakeups, trigger dialogs, notification/audio and persistence effect consumption not wired yet.
- Custom themes: v1 discovery/sync/variants/asset containment/sound resolution implemented and golden-tested; theme chooser/reload, v2 conversion, GPUI style runtime and custom/bundled visual acceptance not wired yet. CSS is returned only as compatibility text, never executed by GPUI.
- Overlay integration: previously tested native window/support primitives exist, but this task app does not yet wire the full click-through/global hotkey/game-monitor/tray/single-instance/audio lifecycle.

The application currently uses a bounded mapping of original default-theme colors. It is not evidence that arbitrary custom themes or the 11 bundled theme appearances work in GPUI.

## Frozen inputs

- Templates: `d263c902451fc429187c783e9266fd48e3223d12`.
- Settings: `f15d62118e1ca20e3f7006f2130b37edc006531f`.
- Background tasks: `799da5bf1b7709d41508410834d11e3f9460ab40`.
- Theme filesystem pipeline: `d2d249e8e7807c71b4f2e39406fdee1a74aff466`.
- Task application: `65151f5313e6d7d700654503f972745f59ef3de8`.

All inputs were clean, committed topic worktrees. The parent combined them without textual conflicts; the combined lockfile works with `--locked`. Production Electron sources, package files, icons and existing workflows are unchanged.

## Parent verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo test --locked --workspace`: **149 passing Rust test functions**, no failures/ignored tests. This is not a count of golden scenarios.
- Windows MSVC-target all-target checks for both `echosight` and `echosight-platform`: pass; not native execution.
- Seven isolated TypeScript suites: **115 tests pass** (themes 15, templates 2, settings 16, background 16, theme library 13, tasks 33, storage 20). Read-only freshness modes run before rewrite-only generators. Golden fixture diff remains empty.
- Linux `echosight` application and `s8_task_app` example link successfully.
- Executable without `--data-dir` exits 2 with usage and no data access.
- Real executable launched from an unrelated scratch directory with a new explicit data path: task window opened through headless platform, externally terminated after three seconds (124), no startup data directory created. **Not a screenshot, visual approval or native Windows pass.**
- Independent review approved the background scheduler. Corrections are required before publication: fallible JSON numeric coercion and exact radix rounding in template/settings normalization, integer-key ordering in raw hotkey maps, sequential destination-root retargeting in theme sync, and visible dialog actions/modal Escape priority in the task app. Fixes are isolated on `gpui-port-coercion-fixes` plus the original theme-library and app-task-ui topics. Re-review and combined gates remain required. Theme v2 contract/resolver/conversion work proceeds independently in its own worktree; it does not imply these review gates passed.

## Next implementation seams

1. Background UI integration: make the application consume `BackgroundTaskState` commands/effects, including real wakeups and persistence payloads without recording duplicate snapshots. Add a move command that preserves scheduled target identity during reorder/nest/promote; never mutate the facade's underlying tree behind the scheduler. Test drag followed by expiry, delete/undo/redo/reload and stale wakeups.
2. Template/settings flows: connect the reviewed modules to real dialogs and the existing persistence queue, retain original save/cancel/confirmation/focus behavior. Add load-template cancellation/restart semantics and ensure templates/settings writes do not corrupt the task-save queue.
3. Theme v2 contract/converter and runtime: implement the master-plan token/slot/state vocabulary and diagnostics; keep source v1 bundled files available to the unchanged Electron app until cutover. Store ported v2 resources separately during coexistence. Wire custom loading/selection/reload/assets/sounds and all 11 bundled themes, then obtain actual visual evidence.
4. Native lifecycle and remaining dialogs: integrate hotkeys/monitor/tray/single instance/click-through/focus/audio, then execute the complete feature checklist. S5 acceptance and native application tests remain required.
5. Legacy-data compatibility, cross-platform checks and packaging: no in-place writes until the real legacy path is evidenced and sanitized-copy round trips pass. No automatic main cutover or release tag.

Use `parity-templates.md`, `parity-settings.md`, `parity-background.md`, `parity-theme-library.md`, and `parity-task-ui.md` for API details and source-quirk boundaries. These slice documents do not replace the final feature-level parity matrix.
