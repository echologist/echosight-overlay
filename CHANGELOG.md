# Changelog

All notable changes to Echosight are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased - Rust/GPUI development branch

These changes apply only to the experimental `gpui-port` branch, not the shipped Electron application.

### Added

- Settings load-origin reporting, source-backed settings UI observations, immutable save queues, reset controls and typed recorder/overlay integration interfaces.
- An explicit Electron-compatible accelerator parser with separate conversion errors and unsupported backend-mapping diagnostics. This does not register native global shortcuts.
- Modal-safe global task-command interfaces that preserve active drafts, selection and focus while retaining stale-target and duplicate-ID safeguards.

### Fixed

- Late settings-save failures remain visible after Settings closes or another dialog opens, without replacing its draft. Successful retries clear the failure.

### Verification and limits

- Reviewed checkpoint: 341 Rust tests passed, one existing ignored test, and 152 source-backed TypeScript tests passed. Formatting, Clippy, Windows cross-checks, Linux application linking and bounded headless startup/reopen checks passed.
- Native hotkey recording/registration, tray/OS wiring, theme/audio/position effects and Windows interaction acceptance remain pending. The isolated recorder experiment is not part of this application checkpoint. See `docs/plans/gpui-port/settings-checkpoint.md`.

## [2.1.1] - 2026-07-26

### Added

- Added a Windows system tray icon so users can see when Echosight is still running in the background.
- Added `Show Echosight`, `Hide Echosight`, and `Exit Echosight` tray actions, with left-click restoring and focusing the overlay.
- Added a dedicated multi-resolution Windows tray icon that preserves the Echosight eye at 16×16 through 256×256.

### Changed

- Centralized shutdown cleanup so explicit exit stops game monitoring, unregisters global shortcuts, and removes the tray icon before quitting.
- Limited tray integration to Windows, leaving macOS and Linux behavior unchanged.

## [2.1.0] - 2026-07-25

### Added

- Added direct editing for regular tasks, nested tasks, active background tasks, and dormant background tasks.
- Added validation that keeps the edit dialog open when a task name is empty or contains only whitespace.

### Changed

- Improved global shortcut reliability after the Echosight window loses focus.
- Improved spacing between background-task status badges and edit actions.

[2.1.1]: https://github.com/echologist/echosight-overlay/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/echologist/echosight-overlay/releases/tag/v2.1.0
