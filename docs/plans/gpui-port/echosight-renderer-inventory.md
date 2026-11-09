# Echosight Overlay — Renderer Feature & Behaviour Inventory (non-theme scope)

Snapshot: `/home/agent/code/echosight-overlay`, branch `feat/exiles-corruption-sounds`, HEAD `5a63e5c`. Read-only analysis; nothing in the repo was modified. (Worktree was already dirty with unrelated changes — `.env.example`, `README.md`, `data/*.json`, etc. — before this analysis began.)

Scope: `src/renderer/**` except `features/themes/**`, plus `src/shared/{types,settingsDefaults,errors}.ts` and `tests/unit/{tasks,settings,ui}/**`.

Evidence labels: **[V]** = Verified by reading source/tests; **[I]** = Inferred from source but not exercised at runtime.

Line references are `path:line`.

---

## 0. File inventory (purpose, ~LOC)

### Composition root / static shell
| File | LOC | Purpose |
|---|---|---|
| `src/renderer/renderer.ts` | 329 | Composition root: wires every controller, defines `initializeApp`, `loadTasks`, `saveTasks`, `renderTasks`, `updateProgress`, binds DOM controls on `DOMContentLoaded`. |
| `src/renderer/index.html` | 476 | Static DOM: overlay container (header, template bar, task section) + 9 modals (`saveTemplateModal`, `importTemplateModal`, `communityTemplatesModal`, `settingsModal`, `themeSelectionModal`, `addSubTaskModal`, `configureTriggersModal`, `taskEditModal`, `appDialogModal`). Loads `renderer.ts` as ES module. |
| `src/renderer/styles.css` | 1498 | All layout/visual rules incl. click-through mode hiding, drag placeholder, context menu, toasts, modals, hierarchy indent. |
| `src/renderer/globals.d.ts` | 9 | Declares `window.echosight?: EchosightApi`. |

### features/tasks
| File | LOC | Purpose |
|---|---|---|
| `taskStateController.ts` | 272 | In-memory task state + undo/redo stacks (depth 20) + auto-save snapshots (5) + load/save funnel. |
| `taskWorkflowController.ts` | 333 | User-facing task workflows: add, toggle, complete-next, delete (confirm), clear-all (confirm), edit modal, reorder, undo/redo with timer restart + sounds. |
| `taskTree.ts` | 85 | Pure tree helpers: findTaskById, flattenTasks, collectTaskIds, findParentTask, findTaskWithParent, removeTaskFromParent. |
| `taskTreeOperations.ts` | 198 | Drop-operation classification (`getTaskDropOperation`) and move (`moveTaskInTree`) with messages & placeholder colours. |
| `taskCompletion.ts` | 121 | Toggle/complete-next semantics, parent auto-complete / auto-uncheck cascade, next-leaf search. |
| `taskCreation.ts` | 59 | `createTaskId`, `createTask`, `createBackgroundTask`, `sanitizeBackgroundOptions`. |
| `taskMutations.ts` | 91 | `addTaskToList`, `deleteTaskFromList` (removes trigger refs), `editTaskInList`. |
| `taskMigration.ts` | 35 | Fills missing fields on legacy task records. |
| `taskPersistence.ts` | 52 | `loadTaskState` / `saveTaskState` over `api.loadTasks/saveTasks` with normalisation. |
| `taskProgress.ts` | 41 | Progress counts (main + *activated* background, recursive). |
| `taskProgressUi.ts` | 45 | Progress bar/text DOM + 2 s "✓ Completed" feedback. |
| `taskListRenderer.ts` | 161 | Builds `<li class="task-item">` rows (drag handle, folder icon, checkbox, text, badges, hint, delete). |
| `taskListView.ts` | 40 | Renders main list + *activated* background list, toggles background section visibility/count. |
| `taskDragController.ts` | 253 | HTML5 drag/drop event handling for items and list containers, placeholder updates. |
| `dragDropUi.ts` | 178 | Drop intent geometry, placeholder DOM, nearest-item search, drag CSS classes. |
| `backgroundTaskController.ts` | 133 | Trigger activation/deactivation, expiry timers (setTimeout), restart on load/undo. |
| `backgroundTaskUi.ts` | 188 | "N background task(s) activated!" toast; Configure Triggers modal list rows/badges/edit button; input readers. |
| `triggerController.ts` | 103 | Configure Triggers modal workflow: open, save selection (undo-captured), add new background task from modal. |
| `subtaskController.ts` | 77 | Add Sub-Task modal workflow. |
| `subtaskUi.ts` | 44 | Add Sub-Task modal show/hide/read, refocus window after close. |
| `taskEditUi.ts` | 51 | Edit Task modal show/hide/read (text + High Priority for background). |
| `contextMenuUi.ts` | 61 | Right-click context menu (Edit task / Add sub-task / Configure Triggers). |
| `taskInputUi.ts` | 15 | Read/clear `#taskInput`. |
| `taskGuards.ts` | 3 | `isRecord` type guard. |
| `taskTemplates.ts` | 215 | Template <-> task conversion, normalisation, trigger index mapping, import parsing (uses `destr`). |

### features/templates
| File | LOC | Purpose |
|---|---|---|
| `templateController.ts` | 208 | Save / Load / Delete template workflows, template `<select>` refresh; composes community & transfer controllers. |
| `templateOperations.ts` | 121 | Pure ops: id parse, find, create (user/community/imported), upsert by name, remove, instantiate (with trigger remap), export data + filename. |
| `templatePersistence.ts` | 32 | `loadTemplateState` / `saveTemplateState` over `api.loadTemplates/saveTemplates`. |
| `templateTransferController.ts` | 140 | Export (download JSON), Import modal (textarea/file), import parsing + replace-confirm. |
| `templateUi.ts` | 147 | Save/Import modal DOM, `<select>` rendering, Blob download, FileReader. |
| `communityTemplates.ts` | 102 | Built-in community templates (6) + conversion to TemplateTask[]. |
| `communityTemplateController.ts` | 120 | Community modal workflow: list, import (replace-confirm), preview (alert). |
| `communityTemplatesUi.ts` | 97 | Community modal card DOM + preview text. |

### features/settings
| File | LOC | Purpose |
|---|---|---|
| `settingsController.ts` | 284 | Settings state, modal open/close/save, live transparency/sound/theme updates, hotkey recorder wiring, reset hotkeys. |
| `settingsDomain.ts` | 202 | Defaults (platform-aware Cmd/Ctrl), normalisation, migration detection, hotkey recording rules. |
| `settingsPersistence.ts` | 54 | Load (with migration persist) / save (+`api.updateHotkeys`). |
| `settingsUi.ts` | 216 | Settings modal DOM read/write, transparency disable, hotkey recording visuals. |
| `hotkeyRecorder.ts` | 139 | Recorder state machine (record/stop/handleKeydown/duplicate detection). |

### ui
| File | LOC | Purpose |
|---|---|---|
| `dialogService.ts` | 214 | In-DOM alert/confirm replacement (`#appDialogModal`), queued, keyboard rules, tones. |
| `dialogTypes.ts` | 16 | `DialogOptions`, `AlertHandler`, `ConfirmHandler`, no-op defaults. |
| `modalUi.ts` | 309 | Generic `showModal/hideModal`: visibility class, aria, focus scheduling/retries, focus trap, Escape-to-close, modal stack, focus restore. |
| `overlayController.ts` | 64 | Interactive-mode state + IPC listener registration (complete-next/undo/redo/interactive-changed). |
| `overlayUi.ts` | 72 | Interactive/click-through visuals, header window-drag region, global error logging. |
| `feedbackUi.ts` | 39 | Toasts: `showReorderFeedback` (3 s), `showThemesPathNotification` (8 s; **no caller found in src/renderer** [V]). |
| `staticControls.ts` | 144 | Binds all static button/input events by element id. |
| `domEvents.ts` | 92 | `bindClick/bindEnterKey/bindValueInput/bindValueChange/bindEvent/clickElement/isOutsideModalContent`, error-safe wrapper. |
| `windowCommands.ts` | 62 | Reset position (alert), minimize, quit (confirm), toggle interactive — thin IPC wrappers. |
| `windowFocus.ts` | 27 | `focusOverlayNow` (window.focus + api.focusWindow), `focusOverlaySoon` (setTimeout). |
| `hotkeyDisplay.ts` | 17 | `getPrimaryModifierLabel` (Cmd/Ctrl by `navigator.platform`), `formatHotkeyForDisplay`. |

