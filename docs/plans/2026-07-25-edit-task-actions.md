# Edit Tasks and Background Tasks Implementation Plan

> **For Hermes:** Implement these tasks in order with a RED test before each GREEN change. Keep edits ID-based and in place; do not replace task objects or add a new state model.

**Goal:** Let users edit a normal, nested, or currently active background task from the existing task right-click menu, with undo/redo, rerendering, and persistence behaving like existing task mutations.

**Architecture:** Reuse the existing context-menu callback, modal focus helpers, recursive task lookup, task-state history, workflow persistence funnel, and whole-tree IPC save. Add one small task-edit UI module and one modal. The state mutation changes only `text` and, for background tasks, `backgroundOptions.priority`; every other task field and tree/reference relationship stays intact.

**Tech Stack:** Electron, TypeScript, renderer-native DOM APIs, existing CSS, Vitest 4 with jsdom, and the existing `window.echosight` IPC bridge.

## Repository-grounded decisions

### Already present and reused

- `src/renderer/features/tasks/taskListRenderer.ts` already attaches `contextmenu` to every rendered `.task-text` and reports the recursive task ID. No production change is needed there.
- `src/renderer/features/tasks/taskListView.ts` already renders all main roots recursively and active background roots recursively. This already makes nested tasks and active background tasks reach the same context-menu path. No production change is needed there.
- `src/renderer/features/tasks/taskTree.ts::findTaskById` searches the full tree, so an edit mutation can address root or nested tasks by ID without moving them.
- `src/renderer/features/tasks/taskStateController.ts::withUndo` snapshots the whole task tree, clears redo after a real mutation, and is already used by add/toggle/delete/move.
- `src/renderer/features/tasks/taskWorkflowController.ts::persistTaskChanges` already performs the required rerender, progress update, and `saveTasks()` call.
- `src/renderer/features/tasks/taskPersistence.ts`, `src/preload/index.ts`, and `src/main/ipc/storageIpc.ts` persist the entire `TaskSaveData`; an in-place edit needs no IPC, shared-type, schema, package, or main-process change.
- `src/renderer/ui/modalUi.ts` already supplies visibility, focus trapping, Escape-to-cancel, focus restoration, and text selection.
- `src/renderer/ui/staticControls.ts` already centralizes click and Enter-key bindings.
- `src/renderer/features/tasks/backgroundTaskUi.ts` proves that the only background setting users can currently set while creating a background task through **Configure Triggers** is **High Priority**. Its exported `readTrimmedInputValue` and `readCheckboxChecked` helpers can be reused by the edit form.
- `src/renderer/features/tasks/backgroundTaskController.ts` owns expiration timers. Editing text or priority does not require restarting a timer because `expiresAfterMinutes`, `activated`, and `activatedAt` remain unchanged.

### Minimal UX and data contract

- Add **Edit task** as the first item in the existing right-click menu.
- Open one minimal **Edit Task** modal for both modes.
  - All tasks show a prefilled task-name input with the existing 100-character HTML limit.
  - Background tasks additionally show a prefilled **High Priority** checkbox.
  - Normal tasks hide that background-only row.
- Save trims the text. Empty text shows the existing themed alert and leaves the modal open with no mutation, history entry, rerender, or save.
- Cancel or Escape closes the modal and clears the pending task ID without changing state.
- A valid no-op save closes the modal but does not add undo history, rerender, or persist.
- A real save records one `edit task` undo entry, mutates the existing task in place, rerenders, updates progress, and persists through the existing workflow funnel.
- Editing a background task changes only its text and priority. Preserve `id`, `createdAt`, `children`, `triggers`, `completed`, `mode`, `activated`, `activatedAt`, and `backgroundOptions.expiresAfterMinutes`.
- Editing a normal task changes only its text. Ignore background form state for normal tasks and preserve `backgroundOptions` as-is.

### Source-resolved ambiguity

- `BackgroundOptions.expiresAfterMinutes` exists and drives timers, but no current renderer control lets users set it when creating/configuring a background task. Therefore this feature must preserve it but must not invent an expiration editor.
- Dormant background tasks are listed inside **Configure Triggers** but are not rendered in `backgroundTaskList`, so they do not have a task-text right-click target. To keep every user-created background task editable, add a minimal **Edit** button to each existing Configure Triggers row. It opens the same edit modal and leaves trigger checkbox behavior intact; no second management screen is needed.
- The existing `isParent` argument in the task-list context-menu callback is unused by `renderer.ts`; do not remove or repurpose it as part of this feature.

## Files at a glance

### Create

