# Runnable background tasks

## Runtime

`TaskApp` owns `BackgroundTaskState<TaskClock>`. All edit, complete, toggle,
delete, history, trigger and drag commands pass through the scheduler facade.
`TaskApp::state()` is read-only; no mutable TaskState escapes it.
`Move { id, target, above, subtask }` returns `BackgroundTransition.moved` and
preserves captured timer identity through reorder, nesting and promotion. A
labelled topology replay uses the existing move implementation, not ID lookup or
a second move algorithm; duplicate IDs retain the core's existing semantics.
The app blocks ambiguous row/ancestor IDs, including trigger-choice dispatch.

`TaskClock::anchored(executor, epoch_ms, random)` anchors UTC milliseconds to the
GPUI executor clock; `realtime` uses the current UTC epoch. `TaskApp::new_with_runtime`
accepts that clock and a `TaskEventSink`. Virtual-clock tests execute the actual
production `background_executor().timer` + `cx.spawn` path, not a substitute
expiry operation. One owned weak-entity task wakes at the next deadline; each
rearm cancels its predecessor and increments a generation guard. No polling.
Dropping the entity cancels the task. Failed loads never restart; save failures
cancel wakeups and block further mutations. Missing startup directories remain
absent until a mutation needs saving.

`dispatch_background` is the integration seam for subsequent template/settings
work. Replacement callers must dispatch `CancelAll` before `Replace`, then
explicitly `Save` as appropriate. `AddBackground` also requires explicit `Save`.
`Persist` payloads already contain their capped snapshot; FIFO writes serialize
exactly those payloads, including separate immediate expirations during
load/restart. No debounce or second snapshot. Unrelated edits/moves preserve
deadlines; successful history clears/restarts according to source rules,
including missing-activatedAt restart behavior. History stacks remain 20 deep,
auto-save snapshots five deep.

`TaskRuntimeEvent::{Sound, Activated, Log}` is an ordered injectable output port.
The default logs requests; **no native audio playback is claimed**. Calibrated
platform audio remains deferred. Completion feedback lasts two seconds;
activation notification lifetime is 3.3 seconds (the source's 3s + 300ms exit).
The exit animation itself remains a visual parity gate. Expiry logs are emitted
through the sink; actual persistence results update Saved/error status. Expiry
of a focused row restores focus to a surviving composer so local undo still works.

## Trigger and background views

- Main rows expose Configure Triggers. The modal enumerates nested background
  records, including dormant tasks, HIGH and ACTIVE/DORMANT badges, checkboxes
  and the shared Edit Task dialog. Its initial focus is New background task.
- Nested editing retains the parent's draft checked choices, creation input and
  priority, and restores focus to the originating Edit control. Escape cancels
  only the top dialog. Tab/Shift-Tab stay within that dialog; danger confirmation
  remains Cancel-focused and ignores bare Enter.
- Checked IDs save in rendered tree order; unchanged ordered selections cause
  no history/save. Editing a dormant task persists immediately without changing
  trigger selection; cancelling the parent does not roll back that edit.
- Creation validates the same trimmed nonempty description, enforces the existing
  100 UTF-16 insertion limit, saves immediately, then rebuilds choices from saved
  triggers. Unsaved checked changes reset; the new task is not automatically
  selected. Description clears, High Priority stays checked if previously set.
- **The shipped TS form has no expiry input.** Creation explicitly supplies null
  expiry; dormant/active name or priority edits preserve loaded expiry. Real
  scheduled expiry is supported for loaded/configured task data. No new expiry
  input was invented contrary to the master plan's preserve-for-parity rule.
- Active background roots have their own Background Tasks heading/count and HIGH
  badge; dormant records are not exposed as ordinary task rows. Core progress
  retains its reviewed root-filter/descendant-counting legacy quirks.
- Dialog title/footer do not shrink. Only the viewport-bounded body scrolls; the
  overlay occludes wheel/pointer events. Loaded long names survive unchanged or
  priority-only saves; insertion limits apply only to new text.

## Verification

All commands used a distinct worktree `target` directory; no shared-worktree
build cache was used for RED/GREEN.

- RED: loaded active task remained active after 60 executor seconds on baseline.
- RED: new real-TS Move fixture differed at the first nesting operation with a
  compiling pre-implementation adapter. Only new fixture pairs were generated;
  existing input/expected pairs and provenance stayed unchanged.
- RED: dormant rows were visible and Configure Triggers was absent before UI work.
- RED: local undo stopped working after the focused row expired; the dispatched
  regression now passes. Ambiguous trigger-choice handler dispatch was also
  reproduced before its guard was added.
- `cargo test --locked --workspace`: **176 test functions passed**, including
  **61 UI tests**. Dispatched source counterparts cover triggerController,
  backgroundTaskUi and rendererTaskEditing, plus scroll/occlusion at 450×600 and
  500×700 with scales 1/1.5/2, nested focus/cancel, loaded long text, 100 UTF-16
  insertion, 20-entry deep history/redo clearing, five snapshots, real expiry,
  movement/deletion/history, stale callbacks, replacement/cancel-all, shutdown,
  invalid loading, save failures and FIFO immediate-expiry payloads.
- Workspace fmt check and locked all-target clippy with `-D warnings`: passed.
- Locked Windows all-target checks for `echosight` and `echosight-platform`: passed.
- Linked actual `echosight` binary and `s8_task_app` harness on Linux: passed.
- Headless binary smoke from an unrelated scratch directory: missing explicit
  path stays absent; no-argument invocation exits 2 without legacy access. An
  isolated active background task expired via the real executor and wrote one
  snapshot with `activated:false`. The bounded event loop was terminated by the
  harness; this is startup/timer evidence, **not native visual acceptance**.
- TS read-only freshness: themes 15, templates 2, settings 16, background 18,
  theme-library 13 tests passed. Tasks 33 and storage 20 tests regenerated byte-
  identical fixtures. Background has 17 actual-controller oracle scenarios,
  including nested parent movement, promotion and ID collisions. The three
  original trigger/background/edit DOM tests also passed unchanged (jsdom reports
  its existing unimplemented `window.focus` warning). Temporary dependency link
  removed.

## Remaining native gates

Native global hotkeys, game monitoring, tray, theme runtime and calibrated audio
remain separate work. Linux headless tests and Windows cross-checks do not
establish native Windows focus, IME, accessibility, pixel appearance or game-
overlay acceptance. No Electron sources, assets, package manifests or workflows
were changed. Expiry form controls and notification animation are not claimed.