### shared
| File | LOC | Purpose |
|---|---|---|
| `src/shared/types.ts` | 179 | All domain types + `EchosightApi` preload contract. |
| `src/shared/settingsDefaults.ts` | 24 | `CURRENT_SETTINGS_VERSION = 2`, default sounds `{enabled:false, volume:60}`, default settings `{transparency:70, theme:'echosight'}`. |
| `src/shared/errors.ts` | 3 | `getErrorMessage(unknown)`. |

### tests (behavioural spec)
`tests/unit/tasks/`: backgroundTaskUi (60), rendererPersistence (183), rendererTaskEditing (295), taskPersistence (88), taskProgressUi (63), taskStateController (300), taskWorkflowController (377), triggerController (84). `tests/unit/settings/`: hotkeyRecorder (173), settingsDomain (113), settingsPersistence (102), settingsUi (164). `tests/unit/ui/`: dialogService (226), modalUi (242).

---

## 1. Data model & invariants (`src/shared/types.ts`)

### Task [V] `types.ts:9-20`
```
id: number            // Date.now()*1000 + random(0..999)  taskCreation.ts:7-9
text: string
completed: boolean
createdAt: ISO string
children: Task[]      // arbitrary depth (rendering indents level*20px; CSS has special rules only for level 0/1)
mode: 'main' | 'background'
triggers: number[]    // ids of BACKGROUND tasks activated when this task completes
activated: boolean    // main tasks: always true; background: false until triggered
activatedAt: ISO | null
backgroundOptions: { expiresAfterMinutes: number|null; priority: 'normal'|'high' } | null
```
Invariants:
- Top-level `tasks[]` holds both main and background tasks mixed; lists are filtered by `mode` at render time (`taskListView.ts:18-19`). [V]
- Subtask inherits parent's `mode`; a background child is `activated` iff parent is activated at creation (`taskCreation.ts:27,37`). [V]
- `createBackgroundTask` always `activated:false`, `backgroundOptions` defaults `{expiresAfterMinutes:null, priority:'normal'}` (`taskCreation.ts:43-58`). [V]
- `sanitizeBackgroundOptions`: non-record → null; `expiresAfterMinutes` kept only if finite & >0, floored; priority `'high'` else `'normal'` (`taskCreation.ts:11-24`). [V]
- Deleting a task removes it and all descendants and strips their ids from every task's `triggers` (`taskMutations.ts:41-51, 86-91`). [V] Test: `taskStateController.test.ts:158-190`.
- Moving across main/background lists is refused (`taskTreeOperations.ts:73-78`); moving into own subtree refused (`:80-85`). [V]
- Migration of legacy records (`taskMigration.ts:9-35`): fills `children=[]`, `mode='main'`, `triggers=[]`, `activated=true`, `activatedAt=null`, `backgroundOptions=null` when absent; recursive; returns `true` if anything changed → triggers a save (`taskWorkflowController.ts:150-155`). [V]

### Progress [V] `taskProgress.ts:9-21`
`completed/total` count main tasks (all depths) + **activated** background tasks (all depths); each node counts 1 (parents count too). `percentage = completed/total*100` or 0.

### Undo/redo & snapshots [V] `taskStateController.ts`
- `MAX_UNDO_ACTIONS = 20` per stack (`:43`, shift oldest `:207-209`). `MAX_AUTO_SAVE_SNAPSHOTS = 5` (`:44`).
- Snapshot = deep clone via JSON of `{tasks, currentTemplate}` (`:212-217, 270-272`).
- Undo-labelled mutations: `'add background task'`, `'add task'`, `'complete next task'`, `'delete task'`, `'edit task'`, `'move task'`, `'toggle task'`, `'clear all tasks'`, plus externally captured `'update task triggers'` (`triggerController.ts:69`) and `'load template'` (`templateController.ts:157`). History pushed **only if** the mutation reports it changed something (`:174-185, 236-266`). [V]
- Any new mutation clears the redo stack (`:187-190`); test `taskStateController.test.ts:141-156`.
- Undo pushes current state onto redo, restores previous; redo pushes onto undo **without** clearing redo (`:146-172`).
- Every `saveTasks()` appends an auto-save snapshot `{createdAt, state}` and keeps last 5 (`:107-114, 224-232`); test `:192-208`.
- `clearTasks()` is a no-op (no history) when already empty and no template (`:137-144`).

### Templates [V] `types.ts:22-47`
- `TemplateTask {text; children?; mode?; backgroundOptions?; triggerIndices?}`; `TaskTemplate {id; name; tasks; createdAt; imported?; community?}`; `TemplateExportData` adds `version:'1.0'|'2.0'`, `exportedAt`, `description`, `taskCount`.
- **Trigger index semantics**: when saving tasks as a template, only *incomplete* tasks are included; a flat pre-order index (depth-first, incomplete only) is assigned to each included task; `triggers` ids are translated to `triggerIndices` into that flat order (`taskTemplates.ts:21-57`). On load, tasks are instantiated depth-first, flattened, and `triggerIndices[i]` → `flatLoaded[i].id` (`templateOperations.ts:97-110`). Indices that don't resolve are dropped. Normalisation dedupes and keeps only non-negative integers (`taskTemplates.ts:84-94`).
- Instantiation: fresh ids, `completed:false`, background mode inherited to children, background `activated:false`, main `activated:true` (`taskTemplates.ts:99-126`).
- Template text max 200 chars; template name max 50 chars, fallback `'Imported Template'` (`taskTemplates.ts:15-19, 145`).
- Template ids: `Date.now()` (`templateOperations.ts:35,44,54`); saved-template normalisation falls back to `createTaskId()` if id not finite (`taskTemplates.ts:168-174`); templates with 0 valid tasks are dropped on load (`:164-166`).
- Upsert is **by name** (replaces any existing template of same name, appends at end) (`templateOperations.ts:62-67`).
- Export version `'2.0'` iff any task (any depth) has `mode:'background'`, else `'1.0'` (`templateOperations.ts:79-90`). Export filename `poe2-template-<slug>.json`, slug = lowercase, non-alnum → `-`, trimmed, fallback `template` (`:92-95`).
- Import parsing (`taskTemplates.ts:193-215`): trim; empty → `'Template data is empty'`; >100000 chars → `'Template data too large'`; parsed with `destr(strict)`; must be object with string `name` and array `tasks` else `'Invalid JSON template format'`; after normalisation ≥1 task else `'Template must contain at least one valid task'`. Tasks may be plain strings or objects (`:59-62`).

### Settings [V] `types.ts:56-76`, `settingsDefaults.ts`, `settingsDomain.ts`
- `{settingsVersion:2, transparency:0-100 (default 70), theme:string (default 'echosight'), hotkeys:{toggleVisibility, toggleInteractive, completeNextTask, undoLastAction, redoLastAction}, sounds:{enabled:false, volume:60}}`.
- Default hotkeys: `<Mod>+Shift+T / I / N / Z / Y` where `<Mod>` = `Cmd` if `navigator.platform` matches mac/iphone/ipad/ipod else `Ctrl` (`settingsDomain.ts:14-20`, `hotkeyDisplay.ts:1-3,15-17`). Test `settingsDomain.test.ts:85-102`.
- Normalisation: percentages → `Number`, non-finite → fallback, clamp 0..100, `Math.round` (`:38-45`); legacy `backgroundColor` string used as theme if `theme` missing (`:94-106`); hotkeys/strings trimmed, empty → fallback (`:165-172`); `sounds.enabled` only if boolean (`:147-154`). Tests `settingsDomain.test.ts:23-83`.
- `shouldPersistSettingsMigration`: true if input not an object, version ≠ 2, has `backgroundColor`, or any normalised field differs (`:108-121`).

