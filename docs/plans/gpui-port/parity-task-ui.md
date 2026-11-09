# Runnable GPUI task application slice

Status: executable, bounded task slice. **Not full renderer/platform parity.**
Native Windows S5 input acceptance and visual/style acceptance remain pending.

## Run

```sh
cargo run --locked -p echosight -- --data-dir <explicit-development-directory>
# Same entry point, selectable by the existing Windows example build harness:
cargo run --locked -p echosight-platform --example s8_task_app -- --data-dir <explicit-development-directory>
# Third-party input component notices:
cargo run --locked -p echosight -- --licenses
```

No implicit directory, legacy identity guess, or access to the user's live data.
Missing/empty `--data-dir` exits with usage and status 2 before GPUI starts.
Use a disposable directory, and only one process per directory. The reviewed
storage queue is process-local, not a cross-process lock.

## Implemented

- Real `TaskState` operations, reviewed `TextInput`, actual `Storage` reads/saves.
  No local mock task list or alternative mutation model.
- Root and recursively nested rows; root add, subtask dialog, edit (100 UTF-16
  units for new input, preserve longer loaded names, select existing text), background priority editing for loaded background
  rows, toggle completion, complete-next, domain progress count/bar, delete.
- One active task/quit dialog. Danger dialogs focus Cancel, ignore Enter on
  destructive confirmation, support explicit click/Space confirmation, cancel on
  Escape, trap Tab/Shift+Tab, and restore focus. Deleting a focused row returns
  focus to the composer so local undo remains reachable.
- Undo/redo buttons and window-local Ctrl/Cmd+Shift+Z / Ctrl/Cmd+Shift+Y.
  Core history/snapshot limits are unchanged.
- Pointer drag from row text: drag threshold, midpoint/half-width intent,
  nest/reorder/promote/sibling moves, nearest center snapping strictly within
  50px, rejection outside the visible list and for self/descendant/cross-list or
  ambiguous targets. Core validates moves again. Escape cancels a drag.
  The labelled insertion placeholder is an overlay on the target row, rather
  than a layout-displacing row; this keeps hit geometry stable.
- Rows use stable unique-ID keys; duplicate IDs get structural suffixes to avoid
  GPUI key collisions. ID-based core APIs cannot reliably address duplicate
  records: actions on duplicates or descendants of duplicate ancestors are
  visibly disabled, rather than claiming perfect duplicate-ID editing. Complete-next
  is also disabled for any ambiguous tree: core parent completion still resolves
  IDs even when leaf selection used a structural path. Deletion also rejects any
  subtree containing ambiguous IDs, protecting surviving trigger references.
- 500×700 window, minimum 450×600, core top-right placement using the primary
  visible display bounds, frameless draggable header. The original eye PNG is
  embedded unchanged. No runtime repository asset paths.
- Default theme colors are a bounded mapping of `data/themes/echosight.json`
  (gold, teal, text and dark surface), not a replacement design. Custom CSS,
  gradients, fonts and Theme V2 are not claimed to be rendered.

## Persistence safety and bounded differences

`TaskApp` starts with no mutable state. A background-executor load uses the
explicit `DataPaths` and shared `Storage`, migrates raw task structure, and then
performs typed decoding. Neither a missing file nor an ordinary successful load
writes data. Missing files use the **embedded original bundled task defaults**;
the first successful mutation persists them. Templates/settings are not seeded
by this task-only bootstrap. Legacy migration is retained in memory and persisted
on the first edit, not automatically written during startup.

Every mutation creates a core auto-save snapshot immediately. A FIFO starts only
one background save at a time, then starts the next only after completion. There
is no debounce or latest-write coalescing. No synchronous file I/O runs in view
handlers/rendering. Load/decode errors block all task mutations. Save errors keep
the unsaved visible state, stop the funnel, and disable subsequent mutations;
there is no automatic retry that could overwrite recoverable data. Restarting
loads the last successful disk state, not unsaved memory.

Core corruption recovery remains authoritative: it copies original bytes before
publishing fallback. The app displays the returned backup path and blocks editing
until restart, so recovery is not silent. Unsupported typed shapes are not
quarantined or normalized back onto disk.

