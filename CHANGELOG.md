# Changelog

All notable changes to Echosight are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [2.1.1] - 2026-07-26

### Added

- Added a Windows system tray icon so users can see when Echosight is still running in the background.
- Added `Show Echosight`, `Hide Echosight`, and `Exit Echosight` tray actions, with left-click restoring and focusing the overlay.
- Added a dedicated multi-resolution Windows tray icon that preserves the Echosight eye at 16×16 through 256×256.
- Added sound feedback to the Exile's Corruption theme for background task activation, task completion, undo, and redo actions.

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

[2.1.1]: https://github.com/echologist/echoesight-overlay/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/echologist/echoesight-overlay/releases/tag/v2.1.0
