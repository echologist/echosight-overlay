# Runnable background tasks

## Runtime

`TaskApp` owns `BackgroundTaskState<TaskClock>`. All edit, complete, toggle,
delete, history, trigger and drag commands pass through the scheduler facade.
`TaskApp::state()` is read-only; no mutable TaskState escapes it.
`Move { id, target, above, subtask }` returns `BackgroundTransition.moved` and
preserves captured timer identity through reorder, nesting and promotion. A
labelled topology replay uses the existing move implementation, not ID lookup or
a second move algorithm. Failed moves restore the exact original structural
parent, including duplicate parent IDs; successful sibling destinations retain
the selected parent identity too. Failed moves leave timer paths/deadlines intact.
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
platform audio remains deferred. Completion feedback comes only from the actual
successful `BackgroundTransition.completed_task` result, never a second leaf
search. Complete-next filters background roots before descending; no eligible
main-root leaf means no completion feedback. Feedback lasts two seconds;
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
  trigger selection; cancelling the parent does not roll back that edit. The
  parent's displayed trigger labels remain stale after nested editing, matching TS.
- Creation validates the same trimmed nonempty description, enforces the existing
  100 UTF-16 insertion limit, saves immediately, then rebuilds choices from saved
  triggers. Unsaved checked changes reset; the new task is not automatically
  selected. Description clears, High Priority stays checked if previously set.
- **The shipped TS form has no expiry input.** Creation explicitly supplies null
  expiry; dormant/active name or priority edits preserve loaded expiry. Real
  scheduled expiry is supported for loaded/configured task data. No new expiry
  input was invented contrary to the master plan's preserve-for-parity rule.
- Active background roots have their own Background Tasks heading/count and HIGH
  badge; dormant **roots** are not exposed as ordinary task rows. As in
  `taskListView.ts` / `taskListRenderer.ts`, all descendants of visible roots are
  rendered, even dormant background children and legacy mixed-mode descendants.
  Activating a parent does not activate its children. Core progress retains its
  reviewed root-filter/descendant-counting legacy quirks.
- Dialog title/footer do not shrink. Only the viewport-bounded body scrolls; the
  overlay occludes wheel/pointer events. Loaded long names survive unchanged or
  priority-only saves; insertion limits apply only to new text.

## Initial slice verification

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

## Reviewed parity-blocker regressions

This follow-up uses the worktree's own fresh `target`, never copied workspace
artifacts. Before production changes, all four named `parity_` UI tests failed:
wrong completion text, false no-op feedback, missing dormant child after a
triggered parent activation, and missing legacy mixed-mode descendant. Three
named core tests also failed: wrong rollback parent, the captured-expiry panic,
and wrong duplicate-parent sibling destination. The new actual-TS fixture
comparison failed at the failed move's exact post-tree.

- `BackgroundTransition.completed_task: Option<Task>` is an additive, nonserialized
  result populated only by successful CompleteNext. Existing serialized oracle
  shapes and fixtures are unchanged. Duplicate-ID structural completion and app
  ambiguity guards remain covered by the original suites.
- `tools/golden/background-ui.test.ts` dispatches DOM clicks into the real TS
  workflow and inspects production feedback/rendering. It confirms no-op feedback
  stays absent and activated-parent rows have IDs `[20, 21]` while 21 is dormant.
  GPUI regressions dispatch actual mouse clicks and assert `row-1-0` bounds.
- Two new actual-controller input/expected pairs pin failed-move rollback/expiry
  and successful duplicate-parent destinations, including removal-index shifts.
  Full trees, schedules, exact effects and save payloads are compared. Original
  fixture bytes were checked before regeneration and remained unchanged after it.
  Source self-subtask success/lost-object behavior is deliberately unchanged.
- `cargo test --locked --workspace`: **183 test functions passed**, including
  **65 UI tests**. The direct failed-move regression, public-facade expiry probe,
  background oracle and original task oracle all pass with nonzero named tests.
- Workspace fmt and locked all-target clippy (`-D warnings`), Windows all-target
  app/platform checks, and Linux app plus `s8_task_app` linking passed.
- Both linked binaries were launched from an unrelated disposable directory:
  no arguments exited 2, a missing explicit directory stayed absent, and a loaded
  active background task expired through the actual timer and saved one snapshot.
  Each running event loop was externally bounded (124); no native UI acceptance.
- TS background freshness **24 passed** (19 scenarios, four new workflow/render
  regressions, one existing trigger regression); original task lane **33 passed**;
  original trigger/background/edit DOM tests **3 passed**. Strict standalone
  TypeScript checking of both background tests and config passed. Original jsdom
  tests still emit their existing unimplemented `window.focus` warning.

## Integrated checkpoint review

The combined feature candidate `ecaf18b529f7284a3c2ba3ea061eef739766d2d9`
passed independent spec-then-quality reviews for both runtime/persistence and
trigger-dialog/rendering scopes. No publication blockers remained.

Parent verification on the combined source used a fresh build target: 183 Rust
workspace tests and 123 TypeScript tests across all seven oracle lanes passed;
formatting, locked all-target Clippy and Windows app/platform checks passed.
Both linked Linux executables independently passed explicit-directory startup
and real scheduled-expiry persistence smoke tests from an unrelated directory.
No-argument startup exited 2, missing data directories remained absent, and expiry
persisted an inactive task with exactly one snapshot. Event-loop timeout is not
native acceptance.

Independent runtime stress probes covered 1,000 deterministic typed trees,
16,056 moves and 3,944 deletions before captured expiry. Independent dispatched
UI probes covered expiry during nested editing, undo-focus restoration and
expiry-save failure with intact modal cancellation. All supplied tests and named
review probes passed; original fixture bytes and protected Electron resources
were unchanged. Review stress probes remain separate from committed test counts.

## Remaining native gates

Native global hotkeys, game monitoring, tray, theme runtime and calibrated audio
remain separate work. Linux headless tests and Windows cross-checks do not
establish native Windows focus, IME, accessibility, pixel appearance or game-
overlay acceptance. No Electron sources, assets, package manifests or workflows
were changed. Expiry form controls and notification animation are not claimed.
