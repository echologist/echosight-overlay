# Echosight

<p align="center">
  <img src="docs/assets/icon.png" alt="Echosight logo" width="160">
</p>

Echosight is a desktop task overlay for persistent checklists. It was originally built for Path of Exile players, including Path of Exile 1 and 2, but it also works as a general overlay for office work, live operations, study sessions, and other workflows where a compact task list needs to stay visible.

The project is built with Electron, TypeScript, and Vite. It stores user tasks, templates, settings, and themes locally.

## Features

- Always-on-top overlay window with click-through and interactive modes.
- Hierarchical task lists with subtasks, progress tracking, completion state, and deletion.
- Undo and redo for recent task actions.
- Drag-and-drop task reordering.
- Background tasks that can be activated by completing specific main tasks.
- Template import, export, save, and load support.
- Built-in community templates for common game and workflow goals.
- Theme support with bundled themes and local custom theme files.
- Configurable transparency, window position, and global hotkeys.

## Controls

Default hotkeys:

- `Ctrl+Shift+T`: show or hide the overlay.
- `Ctrl+Shift+I`: toggle interactive mode.
- `Ctrl+Shift+N`: complete the next available main task.
- `Ctrl+Shift+Z`: undo the last task action.
- `Ctrl+Shift+Y`: redo the last undone task action.

Hotkeys can be changed from the settings window.

## Modes

### Interactive Mode

Use interactive mode to manage the overlay:

- Add, complete, delete, and reorder tasks.
- Configure background task triggers.
- Save or load task templates.
- Change theme, transparency, hotkeys, and position.

### Click-Through Mode

Use click-through mode when the overlay should remain visible without capturing mouse input. Editing controls are hidden, and mouse input passes through to the application underneath.

## Background Tasks

Background tasks are optional objectives that become relevant after a main task is completed. For example, a campaign task can activate a reminder to watch for a specific item type, or an office checklist can activate follow-up reminders after a review step is completed.

To configure them:

1. Right-click a task.
2. Select `Configure Triggers`.
3. Create or select background tasks.
4. Save the trigger configuration.

When the trigger task is completed, linked background tasks appear in the background task section. The `Ctrl+Shift+N` hotkey only advances main tasks.

## Templates

Templates allow task lists to be reused or shared.

- Save the current task list as a named template.
- Load a saved template when starting a new run, character, project, or recurring workflow.
- Export templates as JSON files.
- Import shared template JSON files.
- Use bundled community templates as a starting point.

Template data preserves hierarchy, background task configuration, and trigger relationships.

## Themes

Echosight ships with several bundled themes and supports custom theme folders. Themes are stored under the application data directory and can include JSON configuration, CSS, and image assets.

Use the settings window to open the local themes folder or reload themes after editing them.

## Development

Requirements:

- Node.js `^20.19.0` or `>=22.12.0`
- npm

Install dependencies:

```bash
npm install
```

Run the development app:

```bash
npm run dev
```

Run checks:

```bash
npm run typecheck
npm test
npm run build
```

Create a packaged build:

```bash
npm run pack
```

Platform-specific builds:

```bash
npm run build-win
npm run build-mac
npm run build-linux
```

## Project Structure

- `src/main`: Electron main process, hotkeys, IPC, theme loading, and overlay window management.
- `src/preload`: preload bridge exposed to the renderer.
- `src/renderer`: renderer UI, task workflow, settings, templates, themes, and dialogs.
- `src/shared`: shared TypeScript types and utilities.
- `data`: default settings, templates, and bundled themes.
- `tests`: unit tests for main and renderer modules.

## Safety

Echosight is a manual overlay. For game use, it does not:

- Read game memory.
- Read or modify game files.
- Send gameplay input.
- Automate gameplay.
- Inspect network traffic.

It behaves like a task list displayed above another application. Users are responsible for making sure any third-party tool use complies with the rules that apply to their game, workplace, or platform.

## Credits

Major TypeScript refactor, renderer modularization, tests, and theme work by Flycro.

Maintained by Echologist.

## License

MIT