---

## 2. Composition root & startup (`renderer.ts`)

Startup sequence `initializeApp()` `renderer.ts:189-210` [V]:
1. `loadTasksForInitialization()` — on failure logs + alert `Could not read task data: <msg>\n\nSaving is disabled until this is fixed.` (title `Task Data Unavailable`, tone danger) and **continues** (`:212-222`). Test `rendererPersistence.test.ts:108-127`.
2. `migrateTaskStructure()` (saves if changed).
3. `loadTemplates()` → `loadTemplateState` (`:279-281`).
4. `themeController.loadThemes()` (out of scope).
5. `settingsController.loadSettings()`.
6. `templateController.updateTemplateSelect()`; `renderTasks()`; `taskDragController.setupContainerDropHandlers()`; `updateProgress()`; `overlayController.setupIpcListeners()`; `settingsController.applyTheme()`; `overlayController.refreshInteractiveVisuals()`; `restartExpirationTimers()`.
7. Any other error → alert `Error initializing app. Check console for details.`

`loadTasks()` `:254-264`: sets `taskPersistenceReady=false` until load resolves; if `corruptBackupPath` present → alert `Task data was corrupt and has been moved to:\n<path>\n\nEchosight started with an empty task list.` (title `Task Data Recovered`, danger). Test `rendererPersistence.test.ts:32-66`.

`saveTasks()` `:266-277`: if not ready → toast `Cannot save tasks until task data loads successfully.` and return false **without calling API**; if API save fails → toast `Failed to save tasks. Changes may not persist.` Tests `:68-145` (also: a re-load in flight blocks saving).

Shared "persist" sequence used everywhere after a mutation: `renderTasks(); updateProgress(); saveTasks()` (`renderer.ts:70-74, 109-113, 121-125`; `taskWorkflowController.ts:76-80`). **No debounce — every mutation saves immediately** [V].

`renderTasks()` `:229-239` re-renders both lists from scratch and re-attaches drag handlers. `showContextMenu` `:241-247`.

Error boundary: `window.echosight` missing → throws `'Echosight preload API is unavailable'` at module load (`:180-186`).

---

## 3. Static shell: DOM structure & ids (`index.html`)

- `#overlayContainer.overlay-container.interactive` (`:11`)
  - `#header.header` — `<h2>Echosight <span#shortcut-hint>(Interactive Mode)</span></h2>`; buttons `#settingsButton` "⚙" (title Settings), `#minimizeButton` "−" (title Hide overlay), `#closeButton.close` "×" (title Quit application) (`:13-20`).
  - `.template-section` — row 1: `<select#templateSelect>` (placeholder option `Select Template...`), `#loadTemplateButton` Load, `#showSaveTemplateButton` Save, `#exportTemplateButton` Export, `#showImportButton` Import, `#deleteTemplateButton.danger` Delete, `#clearTasksButton.danger` Clear All; row 2: `#communityTemplatesButton.success` Community Templates, `#interactiveToggle.success` Click-Through Mode (`:23-40`).
  - `.tasks-section` — `#taskInput` (placeholder `Add new task...`, maxlength 100) + `#addTaskButton` Add; `.progress-bar > #progressFill`; `#progressText` `0 / 0 tasks completed`; `#mainTasksLabel` "Main Tasks"; `<ul#taskList>`; `#backgroundTasksSection` (hidden unless `.has-active-background-tasks`) with label "Background Tasks" + `#backgroundCount`, `<ul#backgroundTaskList>` (`:43-70`).
- Modals (all `.modal`, shown via `.is-visible`): `#saveTemplateModal` (`#templateNameInput` maxlength 50, `#cancelSaveTemplateButton[data-modal-close]`, `#saveTemplateButton`); `#importTemplateModal` (`<textarea#importTemplateInput rows=8>`, hidden `<input type=file#importFileInput accept=.json>`, `#chooseImportFileButton` "Choose File", hint `JSON file, JSON text, or share code`, `#cancelImportButton`, `#importTemplateConfirmButton`); `#communityTemplatesModal` (`#communityTemplatesList`, `#closeCommunityButton`); `#settingsModal` (see §7); `#themeSelectionModal` (theme scope); `#addSubTaskModal` (`#subTaskInput` maxlength 100, `#closeSubTaskButton`, `#saveSubTaskButton` "Add"); `#configureTriggersModal` (description text, `#triggerTaskList`, `#newBgTaskInput` maxlength 100, `#addNewBgTaskButton` "+ Add", `#bgTaskHighPriority` checkbox "High Priority", `#closeTriggersButton`, `#saveTriggersButton`); `#taskEditModal` (`#taskEditInput` maxlength 100, `#taskEditBackgroundOptions[hidden]` containing `#taskEditHighPriority`, `#cancelTaskEditButton`, `#saveTaskEditButton`); `#appDialogModal[role=alertdialog]` (`#appDialogTitle`, `#appDialogMessage`, `#appDialogCancelButton`, `#appDialogConfirmButton`). (`:73-470`)

CSS structural facts [V]:
- `body` transparent, `user-select:none`, `overflow:hidden` (`styles.css:7-15`); inputs/textarea re-enable text selection (`:515-519`).
- `.overlay-container` = full-height flex column, 2px gold border (`:17-25`).
- **Click-through mode** (`.overlay-container.click-through`): header, header buttons, template section, task input, delete buttons, section labels, trigger indicators, right-click hint all hidden; checkboxes `pointer-events:none`; `.tasks-section::before` shows `var(--click-through-shortcut-hint)` on hover (`:26-118, 1352-1438`).
- Header `-webkit-app-region: drag` (window dragging) (`:128`), all controls `no-drag` (`:508-513`).
- `.tasks-section` scrolls (`overflow-y:auto`, `:233-237`).
- `.task-row` indents via `padding-left: var(--task-row-indent)` (`:304-309`); level 1 gets green left border and smaller/secondary text (`:1150-1176`); background items gold left border, `.high-priority` red pulsing border (`:1400-1413`).
- `.modal` fixed full-screen dim overlay, `display:none` → `flex` with `.is-visible` (`:459-476`); `.modal-content` max-width 300 (import 400, community 500 with scroll, settings/triggers/etc. wider) (`:478-577`).
- `.context-menu` fixed-position, items get ✎ / ➕ / ⚡ prefixes by nth-child (`:1212-1265`).
- Toasts fixed top-right: `.reorder-feedback` (green), `.bg-task-notification` (gold), `.themes-path-notification`; `slideIn` animation, `.toast-exit` reverses (`:1267-1308, 1441-1455`).
- Progress fill gradient, width transition (`:433-457`); `.progress-text.completed-feedback` green.

---

## 4. Feature area: Tasks

### 4.1 Task list rendering [V] `taskListRenderer.ts`, `taskListView.ts`
- Both lists cleared and rebuilt on every render (`taskListView.ts:15-23`). Main list = top-level tasks with `mode !== 'background'`; background list = top-level `mode==='background' && activated` (dormant background tasks are **not shown** in the overlay). Children rendered as flat sibling `<li>`s with increasing `data-level` (`taskListRenderer.ts:24-34`).
- Each `<li.task-item>` has `data-task-id`, `data-level`, `data-mode`, `draggable=true`, class `high-priority` if background & priority high (`:42-55`).
- Row contents in order: drag handle `⋮⋮` (title `Drag to reorder`), `📁` parent icon or spacer, checkbox (change → toggle), text span (`.completed` when done, `.parent-task-text` when has children; `contextmenu` → context menu), badges (`⚡N` trigger indicator with title `Triggers N background task(s)` → click opens Configure Triggers; `(done/total)` child count), hint `Right-click for options` (visible on hover only), delete button `×` (`:57-161`).
- Background section: `.has-active-background-tasks` toggled; `#backgroundCount` = `(completedActive/totalActive)` or empty (`taskListView.ts:26-40`).