- `src/renderer/features/tasks/taskEditUi.ts`
- `tests/unit/tasks/rendererTaskEditing.test.ts`
- `tests/unit/tasks/triggerController.test.ts`

### Modify

- `src/renderer/features/tasks/taskMutations.ts`
- `src/renderer/features/tasks/taskStateController.ts`
- `src/renderer/features/tasks/taskWorkflowController.ts`
- `src/renderer/features/tasks/contextMenuUi.ts`
- `src/renderer/features/tasks/backgroundTaskUi.ts`
- `src/renderer/features/tasks/triggerController.ts`
- `src/renderer/renderer.ts`
- `src/renderer/ui/staticControls.ts`
- `src/renderer/index.html`
- `src/renderer/styles.css`
- `tests/unit/tasks/taskStateController.test.ts`
- `tests/unit/tasks/taskWorkflowController.test.ts`

### Reuse unchanged

- `src/shared/types.ts`
- `src/renderer/features/tasks/taskListRenderer.ts`
- `src/renderer/features/tasks/taskListView.ts`
- `src/renderer/features/tasks/backgroundTaskController.ts`
- `src/renderer/features/tasks/taskPersistence.ts`
- `src/renderer/ui/modalUi.ts`
- package files and build/test configuration

## Task 1: Add the undoable domain mutation

**Objective:** Update a task by recursive ID while changing only editable fields, rejecting empty/no-op edits, and participating in existing state history.

**Exact files:**

- Modify `tests/unit/tasks/taskStateController.test.ts`
- Modify `src/renderer/features/tasks/taskMutations.ts`
- Modify `src/renderer/features/tasks/taskStateController.ts`

**Failing test (RED):**

Add focused tests to `tests/unit/tasks/taskStateController.test.ts`:

1. Load a parent with a nested normal child whose `completed`, `triggers`, and timestamps are non-default. Call `controller.editTask(childId, '  Renamed child  ')`. Assert:
   - the return value is `true`;
   - the child text becomes `Renamed child`;
   - the same ID, parent/child placement, completion, activation, trigger references, timestamps, mode, children, and background options remain unchanged.
2. Load an active background task with a stable ID, `completed` value, `activatedAt`, triggers, and `{ expiresAfterMinutes: 15, priority: 'normal' }`. Edit its text and priority to `high`. Assert:
   - expiration and all non-editable fields remain unchanged;
   - undo returns `{ restored: true, label: 'edit task' }` and restores the old text/priority;
   - redo restores the new text/priority.
3. Assert whitespace-only text and a normalized no-op both return `false`, create no undo entry, and leave state byte-for-byte unchanged.

**RED command:**

```bash
npm test -- --run tests/unit/tasks/taskStateController.test.ts
```

**Expected failure:** The new tests fail because `TaskStateController` has no `editTask` API and `taskMutations.ts` has no edit mutation.

**Minimal implementation (GREEN):**

In `src/renderer/features/tasks/taskMutations.ts`, add:

```ts
export function editTaskInList(
  taskId: number,
  taskList: Task[],
  text: string,
  priority?: BackgroundPriority
): boolean
```

Behavior:

- Normalize with `text.trim()` and return `false` when empty.
- Find the task with the existing recursive `findTaskById`.
- Return `false` when the ID is missing.
- For a normal task, detect changes from text only.
- For a background task, compare against `task.backgroundOptions?.priority ?? 'normal'`; only consider priority when the optional `priority` argument is supplied.
- Return `false` before assigning anything when neither editable value changes.
- Assign the trimmed text.
- When background priority changes, replace only `backgroundOptions` with:

```ts
{
  expiresAfterMinutes: task.backgroundOptions?.expiresAfterMinutes ?? null,
  priority
}
```

Do not recreate the `Task`, traverse/mutate children, or touch any other field.

In `src/renderer/features/tasks/taskStateController.ts`:

- Import `BackgroundPriority` and `editTaskInList`.
- Add this public signature:

```ts
editTask: (taskId: number, text: string, priority?: BackgroundPriority) => boolean;
```

- Implement it through the existing helper:

```ts
editTask: (taskId, text, priority) =>
  withUndo(
    'edit task',
    () => editTaskInList(taskId, tasks, text, priority),
    changed => changed
  )
```

This keeps redo clearing and the 20-entry history limit consistent with all other real mutations.

**Passing verification:**

```bash
npm test -- --run tests/unit/tasks/taskStateController.test.ts
npm run typecheck
```

**Commit message:**

```text
feat(tasks): add undoable task edit mutation
```

## Task 2: Add and control the minimal edit modal

**Objective:** Reuse the current modal system to prefill the selected task, expose only current user-editable background settings, validate saves, and route real changes through the workflow persistence funnel.

