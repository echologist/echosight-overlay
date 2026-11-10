# Runnable background tasks

## Runtime checkpoint

`TaskApp` owns `BackgroundTaskState<TaskClock>`. All existing edit, complete,
toggle, delete, history and drag commands now pass through the scheduler facade.
No mutable TaskState escapes it. `Move { id, target, above, subtask }` returns
`BackgroundTransition.moved` and preserves captured timer identity through
reorder, nesting and promotion. A labelled topology replay uses the existing move
implementation, not ID lookup or a second move algorithm; duplicate IDs retain
the core's existing semantics. The app still blocks ambiguous row/ancestor IDs.

`TaskClock::anchored(executor, epoch_ms, random)` anchors UTC milliseconds to the
GPUI executor clock; `realtime` uses the current UTC epoch. `TaskApp::new_with_runtime`
accepts that clock and a `TaskEventSink`. Virtual-clock tests execute the actual
production `background_executor().timer` + `cx.spawn` path, not a substitute
expiry operation. One owned weak-entity task wakes at the next deadline; each
rearm cancels its predecessor and increments a generation guard. No polling.
Dropping the entity cancels the task. Failed loads never restart; save failures
cancel wakeups and block further mutations.

`dispatch_background` is the integration seam. Replacement callers must dispatch
`CancelAll` before `Replace`, then explicitly `Save` as appropriate. `AddBackground`
also requires explicit `Save`. `Persist` payloads already contain their capped
snapshot; FIFO writes serialize exactly those payloads, including separate
immediate expirations during load/restart. No debounce or second snapshot.
Unrelated edits/moves preserve deadlines; successful history clears/restarts
according to the source rules, including missing-activatedAt restart behavior.

`TaskRuntimeEvent::{Sound, Activated, Log}` is an ordered injectable output port.
The default logs requests; **no native audio playback is claimed**. Calibrated
platform audio remains deferred. Completion feedback lasts two seconds;
activation notification lifetime is 3.3 seconds (the source's 3s + 300ms exit).
The animation itself remains a native visual acceptance item.

## Evidence

- RED: loaded active task remained active after 60 executor seconds on baseline.
- RED: new real-TS Move fixture differed at the first nesting operation with the
  compiling pre-implementation adapter. Only new fixture pairs were generated;
  the existing 15 pairs and their provenance are unchanged.
- GREEN: 52 UI tests including actual dispatched complete/move/edit/delete/history,
  real expiry wakeups, immediate-expiry FIFO payloads, save-failure blocking,
  cancel/replacement/stale callback and entity-drop cancellation.
- Core suite and core/UI all-target clippy pass. The background oracle now has
  17 scenarios, including nested parent movement, promotion and ID collisions.

## Remaining at this checkpoint

Trigger configuration/dormant editing and active-only background rendering are
the next UI slice. Native hotkeys, game monitoring, tray, theme runtime and
calibrated audio remain separate gates. Linux headless tests do not establish
native Windows focus, IME, accessibility or game-overlay acceptance.