### 4.2 Add task [V] `taskWorkflowController.ts:120-148`
- Triggered by `#addTaskButton` click or **Enter** in `#taskInput` (`staticControls.ts:78,119`).
- Reads trimmed input; empty → silently ignored. Appends new main task at end of top-level. Clears input. Persists. Undo label `'add task'`. On exception → alert `Error adding task. Please try again.`
- Focus/blur of `#taskInput` only logged (`staticControls.ts:128-134`).

### 4.3 Toggle completion [V] `taskCompletion.ts:75-99`, `taskWorkflowController.ts:205-225`
- Checkbox change flips `completed`. Completing a parent completes **all descendants**. Completing a child auto-completes its parent when all siblings complete (cascades upward, collecting `completedParents`). Unchecking a child un-completes every completed ancestor (`uncheckParent`). Children are NOT unchecked when parent is unchecked.
- After completion: activate triggers of the task and of each auto-completed parent; play `taskCompleted` sound. After un-completion: deactivate triggers of task + unchecked parents. Persist. Undo label `'toggle task'`.

### 4.4 Complete next task (global hotkey) [V] `taskCompletion.ts:61-73,101-121`, `taskWorkflowController.ts:94-118`
- Finds first incomplete **leaf** in pre-order among top-level non-background tasks (skips completed subtrees; background leaves never chosen). None → log only.
- Marks completed, cascades parent completion, activates triggers, persists, plays `taskCompleted`; if interactive mode: progress text shows `✓ Completed: <text>` (green) for 2000 ms then reverts (`taskProgressUi.ts:21-38`; tests `taskProgressUi.test.ts:22-46` — a re-render cancels the pending revert timer).

### 4.5 Delete task [V] `taskWorkflowController.ts:227-248`
- `×` button → confirm `Delete "<text>"? You can undo this with Ctrl+Shift+Z.` (title `Delete Task`, danger — the string is hard-coded "Ctrl+Shift+Z" regardless of configured hotkey/platform). Cancel → nothing (test `taskWorkflowController.test.ts:165-182`). Confirm → delete subtree, clear expiry timers for every removed id, strip trigger refs, persist. Undo label `'delete task'`.

### 4.6 Clear all [V] `taskWorkflowController.ts:250-266`
- `#clearTasksButton`. No-op when progress total is 0. Confirm `Clear all N task(s)? You can undo this with Ctrl+Shift+Z.` (title `Clear Tasks`, danger). Confirm → clear all expiry timers, replace with `[]` and `currentTemplate=null`, persist. Test `:184-206`.

### 4.7 Edit task (modal) [V] `taskEditUi.ts`, `taskWorkflowController.ts:161-199`
- Opened from context menu "Edit task" or from the "Edit" button on a row inside Configure Triggers (`backgroundTaskUi.ts:141-152`, `triggerController.ts:49`).
- Modal shows current text (focused + selected); `#taskEditBackgroundOptions` visible only for background tasks, with High Priority checkbox pre-set (`taskEditUi.ts:31-33`).
- Save (`#saveTaskEditButton` or Enter in `#taskEditInput`): empty/whitespace → alert `Please enter a task name!`, modal stays open, nothing changes. Trimmed text equal to current and priority unchanged → modal closes, no history, no save. Otherwise update text (and priority for background, preserving `expiresAfterMinutes`, `activated`, `activatedAt`, `triggers`, `completed`, children) → persist once; undo label `'edit task'`. Cancel/Escape discards. Tests `taskWorkflowController.test.ts:17-163`, `taskStateController.test.ts:11-113`, `rendererTaskEditing.test.ts` (end-to-end through real `index.html`, including editing an *active* background task and a *dormant* one opened on top of the Configure Triggers modal, where the trigger modal remains open underneath and its checkbox selection survives, `:133-170`).

### 4.8 Sub-tasks [V] `subtaskController.ts`, `subtaskUi.ts`
- Context menu "Add sub-task" → `#addSubTaskModal` (focus `#subTaskInput` after 150 ms, select text). Save via button or Enter. Empty → alert `Please enter a task name!`. Added as last child of the parent (inherits mode). Persist. Close clears input and re-focuses the window after 50 ms (`subtaskUi.ts:19-28`).

### 4.9 Context menu [V] `contextMenuUi.ts`
- Right-click on task text (`preventDefault`), menu `<div.context-menu>` appended to body at `pageX/pageY`; items: `Edit task`, `Add sub-task`, `Configure Triggers` (shown for all tasks incl. background). Any item click closes menu first. Closes on next document click (registered 10 ms later, `{once:true}`). Only one menu at a time.