**Exact files:**

- Modify `tests/unit/tasks/taskWorkflowController.test.ts`
- Create `src/renderer/features/tasks/taskEditUi.ts`
- Modify `src/renderer/features/tasks/taskWorkflowController.ts`
- Modify `src/renderer/index.html`
- Modify `src/renderer/styles.css`

**Failing test (RED):**

Extend `tests/unit/tasks/taskWorkflowController.test.ts` with a local DOM fixture containing the edit modal IDs listed below and an injected `alertUser` spy.

Add tests that:

1. Open a nested normal task and assert its text is prefilled/selected, the background options row is hidden, and a valid save updates state and calls `renderTasks`, `updateProgress`, and `saveTasks` exactly once.
2. Open an active background task and assert the background row is visible and reflects its current priority. Change text and priority, save, and assert the workflow updates both while preserving expiration and activation fields.
3. Enter whitespace and assert the alert receives `Please enter a task name!`, the modal stays open, state is unchanged, and render/save/history are untouched.
4. Cancel after changing form controls and assert no state or persistence calls occur.
5. Save unchanged normalized values and assert the modal closes without render/save.

**RED command:**

```bash
npm test -- --run tests/unit/tasks/taskWorkflowController.test.ts
```

**Expected failure:** Tests fail because the task-edit UI module and the workflow methods `openTaskEditor`, `closeTaskEditor`, and `saveTaskEdit` do not exist.

**Minimal implementation (GREEN):**

Create `src/renderer/features/tasks/taskEditUi.ts` with:

```ts
export interface TaskEditFormValue {
  text: string;
  highPriority: boolean;
}

export function showTaskEditModal(task: Task, api?: FocusOverlayApi): boolean;
export function closeTaskEditModal(): void;
export function readTaskEditForm(): TaskEditFormValue;
```

Implementation requirements:

- Reuse `showModal`/`hideModal` from `ui/modalUi`.
- Reuse `focusOverlayNow` and select `#taskEditInput` on open.
- Reuse `readTrimmedInputValue` and `readCheckboxChecked` from `backgroundTaskUi`.
- Return `false` without opening when required modal/form elements are missing.
- Set `taskEditInput.value = task.text`.
- Set `taskEditBackgroundOptions.hidden = task.mode !== 'background'`.
- Set `taskEditHighPriority.checked` from `task.backgroundOptions?.priority === 'high'`.
- Do not retain task IDs in the UI module.

Add the minimal markup to `src/renderer/index.html` near the existing task modals:

```html
<div class="modal" id="taskEditModal" aria-hidden="true">
  <div class="modal-content modal-content-task-edit">
    <h3 class="modal-title">Edit Task</h3>
    <label class="setting-label" for="taskEditInput">Task name</label>
    <input type="text" class="modal-input" id="taskEditInput" maxlength="100">
    <div class="checkbox-group" id="taskEditBackgroundOptions" hidden>
      <label class="checkbox-row">
        <input type="checkbox" id="taskEditHighPriority"> High Priority
      </label>
    </div>
    <div class="modal-buttons">
      <button class="modal-btn secondary" id="cancelTaskEditButton" data-modal-close>Cancel</button>
      <button class="modal-btn primary" id="saveTaskEditButton">Save</button>
    </div>
  </div>
</div>
```

In `src/renderer/styles.css`, reuse the existing compact modal size rather than adding a new layout:

```css
.modal-content-subtask,
.modal-content-task-edit {
  max-width: 350px;
}
```

The existing `.modal-input`, `.checkbox-group`, `.checkbox-row`, and `.modal-buttons` rules cover the rest.

In `src/renderer/features/tasks/taskWorkflowController.ts`:

- Add optional `api?: FocusOverlayApi` to `TaskWorkflowControllerOptions`.
- Add the public methods:

```ts
openTaskEditor: (taskId: number) => void;
closeTaskEditor: () => void;
saveTaskEdit: () => void;
```

- Keep one private `editingTaskId: number | null`.
- `openTaskEditor` recursively finds the current task, opens/prefills the modal, and records the ID only when the modal opens successfully.
- `closeTaskEditor` hides the modal and clears the ID. This is also the Escape/cancel path.
- `saveTaskEdit`:
  1. returns when no edit is active;
  2. re-finds the task in case state changed;
  3. reads the trimmed form;
  4. alerts and keeps the modal open when text is empty;
  5. passes `high`/`normal` only when the actual task mode is `background`;
  6. calls `taskState.editTask`;
  7. calls existing `persistTaskChanges()` only when it returns `true`;
  8. closes the modal after a valid changed or no-op save.

