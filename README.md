# Echosight - Path of Exile Task Overlay

Echosight is a lightweight desktop overlay for tracking Path of Exile league-start goals, character progression, and endgame objectives while staying in-game.

## Links

- Website: https://echologist.github.io/echosight-overlay/
- Repository: https://github.com/echologist/echosight-overlay

## Features

### Overlay

- Click-through mode for viewing tasks without blocking game input.
- Interactive mode for editing tasks, templates, settings, and hotkeys.
- Automatic visibility when Path of Exile is running.
- Always-on-top window behavior for in-game use.

### Task Management

- Add, complete, delete, and reorder tasks.
- Track progress with a visual progress bar and completed-task count.
- Organize tasks with parent and child relationships.
- Use `Ctrl+Shift+N` to complete the next task.
- Use `Ctrl+Shift+Z` to undo recent task actions.
- Confirm destructive actions before deleting a task or clearing all tasks.
- Keep the last 5 task-state snapshots for recovery.

### Background Tasks

Background tasks stay hidden until a trigger task is completed. They are useful for objectives that become relevant later, such as checking a spawned objective after completing a campaign or atlas milestone.

- Create background tasks from a task's "Configure Triggers" menu.
- Link one or more background tasks to a trigger task.
- Activated background tasks appear in a separate section below the main task list.
- `Ctrl+Shift+N` completes main-chain tasks first, then active background tasks.
- High-priority background tasks use a stronger visual indicator.
- Optional expiration timers can hide time-limited background tasks.

### Templates

- Save current task lists as reusable templates.
- Load built-in community templates.
- Import and export templates as JSON files.
- Preserve background task trigger configuration in templates.

### Customization

- Adjust overlay transparency.
- Choose from bundled visual themes.
- Record custom global hotkeys.
- Drag the overlay to a preferred position.
- Persist settings between sessions.

## Quick Start

### Build from Source

```bash
git clone https://github.com/echologist/echosight-overlay.git
cd echosight-overlay
npm install
npm run dev
```

### Common Commands

```bash
# Start development with hot reload
npm run dev

# Build for production
npm run build

# Preview the production build
npm run preview

# Create a distributable package
npm run pack

# Run tests
npm test
```

## Usage

### Basic Controls

| Action | Default |
| --- | --- |
| Show or hide overlay | `Ctrl+Shift+T` |
| Toggle interactive mode | `Ctrl+Shift+I` |
| Complete next task | `Ctrl+Shift+N` |
| Undo last task action | `Ctrl+Shift+Z` |
| Add task | Type in the task field and press Enter |
| Delete task | Click the `X` button next to a task |

All hotkeys can be changed from the settings modal.

### Modes

#### Click-Through Mode

Use click-through mode while playing. The overlay remains visible, but mouse input passes through to the game.

#### Interactive Mode

Use interactive mode when editing the task list. This mode enables task creation, deletion, reordering, template management, imports, exports, and settings.

### Background Task Workflow

1. Right-click a task and select "Configure Triggers".
2. Create a background task in the modal, or select an existing one.
3. Save the trigger configuration.
4. Complete the trigger task.
5. The linked background task becomes visible in the background section.
6. Press `Ctrl+Shift+N` again after main-chain tasks are complete to complete active background tasks.

### Undo and Recovery

- `Ctrl+Shift+Z` restores the most recent task-state action.
- Undo covers accidental task completion, task deletion, and clear-all.
- Individual task deletion requires confirmation.
- Clear-all requires confirmation and shows how many tasks will be removed.
- The app stores the last 5 snapshots alongside the current task state.

### Templates

#### Create a Template

1. Build the task list you want to reuse.
2. Click "Save" in the template section.
3. Enter a descriptive template name.

#### Use a Community Template

1. Click "Community Templates".
2. Browse the available templates.
3. Add the chosen template to your local templates.
4. Load it when starting a new character or league.

#### Share a Template

1. Select a saved template.
2. Click "Export" to download it as JSON.
3. Share the file.
4. Other users can import it through the "Import" button.

## Hotkey Guidance

- Prefer `Ctrl+Key`, `Alt+Key`, or function-key combinations.
- Avoid single-letter hotkeys that conflict with typing.
- Avoid combinations already used by Path of Exile or other overlays.
- Examples: `Ctrl+F1`, `Alt+Q`, `Ctrl+Shift+T`.

## Technical Details

### Built With

- Electron
- Node.js
- Native global shortcut registration

### System Requirements

- Windows 10 or Windows 11, 64-bit
- About 150 MB disk space
- No additional runtime dependencies

### Performance and Safety

- Minimal CPU usage.
- Low memory footprint.
- No game memory reading.
- No game file modification.
- No gameplay automation.
- No keystrokes or clicks sent to the game.

Echosight is a user-managed task overlay. It displays information you enter manually and behaves similarly to other non-automation overlays.

## Credits

Initial inspiration: Luca.