### 4.10 Drag & drop reordering [V] `taskDragController.ts`, `dragDropUi.ts`, `taskTreeOperations.ts`
- Every `.task-item` is draggable; handlers for dragstart/dragend/dragover/drop/dragenter/dragleave are re-attached after each render (`taskDragController.ts:41-52`). Both `<ul>` containers also get `drop`/`dragover` handlers once (`:141-176`).
- dragstart: remember id/element, add `.is-dragging`, create initial placeholder (`Drop here to reorder`), `effectAllowed='move'`, `setData('text/html', outerHTML)` (`:54-70`).
- dragover (item or container): compute **drop intent** from pointer: `isAbove = clientY < vertical midpoint`; `isIndented = clientX > horizontal midpoint` of target row (`dragDropUi.ts:12-24`). Container-level dragover snaps to nearest `.task-item` within 50 px vertical distance; container drop uses unlimited distance (`taskDragController.ts:142-143,166`).
- Operation classification (`taskTreeOperations.ts:24-54,157-186`): null if dragging onto itself or into own subtree. `makeSubtask = isIndented && !isAbove && draggedLevel <= targetLevel`. Types/colours: `Make subtask of target` (#4285f4 blue), `Reorder within same level` (#d4af37 gold), `Promote to higher level` (#32cd32 green, dragged deeper than target), `Reorder as sibling` (gold).
- Placeholder `<li.drag-placeholder>` re-rendered only when operation key `type-isAbove-targetId` changes; text = operation type or `Drop to make subtask` (indented `(targetLevel+1)*20px`); inserted above/below target, or below target for subtask (`dragDropUi.ts:34-59, 96-134`).
- dragenter/leave toggle `.is-drag-hover` on non-dragged items.
- drop → `reorderTasksAdvanced` (`taskWorkflowController.ts:268-286`): `moveTaskInTree` — subtask: append to target's children; else insert in target's sibling array at targetIndex (above) or +1 (below). Refused across main/background (`'Cannot reorder across main/background lists'`). On success toast with message: `Made "A" a subtask of "B"` / `Reordered "A" within same level` / `Promoted "A" from level X to level Y` / `Moved "A" to be sibling of "B"` (`taskTreeOperations.ts:141-155`), re-render + save (note: `updateProgress` is **not** called after a move). Undo label `'move task'`.
- dragend clears styles/placeholder/state.

### 4.11 Background tasks, triggers, expiry, priority [V]
- Creation: only via Configure Triggers modal "+ Add" (`triggerController.ts:76-91`): empty text → alert `Please enter a background task description.`; else `addBackgroundTask(text, {priority})` (always top-level, dormant, `expiresAfterMinutes:null` — **no UI sets an expiry**; expiry only arrives via imported/legacy data) → `onChanged` (render/progress/save) → modal list re-rendered for the same configuring task. Undo label `'add background task'`.
- Configure Triggers modal (`backgroundTaskUi.ts:24-66,107-175`): lists **all** background tasks (any depth, flattened); each row: checkbox (checked if in `task.triggers`, `data-bg-task-id`), text, `HIGH` badge if high priority, `ACTIVE`/`DORMANT` badge, `Edit` button (aria-label `Edit <text>`). Empty list → `No background tasks yet. Create one below.` Focus → `#newBgTaskInput` (cleared, not selected; test `backgroundTaskUi.test.ts:24-36`).
- Save (`triggerController.ts:57-74`): new ids = checked checkboxes in DOM order; if differs from current `triggers` → capture undo `'update task triggers'`, assign, `onChanged`. Close resets state. Enter in `#newBgTaskInput` adds a background task (not save) (`staticControls.ts:123`). Test `triggerController.test.ts:17-66`.
- Activation (`backgroundTaskController.ts:31-50`): on completion of a task with triggers, each referenced **background, not yet activated** task gets `activated=true`, `activatedAt=now`, timer started; count>0 → toast `N background task(s) activated!` (`backgroundTaskUi.ts:10-22`, 3 s then exit animation 300 ms) + `backgroundActivated` sound (`renderer.ts:75-78`).
- Deactivation (`:52-65`): when a triggering task is un-completed, referenced background tasks that are activated and **not completed** become dormant (`activated=false`, `activatedAt=null`, timer cleared). Completed background tasks remain visible.
- Expiry (`:67-96,121-133`): only when `backgroundOptions.expiresAfterMinutes` truthy. Delay = minutes·60000 − (now − activatedAt) (or full duration if no/invalid `activatedAt`). If ≤0 → immediately deactivate + `onChanged`. Else `setTimeout`; on fire, if still activated & not completed → `activated=false`, `onChanged`, log `Background task "<text>" expired`. `restartExpirationTimers()` re-arms timers for all activated, incomplete background tasks (called at startup, after undo/redo). Timers cleared on delete (per id), clear-all, template load, undo/redo (then restarted).
- Priority: `'high'` adds `.high-priority` class on the overlay row and `HIGH` badge in trigger modal; set at creation (checkbox in trigger modal) or via Edit Task modal.

### 4.12 Undo / Redo (hotkeys) [V] `taskWorkflowController.ts:288-314`
- Invoked from IPC `onUndoLastTaskAction` / `onRedoLastTaskAction` (global hotkeys handled by main process) via `overlayController.ts:48-56`. No in-renderer keyboard shortcut.
- Nothing to undo/redo → log only. Else restore state, clear + restart all expiry timers, persist, play `undo`/`redo` sound. Test `taskWorkflowController.test.ts:208-232`.

---

## 5. Feature area: Templates

### 5.1 Template `<select>` [V] `templateUi.ts:59-77`
Options: placeholder `Select Template...` (value `''`) then `"<name> (<N> tasks)"` with value = id, N = flattened template task count (`templateOperations.ts:112-121`). Rebuilt after load and after every successful collection save (`templateController.ts:73-81,100-102`).

### 5.2 Save template [V] `templateController.ts:104-136`
- `#showSaveTemplateButton`: no tasks → alert `No tasks to save as template!`; else open `#saveTemplateModal` (focus+select `#templateNameInput`).
- Save (button or Enter): empty name → `Please enter a template name!`; no tasks → `No tasks to save!`; else build template from **incomplete** tasks (see §1), upsert by name, `api.saveTemplates(all)`; failure → `Failed to save template. Please try again.`; success → close modal (clears input) + alert `Template "<name>" saved successfully!`.

### 5.3 Load template [V] `templateController.ts:138-162`
- `#loadTemplateButton`. No selection / unknown id → silently return. If any current task is incomplete → confirm `This will replace your current tasks. Continue?` (title `Load Template`, danger). Then: clear all expiry timers, capture undo `'load template'`, replace tasks with instantiated template (fresh ids, triggers remapped), set `currentTemplate = template.name`, run migration, render + progress, `saveTasks()`, focus window after 200 ms.

### 5.4 Delete template [V] `templateController.ts:164-191`
- `#deleteTemplateButton`. No selection → alert `Please select a template to delete!`. Confirm `Are you sure you want to delete the template "<name>"? This cannot be undone.` (title `Delete Template`, danger). Save collection; failure → `Failed to delete template "<name>". Please try again.`; success → `Template "<name>" deleted successfully!`.

### 5.5 Export template [V] `templateTransferController.ts:58-79`, `templateUi.ts:79-89`
- `#exportTemplateButton`. No selection → `Please select a template to export!`. Else JSON (2-space) `{name, tasks, version, exportedAt, description:"PoE Task Template: <name>", taskCount}` downloaded via Blob + object URL + synthetic `<a download>` click; alert `Template "<name>" exported successfully!`. Errors → `Error exporting template. Please try again.`

### 5.6 Import template [V] `templateTransferController.ts:81-131`, `templateUi.ts:29-53,91-113`
- `#showImportButton` → `#importTemplateModal` (textarea placeholder reset to `Paste template JSON or share code here...`, focused).
- "Choose File" → programmatic click on hidden `<input type=file accept=.json>`; on `change` the first file is read with `FileReader.readAsText` and its text written into the textarea; errors → `Error reading file. Please try again.`
- Import (`#importTemplateConfirmButton` or **Ctrl/Cmd+Enter** in textarea, `staticControls.ts:120`): empty → `Please paste template JSON, share code, or select a file!`. Parse (§1 rules); name collision → confirm `Template "<name>" already exists. Replace it?` (default tone/title `Confirm Action`). Save; failure → `Failed to import template "<name>". Please try again.`; success → close modal (clear textarea + file input) + alert `Template "<name>" imported successfully! (<N> tasks)`. Parse/other errors → `Invalid template format: <message>`.
- "share code" in copy is **just JSON text** — no separate encoding exists in source [V].

### 5.7 Community templates [V] `communityTemplates.ts`, `communityTemplateController.ts`, `communityTemplatesUi.ts`
- `#communityTemplatesButton` → `#communityTemplatesModal` with one card per template: `<h4>name`, description, `"N tasks"`, buttons `Add to My Templates` (primary) and `Preview Tasks` (secondary). Errors → `Error loading community templates. Please try again.`
- Add: name collision → confirm `Template "<name>" already exists. Replace it?`; save (`community:true`); failure → `Failed to import community template "<name>". Please try again.`; success → `Community template "<name>" added to your templates!` then focus window after 200 ms. Modal stays open.
- Preview: alert with text `"<name> Tasks:\n\n1. …\n2. …"`.
- **The task said "10 community templates"; the source defines exactly 6** (`communityTemplates.ts:9-95`) [V]:
  1. League Start Essentials — 8 tasks
  2. Endgame Progression — 9 tasks
  3. New Character Setup — 8 tasks
  4. Currency Goals — 8 tasks
  5. HC/SSF Priorities — 8 tasks
  6. Crafting Checklist — 8 tasks
  Full task texts are in `communityTemplates.ts:13-22, 27-37, 42-51, 56-65, 71-79, 84-93`. All are flat (no children, no background, no triggers) (`:97-102`).

---

## 6. Feature area: Overlay / window commands / interactive mode

- `#settingsButton` → settings modal; `#minimizeButton` → `api.minimizeOverlay()`; `#closeButton` → confirm `Close Echosight Overlay completely?` (title `Quit Echosight`, danger) → `api.quitApplication()` (`windowCommands.ts:28-53`).
- `#interactiveToggle` → `api.toggleInteractiveMode()`; actual state arrives back via `onInteractiveModeChanged` (`overlayController.ts:37-41`). Renderer starts assuming interactive=true (`:29`).
- `renderInteractiveMode` (`overlayUi.ts:5-48`): sets CSS var `--click-through-shortcut-hint` = `"<hotkey> to edit"`; interactive → container classes `interactive` (+`glass-mode` iff theme id `glass`), hint text `(Interactive Mode - <hotkey> to exit)`, toggle button text `Interactive Mode` + `.interactive-toggle-active`; click-through → classes `click-through`, button text `Click-Through Mode` + `.interactive-toggle-inactive`. Hotkey displayed through `formatHotkeyForDisplay` (CommandOrControl/Command/Cmd/Ctrl → platform label).
- Header made a native drag region via inline `-webkit-app-region: drag`, controls `no-drag` (`overlayUi.ts:50-60`).
- Global `error` / `unhandledrejection` listeners log to console (`:62-72`).
- Reset position (settings): `api.resetWindowPosition()` + alert `Window position reset! The overlay will move to the top-right corner.` (`windowCommands.ts:15-26`).
- IPC inbound: `onCompleteNextTask`, `onUndoLastTaskAction`, `onRedoLastTaskAction`, `onInteractiveModeChanged` (`overlayController.ts:36-57`). `api.toggleOverlay` and `api.getThemesPath` are **not used** by non-theme renderer code [V].

---

## 7. Feature area: Settings

### 7.1 Settings modal UI [V] `index.html:116-258`, `settingsUi.ts`
Sections: **Appearance** — `#transparencySlider` (range 0-100) with labels `More Transparent | <N>% visible (#transparencyValue) | More Solid`; Theme summary (`#themeSelect` hidden input, `#selectedThemeName`, `#selectedThemeId`, buttons `Choose Theme`, `Open Folder`, `Reload`); **Theme Sounds** — `#themeSoundsEnabled` checkbox "Enable optional theme sounds", `#themeSoundVolumeSlider` with `Muted | <N>% volume (#themeSoundVolumeValue) | Louder`. **Hotkeys** — 5 rows (labels `Toggle Overlay Visibility`, `Toggle Interactive Mode`, `Complete Next Task`, `Undo Last Task Action`, `Forward Last Task Action`), each a readonly input (`#toggleVisibilityHotkey` … `#redoLastActionHotkey`, placeholder `Click Record and press keys`) + `Record` button (`#recordBtn1`..`#recordBtn5`); help text + `Reset to Defaults` (`#resetHotkeysButton`). **Position** — `Reset to Top-Right` (`#resetPositionButton`). Footer `Cancel` (`#closeSettingsButton`), `Save Settings` (`#saveSettingsButton`).

Opening (`settingsController.ts:127-129`, `settingsUi.ts:32-57`): show modal (focus first focusable), write all controls from current settings (hotkeys formatted for display). Test `settingsUi.test.ts:24-43`.

### 7.2 Live behaviours while modal open [V]
- Transparency slider `input` event → normalise, update label, set `settings.transparency`, set CSS var `--user-transparency` (= value/100) on `<html>`, re-apply theme (`settingsController.ts:148-159`). Applied immediately (not waiting for Save).
- When applied theme reports `supportsTransparency === false` → slider disabled + `.transparency-control-disabled` on slider/label/value, value text `Not supported by this theme`; else re-enabled with `<N>% visible` (`settingsUi.ts:97-117`; test `settingsUi.test.ts:84-98`).
- Sound checkbox `change` → `settings.sounds.enabled`, notify theme-sounds immediately (`:161-170`). Volume slider `input` → normalise, label `<N>% volume`, notify (`:172-183`).
- Theme changes come from the theme-selection modal `onApply` → `updateTheme` (`:185-197`): set, re-render summary, apply, refresh interactive visuals if interactive. (`bindValueChange('themeSelect')` also wired but the input is hidden, `staticControls.ts:116`.)

### 7.3 Save / Cancel [V] `settingsController.ts:241-259`, `settingsPersistence.ts:38-54`
- Save: stop any hotkey recording; read every control (raw strings) → `normalizeSettings` against current; notify sounds; `api.saveSettings(settings)`; on failure throws → alert `Error saving settings: <msg>. Please try again.`; on success `api.updateHotkeys(settings.hotkeys)`, close modal, alert `Settings saved successfully! Hotkey changes are active now.`
- Cancel/Escape: stop recording, hide modal — **but live-applied transparency/sound/theme changes are NOT reverted** (in-memory `settings` already mutated) [V `:135-138,148-197`].

### 7.4 Hotkey recording [V] `hotkeyRecorder.ts`, `settingsDomain.ts:60-92,174-198`
- Click `Record` → input shows `Press keys now...` + `.hotkey-recording-input`, button text `Cancel` + `.hotkey-recording-button`, input focused; `api.setHotkeyRecording(true)` (main suspends global shortcuts, inferred). Clicking the same button again cancels and restores previous value; clicking another Record button stops the first (restoring) and starts the new one. Clicking anywhere outside `.modal-content` while recording stops it (`staticControls.ts:136-140`).
- Keydown captured at `window` capture phase (`staticControls.ts:126`); while recording: `preventDefault`/`stopPropagation`. Pure modifier keys ignored. Main key normalised: `' '`→`Space`, arrows → `Up/Down/Left/Right`, single chars uppercased. Combo = `[Mod][Alt][Shift]+Key` joined by `+` (Ctrl or Meta both map to platform primary label). Accepted iff it has Mod, Alt, or main key is `F1`–`F12`. If ≥2 keys but no required modifier (e.g. Shift+I) → alert `Please use <Mod>+<Key>, Alt+<Key>, or a function key to avoid conflicts with normal typing.` and keep recording. Bare single key → ignored.
- Duplicate check against other actions (case-insensitive after display formatting) → alert `"<combo>" is already assigned to <Action Label>. Choose a different shortcut.`, keep recording.
- Accepted → write into input, update in-memory `settings.hotkeys[action]` (`settingsController.ts:84-92`), stop after 200 ms without restoring; `api.setHotkeyRecording(false)`. Tests `hotkeyRecorder.test.ts:17-133`.
- Reset to Defaults (`settingsController.ts:102-125`): confirm `Reset hotkeys to default values (<Mod>+Shift+T, <Mod>+Shift+I, <Mod>+Shift+N, <Mod>+Shift+Z, <Mod>+Shift+Y)?` (title `Reset Hotkeys`, danger) → write defaults into inputs + memory, alert `Hotkeys reset to defaults. Click "Save Settings" to apply.`

### 7.5 Settings load [V] `settingsPersistence.ts:7-36`
`api.loadSettings()` → normalise; if migration needed → `api.saveSettings(normalised)` immediately (error only logged); on exception → return in-memory defaults. Tests `settingsPersistence.test.ts:9-45`.

---

## 8. Shared UI infrastructure

### 8.1 Dialog service (alert/confirm) [V] `dialogService.ts`
- Single `#appDialogModal`; requests are **queued** so one is visible at a time (`:43-68`; test `dialogService.test.ts:153-168`).
- Alert: title default `Echosight`, cancel button hidden+disabled, confirm `OK`, role `alertdialog`, resolves `void` on OK/Escape/Enter. Confirm: title default `Confirm Action`, both buttons, resolves boolean; role `dialog` unless tone `danger` (`alertdialog`).
- Tones `info|danger|success` → `app-dialog-tone-*` on content and `app-dialog-confirm-*` on OK button. Custom `title`, `confirmLabel`, `cancelLabel`, `initialFocus`, `confirmOnEnter`.
- Initial focus: alert → OK; confirm → explicit `initialFocus`, else Cancel for danger, OK otherwise (`:178-198`).
- Keys (document capture): Escape → cancel (false for confirm). Enter → accept only if `confirmOnEnter` (default: **false for danger confirms**, true otherwise) and the event target is not a button/input/select/textarea (so Enter on focused Cancel button does the native click = cancel) (`:102-119, 200-214`). Tests `:79-151`.
- If the dialog DOM is missing: alert resolves immediately, confirm resolves `false` (`:72-75`).

### 8.2 Generic modal helper [V] `modalUi.ts`
- `showModal(id, opts)`: add `.is-visible`, `aria-hidden=false`, `aria-modal=true`, default `role=dialog`, `tabindex=-1`; push onto modal stack; install capture-phase keydown handler.
- Focus scheduling: for each retry delay in `[0,75,200,400]` ms (+`focusDelayMs`), call `focusWindow()` then after +50 ms focus the target (`focusSelector` if focusable, else first focusable, else modal itself); `selectText` selects input content. Skipped if modal already hidden (`:73-103`); tests `modalUi.test.ts:39-76,185-202`.
- **Escape** (only for topmost visible modal, `closeOnEscape` default true): click the first focusable element matching `[data-modal-close], button[id*=cancel|Cancel|close|Close]`, else hide (`:164-244`). Tests `:98-163`.
- **Tab/Shift+Tab** trapped within modal (wraps; if focus outside modal, moves to first/last) (`:179-211`; test `:78-96`).
- Hide: remove class/aria, pop stack, restore focus to the previously focused element if still in DOM and not inside a hidden modal (`:62-71,145-162,246-260`; test `:165-183`).
- Multiple modals can stack (Edit Task over Configure Triggers) — Escape/Tab apply to the top one only.

### 8.3 Toasts [V] `feedbackUi.ts`, `backgroundTaskUi.ts:10-22`
`showReorderFeedback(msg='Task order updated!')` and `showBackgroundTaskNotification(n)`: appended to body, 3000 ms then `.toast-exit` 300 ms then removed. Multiple toasts can overlap at the same fixed position [I]. `showThemesPathNotification` (8 s) exists but has no caller in renderer.

### 8.4 DOM binding helpers [V] `domEvents.ts`
All handlers wrapped in `runSafely` (sync throw / async rejection → `console.error('Unhandled DOM event handler error:')`). `bindEnterKey` fires on `keydown` `Enter` (optionally requiring Ctrl/Meta). Missing elements are silently skipped.

### 8.5 Window focus [V] `windowFocus.ts`
`focusOverlayNow`: `window.focus()` then `api.focusWindow()` (IPC), both try/catch. `focusOverlaySoon(api, 200ms default)`.

---

## 9. Persistence contract (exact `window.echosight` usage)

| API | Called from | When | Payload / Result |
|---|---|---|---|
| `loadTasks(): Promise<TaskLoadData>` | `taskPersistence.ts:18` via `taskStateController.loadTasks` via `renderer.loadTasks` | Startup only | Result normalised: `tasks` (array or `[]`), `currentTemplate` (string or null), `snapshots` (last 5), optional `corruptBackupPath`. Rejection is rethrown (saving stays disabled). |
| `saveTasks(data): Promise<SaveResult>` | `taskPersistence.ts:32` via `taskStateController.saveTasks` via `renderer.saveTasks` | **Immediately after every mutation** (add/toggle/complete-next/delete/clear/edit/move/undo/redo/trigger change/background add/expiry/activation/template load/migration). No debounce. | `{ tasks: Task[], currentTemplate: string\|null, snapshots: TaskSnapshot[] (≤5, each {createdAt, state:{tasks,currentTemplate}}) }`. `success:false` → toast. |
| `loadTemplates(): Promise<unknown>` | `templatePersistence.ts:9` | Startup | Normalised via `normalizeTemplateCollection`; failure → `[]`. |
| `saveTemplates(TaskTemplate[])` | `templatePersistence.ts:22` | After save/delete/import/community-add | Full replacement array. |
| `loadSettings(): Promise<Partial<Settings>\|null>` | `settingsPersistence.ts:14` | Startup | Normalised; migration may trigger immediate `saveSettings`. |
| `saveSettings(Settings)` | `settingsPersistence.ts:24,45` | Startup migration; Save Settings button | Full `Settings` object. |
| `updateHotkeys(HotkeySettings)` | `settingsPersistence.ts:53` | After successful saveSettings | Pushes new global shortcuts to main. |
| `setHotkeyRecording(bool)` | `settingsController.ts:94` | Start/stop of hotkey recording | — |
| `focusWindow()` | `windowFocus.ts:17` | Modal open retries, after sub-task close, template load, community add | — |
| `resetWindowPosition()`, `minimizeOverlay()`, `toggleInteractiveMode()`, `quitApplication()` | `windowCommands.ts` | Buttons | — |
| `onInteractiveModeChanged`, `onCompleteNextTask`, `onUndoLastTaskAction`, `onRedoLastTaskAction` | `overlayController.ts:37-56` | Registered once at startup (unsubscribe fns ignored) | Push events from main. |
| Theme APIs (`loadThemes`, `getTheme`, `reloadThemes`, `openThemesFolder`, `getThemesPath`, `loadThemeCss`, `getThemeAsset`) | themes scope + `settingsController.applyTheme` | — | Out of scope. |

No `localStorage`/`sessionStorage`/IndexedDB usage anywhere in renderer [V grep].

---

## 10. Browser/DOM/Electron-renderer dependencies with no direct non-browser equivalent (flag list)

| # | API / mechanism | Where | Re-implementation note |
|---|---|---|---|
| 1 | **HTML5 Drag & Drop** (`draggable`, dragstart/over/enter/leave/drop/end, `DataTransfer.setData/effectAllowed/dropEffect`, `getBoundingClientRect`, `clientX/Y`, `offsetHeight`) | `taskDragController.ts`, `dragDropUi.ts` | Need pointer-based drag with midpoint/half-width intent and nearest-row (≤50 px) snapping; placeholder row insertion. |
| 2 | **Blob + `URL.createObjectURL` + synthetic `<a download>` click** | `templateUi.ts:79-89` | Replace with native save-file dialog. |
| 3 | **`<input type=file>` + `FileReader.readAsText`** | `templateUi.ts:91-113`, `index.html:92` | Replace with native open-file dialog. |
| 4 | **`contextmenu` event + absolutely positioned DOM menu at `pageX/pageY`**, close on next document click | `contextMenuUi.ts` | Native/GPUI context menu. |
| 5 | **`setTimeout`-based scheduling**: modal focus retries `[0,75,200,400]+50 ms`, sub-task focus delay 150 ms, re-focus 50/200 ms, toast 3000+300 ms, completion feedback 2000 ms, recorder stop 200 ms, context-menu close listener 10 ms, expiry timers (minutes) | `modalUi.ts`, `subtaskUi.ts`, `windowFocus.ts`, `feedbackUi.ts`, `backgroundTaskUi.ts`, `taskProgressUi.ts`, `hotkeyRecorder.ts`, `contextMenuUi.ts`, `backgroundTaskController.ts` | Timers must survive re-render; expiry timers keyed by task id. |
| 6 | **`window.focus()`** + IPC `focusWindow` | `windowFocus.ts`, `subtaskUi.ts:26` | Window activation. |
| 7 | **CSS custom properties set from JS**: `--user-transparency`, `--click-through-shortcut-hint` (string content for `::before`), `--task-row-indent`, `--drag-placeholder-height/-color/-indent`; theme vars `--text-primary/-secondary/-muted`, `--border-interactive` consumed by `styles.css` | `settingsController.ts:157`, `overlayUi.ts:14-17`, `taskListRenderer.ts:60`, `dragDropUi.ts` | Map to style tokens. |
| 8 | **`-webkit-app-region: drag/no-drag`** inline + CSS for frameless-window dragging by header | `overlayUi.ts:50-60`, `styles.css:14,128,152,164,471,485,512` | Native window-move on header drag. |
| 9 | **`document.activeElement`, `.focus()`, `.select()`, Tab focus trap, `aria-*`/`role`** | `modalUi.ts`, `dialogService.ts` | Focus management must be re-created explicitly. |
| 10 | **Global capture-phase `keydown` listeners** on `document`/`window` (modal Escape/Tab, dialog Enter/Escape, hotkey recorder) | `modalUi.ts:142`, `dialogService.ts:123`, `staticControls.ts:126` | Key routing with priority: topmost modal first. |
| 11 | **`navigator.platform`** sniffing for Cmd vs Ctrl labels | `hotkeyDisplay.ts:15-17` | Use OS detection. |
| 12 | **`KeyboardEvent.key/ctrlKey/metaKey/altKey/shiftKey`** semantics incl. `'ArrowUp'`, `' '`, single-char keys | `settingsDomain.ts:60-92,174-198` | Map GPUI key events to the same combo string format (`Ctrl+Shift+Y`, `F8`, `Alt+Space`). |
| 13 | **`<select>`** element with dynamic `<option>`s; `<input type=range>`; `<input type=checkbox>`; `<textarea>`; `maxlength` attributes (100 task text, 50 template name) | `index.html`, `templateUi.ts`, `settingsUi.ts` | Native widgets; enforce maxlength manually. |
| 14 | **`element.hidden`, `classList` toggles driving visibility/animation** (`.is-visible`, `.has-active-background-tasks`, `.high-priority`, `.completed-feedback`, `.is-dragging`, `.is-drag-hover`, `.toast-exit`, recording classes) | throughout | Convert to view-state booleans. |
| 15 | **CSS animations/transitions**: `slideIn` toasts, `placeholderPulse`, `pulse` folder icon, `pulseBorder` high-priority, progress width transition, hover reveals (drag handle, right-click hint, click-through hint) | `styles.css` | Optional fidelity. |
| 16 | **`JSON.parse(JSON.stringify())`** deep-clone for snapshots; **`destr`** npm parser for imports | `taskStateController.ts:270-272`, `taskTemplates.ts:1,204` | Any deep clone / strict JSON parser. |
| 17 | **`window.error` / `unhandledrejection`** logging, `console.*` logging via injected `logger` | `overlayUi.ts:62-72` | Logging sink. |
| 18 | **Electron preload bridge `window.echosight`** (IPC) | everywhere | Replace with direct Rust services; keep the contract in §9. |
| 19 | `<audio>` / `new Audio` — **not used in non-theme scope** (sound playback lives in `features/themes/themeSounds.ts`) [V grep]. `contenteditable` — only appears in the focusable selector list (`modalUi.ts:12`); no editable regions exist [V]. Clipboard API — **not used**; "share code" import is paste-into-textarea only [V]. | — | — |

---

## 11. Complete user-visible string catalogue (non-theme)

Alerts/confirms: `Error initializing app. Check console for details.` · `Could not read task data: … Saving is disabled until this is fixed.` · `Task data was corrupt and has been moved to: … Echosight started with an empty task list.` · `Error adding task. Please try again.` · `Please enter a task name!` · `Delete "<t>"? You can undo this with Ctrl+Shift+Z.` · `Clear all N task(s)? You can undo this with Ctrl+Shift+Z.` · `Please enter a background task description.` · `No tasks to save as template!` · `Please enter a template name!` · `No tasks to save!` · `Failed to save template. Please try again.` · `Template "<n>" saved successfully!` · `This will replace your current tasks. Continue?` · `Please select a template to delete!` · `Are you sure you want to delete the template "<n>"? This cannot be undone.` · `Failed to delete template "<n>". Please try again.` · `Template "<n>" deleted successfully!` · `Please select a template to export!` · `Template "<n>" exported successfully!` · `Error exporting template. Please try again.` · `Error reading file. Please try again.` · `Please paste template JSON, share code, or select a file!` · `Template "<n>" already exists. Replace it?` · `Failed to import template "<n>". Please try again.` · `Template "<n>" imported successfully! (N tasks)` · `Invalid template format: <msg>` · `Error loading community templates. Please try again.` · `Failed to import community template "<n>". Please try again.` · `Community template "<n>" added to your templates!` · `Error importing template. Please try again.` · `Error previewing template.` · `<n> Tasks:\n\n1. …` · `Reset hotkeys to default values (…)?` · `Hotkeys reset to defaults. Click "Save Settings" to apply.` · `Settings saved successfully! Hotkey changes are active now.` · `Error saving settings: <msg>. Please try again.` · `Please use <Mod>+<K>, Alt+<K>, or a function key to avoid conflicts with normal typing.` · `"<combo>" is already assigned to <Action>. Choose a different shortcut.` · `Window position reset! The overlay will move to the top-right corner.` · `Close Echosight Overlay completely?`

Dialog titles: `Echosight`, `Confirm Action`, `Task Data Unavailable`, `Task Data Recovered`, `Delete Task`, `Clear Tasks`, `Load Template`, `Delete Template`, `Reset Hotkeys`, `Quit Echosight`.

Toasts: `Cannot save tasks until task data loads successfully.` · `Failed to save tasks. Changes may not persist.` · `Task order updated!` (default, unused in practice) · move descriptions (§4.10) · `N background task(s) activated!` · `✓ Completed: <t>` (progress text).

Labels/placeholders: `Select Template...` · `Add new task...` · `0 / 0 tasks completed` / `N / M tasks completed` · `Main Tasks` · `Background Tasks` · `Drag to reorder` · `Right-click for options` · `Triggers N background task(s)` · `⚡N` · `(done/total)` · `📁` · `×` · `⋮⋮` · `Drop here to reorder` · `Drop to make subtask` · `Make subtask of target` · `Reorder within same level` · `Promote to higher level` · `Reorder as sibling` · `No background tasks yet. Create one below.` · `HIGH` / `ACTIVE` / `DORMANT` · `Edit` · `Press keys now...` · `Record` / `Cancel` · `<N>% visible` · `Not supported by this theme` · `<N>% volume` · `(Interactive Mode - <hk> to exit)` · `<hk> to edit` · `Interactive Mode` / `Click-Through Mode` · `Add to My Templates` / `Preview Tasks` · `N tasks`.

---

## 12. Test-asserted behaviours (checklist for parity)

tasks/taskStateController: in-place nested edit preserves all other fields; background edit changes text+priority only, undo/redo restore exactly with label `edit task`; empty/missing/no-op edits return false and push no history; toggle undo/redo with label `toggle task`; new mutation clears redo; delete strips triggers and undo restores them; save keeps exactly 5 snapshots and `currentTemplate`; `corruptBackupPath` passthrough; save failure → false.

tasks/taskWorkflowController: edit modal pre-fills & selects text, hides bg options for main; persists once (render/progress/save each once); bg edit preserves `activated/activatedAt/expiresAfterMinutes`; empty text alert keeps modal open; cancel discards; no-op closes silently; delete cancel untouched with exact confirm text; clear-all exact confirm text, clears timers, persists; undo/redo clear+restart timers, 3 saves, sounds `taskCompleted/undo/redo`.

tasks/rendererPersistence: single startup alert with backup path & title; save-failure toast text; load rejection blocks saves with toast text and no API call; init load failure alerts with title/tone and continues; reload in flight blocks saves.

tasks/rendererTaskEditing (full DOM): context-menu edit flow, Escape cancels, save trims and persists exact task fields; active background edit updates `.high-priority` class; dormant background edited from trigger modal with both modals visible (edit modal after trigger modal in DOM), checkbox selection preserved, saving triggers yields `[20, 30]`.

tasks/triggerController, backgroundTaskUi, taskProgressUi, taskPersistence: trigger modal row checkbox/Edit button, saving unchecked clears triggers & calls onChanged once; opening focuses `#newBgTaskInput` without selection and calls focusWindow once; progress width/text and 2 s feedback; feedback timer cancelled by re-render; load errors logged with `'Failed to load tasks:'` and rethrown.

settings/*: defaults (theme `echosight`, transparency 70, version 2, sounds off/60); legacy migration (string transparency rounded, `backgroundColor`→theme, missing hotkeys filled, including redo); sound normalisation (`'yes'`→false, 500→100); mac → `Cmd` labels; load persists migrated settings exactly once and not when unchanged; settings modal renders all controls & aria state; reads raw control values; `CommandOrControl` → platform label; transparency disable/enable visuals and texts; recorder: visual states, `Ctrl+Shift+Y` from ctrl+shift+y, cancel restores, switching buttons, duplicate rejection message, `F8` accepted alone, unknown action ignored, `onRecordingChanged` sequence `[true,false]`.

ui/dialogService & modalUi: as enumerated in §8.1–8.2.

---

## 13. Notable gaps / quirks to preserve or consciously fix (all [V])

1. Confirm texts hard-code `Ctrl+Shift+Z` even on macOS or with custom undo hotkey (`taskWorkflowController.ts:233,256`).
2. Settings "Cancel" does not revert live-applied transparency/sound/theme changes.
3. After drag reorder, `reorderTasksAdvanced` calls `renderTasks` + `saveTasks` but not `updateProgress` (`taskWorkflowController.ts:284-285`); harmless today because moves never change counts.
4. No UI to set `expiresAfterMinutes`; expiry only via imported/legacy data or templates with `backgroundOptions`.
5. Dormant background tasks are invisible in the overlay; only reachable via Configure Triggers modal (where they can be edited).
6. Community template count is 6, not 10.
7. `showThemesPathNotification` has no callers (dead code).
8. Template upsert is by name; import/community replace existing same-name templates after confirm, but **Save Template silently overwrites** a same-name template without confirmation (`templateController.ts:127-129`).
9. Template export filename prefix is `poe2-template-` while descriptions say "PoE Task Template".
10. Task ids rely on `Date.now()*1000 + random` — collisions theoretically possible within the same millisecond; template ids use bare `Date.now()`.