Do not add a separate edit controller; `TaskWorkflowController` already owns task mutation orchestration, alerts, and the render/progress/save funnel.

**Passing verification:**

```bash
npm test -- --run tests/unit/tasks/taskWorkflowController.test.ts
npm test -- --run tests/unit/tasks/taskStateController.test.ts
npm run typecheck
```

**Commit message:**

```text
feat(tasks): add task edit modal workflow
```

## Task 3: Wire right-click through save, rerender, and persistence

**Objective:** Connect the existing rendered task context-menu event to the edit workflow and prove the complete renderer path for a nested normal task and active background task.

**Exact files:**

- Create `tests/unit/tasks/rendererTaskEditing.test.ts`
- Create `tests/unit/tasks/triggerController.test.ts`
- Modify `src/renderer/features/tasks/contextMenuUi.ts`
- Modify `src/renderer/features/tasks/backgroundTaskUi.ts`
- Modify `src/renderer/features/tasks/triggerController.ts`
- Modify `src/renderer/ui/staticControls.ts`
- Modify `src/renderer/renderer.ts`
- Modify the context-menu and trigger-row rules in `src/renderer/styles.css`

`src/renderer/features/tasks/taskListRenderer.ts` and `taskListView.ts` remain unchanged; the integration test deliberately exercises them.

**Failing test (RED):**

Create one renderer integration test in `tests/unit/tasks/rendererTaskEditing.test.ts` to avoid duplicate `DOMContentLoaded` listeners from repeated renderer imports.

Test setup:

- Parse/import the real `src/renderer/index.html` body into jsdom so the production modal IDs and buttons are exercised rather than copied into a test-only fixture.
- Install a fake `window.echosight` API with:
  - one main parent containing a nested child;
  - the main parent or child referencing an active background task ID;
  - one active background task with stable `id`, `completed`, `activatedAt`, triggers, and `{ expiresAfterMinutes: 15, priority: 'normal' }`;
  - a spying `saveTasks` implementation that records the actual `TaskSaveData`.
- Import `renderer.ts`, dispatch `DOMContentLoaded`, and wait for initial task rendering.

Exercise and assert the full flow:

1. Dispatch `contextmenu` on the nested child’s rendered `.task-text`.
2. Click **Edit task** in `.context-menu`.
3. Change `#taskEditInput`, press Escape, and assert the modal closes without changing state or calling `saveTasks`.
4. Reopen **Edit task**, change `#taskEditInput`, click `#saveTaskEditButton`, and wait for:
   - the nested task text to rerender;
   - `saveTasks` to receive the edited nested tree;
   - the child ID, parent relationship, completion, and trigger data to remain unchanged.
5. Dispatch `contextmenu` on the active background task’s rendered text.
6. Assert the background controls are visible and prefilled.
7. Change text and check high priority, then save.
8. Assert:
   - the background row rerenders with the new text and `.high-priority`;
   - the persisted payload keeps the same background task ID;
   - normal-task trigger references still point to that ID;
   - `completed`, `activated`, `activatedAt`, children/triggers, and `expiresAfterMinutes` are unchanged;
   - priority is now `high`.

This is the repository’s highest useful integration boundary: it covers right-click event creation, menu action, modal controls, workflow mutation, rerender, and the renderer-to-IPC save payload. Existing main-process storage tests already cover durable JSON writes, so do not add an Electron end-to-end harness.

Also add a focused `tests/unit/tasks/triggerController.test.ts` case for a dormant background task listed in Configure Triggers: clicking its **Edit** button calls the shared `openTaskEditor(id)` callback without toggling the trigger checkbox; the existing trigger selection can still be changed and saved afterward. The renderer integration test should additionally prove the edit modal can be opened while Configure Triggers remains visible and that the edited background task is persisted.

**RED command:**

```bash
npm test -- --run tests/unit/tasks/rendererTaskEditing.test.ts
```

**Expected failure:** No **Edit task** menu item or renderer/static-control wiring exists, so the modal cannot be opened or saved from the rendered task.

**Minimal implementation (GREEN):**

In `src/renderer/features/tasks/contextMenuUi.ts`:

- Extend `TaskContextMenuActions`:

```ts
onEditTask: (taskId: number) => void;
```

- Insert **Edit task** as the first menu item. Its click handler must close the menu before calling `actions.onEditTask(taskId)`.
- Leave Add sub-task, Configure Triggers, outside-click closing, and task-ID behavior unchanged.

In `src/renderer/styles.css`:

- Update the existing `:nth-child(...)::before` context-menu icon rules for the new order:
  1. edit/pencil;
  2. add/plus;
  3. triggers/lightning.
- Do not add a new menu component or icon dependency.