Window-close requests open a Cancel-default quit confirmation. Confirmation is
disabled while saving; after a save error it explicitly warns of unsaved loss.
OS termination/crash cannot be made durable by this UI queue; wait for **Saved**.

## Source contract

- `src/renderer/features/tasks/taskWorkflowController.ts`: task workflow,
  persistence after mutations, edit priority and exact delete warning.
- `taskEditUi.ts`, `taskInputUi.ts`, `src/renderer/index.html`: edit selection,
  trimmed text and input limits.
- `taskTreeOperations.ts`, `dragDropUi.ts`: above/indented semantics, depth rule,
  self/descendant checks, nearest-center threshold and intent labels.
- `src/renderer/ui/dialogService.ts`, `modalUi.ts`: danger focus, Enter behavior,
  Escape, focus trapping/restoration. This slice has one task dialog, not a
  completed generic modal queue/stack for future settings/template dialogs.

## Executed evidence

On Linux, after applying the host's existing uncommitted GPUI build shim:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`: **103 passing Rust test functions**, including
  **43 UI tests**. New application coverage is **19 dispatched GPUI tests + 1
  pure bootstrap test**, plus **1 executable argument test**.
- `cargo check --locked -p echosight --all-targets --target x86_64-pc-windows-msvc`
- `cargo check --locked -p echosight-platform --all-targets --target x86_64-pc-windows-msvc`
- Linux application and `s8_task_app` example build; real executable headless
  launch logs the task window opening. Bounded smoke ends by external timeout,
  not a fabricated visual or native-window pass.

Dispatched tests exercise text input, clicks, edit selection, Tab focus, danger
Enter/Space policy, Escape, nested completion/progress, deletion, undo/redo,
reorder/nest/promote, bad drop targets, exact 50px exclusion, viewport bounds
following scroll, drag cancellation, duplicates, loaded background priority,
empty edits, quit cancellation, actual disk reload, per-mutation FIFO writes,
replacement failures, unsupported typed loads and corrupt-byte preservation.
Persistence completion uses GPUI executor/notification conditions, **no sleeps**.
I/O probes wrap reviewed Storage hooks and inspect real written files; they do
not replace the file store. Test regressions caught stale scrolled viewport
bounds and missing editor selection; both now run against the production view.

## Still unsupported / integration handoff

- No display is attached to this host. Headless geometry/event tests are not
  screenshots or approval of layout/typography. Native Windows IME, clipboard,
  focus restoration, DPI and visual acceptance are still required (S5 pending).
- No theme discovery, user-theme selection, custom CSS, Theme V2, bundled font
  pipeline, transparency/settings modal or exact full-theme styling.
- No templates/import/export/community/trigger-configuration dialogs. Dormant
  background rows are explicitly labelled in this development slice instead of
  being hidden behind the not-yet-integrated trigger configurator. Background
  activation/deactivation, expiration timers, sounds and completion toasts are
  not wired; priority and core progress/completion only are supported.
- No global hotkey registration, click-through mode, topmost reassertion,
  show/hide/tray/game-monitor/single-instance lifecycle integration, legacy
  application data-directory discovery, production packaging or migration gate.
- No offscreen drag autoscroll or keyboard drag/drop. Visible rows can be moved
  after scrolling normally. Full accessibility semantics and automatic reveal
  of a keyboard-focused offscreen row still need a separate pass.
- No cross-process data lock, retry/recovery wizard or unsaved-state export.
  This is explicitly a single-process development-directory task slice.

Windows manual acceptance: launch with a new explicit directory; add a root and
subtask; edit with selection; mark Done and verify progress; cancel and confirm
Delete; undo/redo locally; drag a sibling onto the lower-right half, promote it
to a root and reorder above/below; reject own-descendant and cross-list drops;
wait for Saved, quit and relaunch. Check eye branding, 450×600 layout, tab focus,
clipboard/IME and DPI on the actual Windows desktop. Do not mark these passed
from the cross-compile result.