In `src/renderer/ui/staticControls.ts`:

- Add `closeTaskEditor` and `saveTaskEdit` to `RendererControlHandlers`.
- Bind `cancelTaskEditButton` to `closeTaskEditor`.
- Bind `saveTaskEditButton` to `saveTaskEdit`.
- Bind Enter on `taskEditInput` to `saveTaskEdit`, matching task/subtask input behavior.

In `src/renderer/renderer.ts`:

- Pass `api: ipc` into `createTaskWorkflowController`.
- In `showContextMenu`, pass:

```ts
onEditTask: taskWorkflowController.openTaskEditor
```

- In `setupRendererControls`, pass:

```ts
closeTaskEditor: taskWorkflowController.closeTaskEditor,
saveTaskEdit: taskWorkflowController.saveTaskEdit,
```

- Keep the existing render callbacks, `renderTaskLists`, drag reinitialization, progress update, and persistence gate unchanged.

**Passing verification:**

```bash
npm test -- --run tests/unit/tasks/rendererTaskEditing.test.ts
npm test -- --run tests/unit/tasks/taskWorkflowController.test.ts tests/unit/tasks/taskStateController.test.ts
npm run typecheck
npm run build
```

**Commit message:**

```text
feat(tasks): wire edit action into task menu
```

## Acceptance criteria

- Right-clicking a rendered normal task shows **Edit task** and opens a prefilled edit modal.
- The same action works for nested tasks without moving them or changing their parent.
- Right-clicking a currently active background task opens the same modal with **High Priority** visible and prefilled.
- Saving trims and updates text; background saves also update priority.
- Empty text is rejected with visible feedback, and the modal remains open.
- Cancel and Escape make no mutation and create no history or persistence call.
- A normalized no-op save creates no history or persistence call.
- Real edits create exactly one `edit task` undo entry.
- Undo and redo restore both normal text and background text/priority through the existing rerender/save flow.
- IDs, trigger references, hierarchy, completion, activation, activation timestamp, creation timestamp, children, unrelated triggers, and expiration remain intact.
- A successful edit rerenders the correct list, updates progress through the existing funnel, and sends the updated whole task state to `window.echosight.saveTasks`.
- No new dependency, framework, IPC method, shared persisted type, or package change is added. Dormant background tasks use the existing Configure Triggers list rather than a new management surface.

## Risk and edge-case checklist

- [ ] Recursive lookup edits a nested task rather than only top-level arrays.
- [ ] A missing/deleted task ID while the modal is open closes safely without saving.
- [ ] Normal-task saves never create or overwrite background options.
- [ ] Background tasks with `backgroundOptions: null` are treated as normal priority; selecting high creates options with `expiresAfterMinutes: null`.
- [ ] Existing non-null `expiresAfterMinutes` survives priority edits.
- [ ] Active background tasks stay active and keep the same `activatedAt`; their current timer is not reset.
- [ ] Completed tasks stay completed and keep line-through rendering after text changes.
- [ ] Trigger arrays on both the edited task and referring tasks remain unchanged.
- [ ] Redo is cleared only after a real edit, not after empty/cancel/no-op attempts.
- [ ] Modal Escape uses the production cancel button and clears `editingTaskId`.
- [ ] Focus returns to the task/menu opener after close through existing modal behavior.
- [ ] Enter saves from the text input; it does not bypass empty-text validation.
- [ ] The context menu still closes on selection and outside click.
- [ ] Add sub-task and Configure Triggers retain their callbacks and icons after inserting Edit.
- [ ] Rerendered task rows retain drag/drop initialization.
- [ ] A successful edit rerenders the correct list, updates progress through the existing funnel, and sends the updated whole task state to `window.echosight.saveTasks`.
- [ ] Editing a dormant background task from Configure Triggers leaves the trigger checkbox selection intact until the user explicitly changes and saves triggers.
- [ ] Persistence-disabled and persistence-failure feedback still goes through the existing `renderer.ts::saveTasks` gate.
- [ ] No new dependency, framework, IPC method, shared persisted type, or package change is added. Dormant background tasks use the existing Configure Triggers list rather than a new management surface.

## Final verification commands

```bash
npm test -- --run \
  tests/unit/tasks/taskStateController.test.ts \
  tests/unit/tasks/taskWorkflowController.test.ts \
  tests/unit/tasks/rendererTaskEditing.test.ts

npm run verify
```

Manual smoke check:

```bash
npm run dev
```

In interactive mode, verify normal, nested, completed, and active background task edits; verify empty text, Cancel, Escape, undo, redo, restart persistence, and that Add sub-task/Configure Triggers still work.
