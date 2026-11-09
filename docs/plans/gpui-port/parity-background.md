# Background tasks: bounded core parity

## Delivered contract

`task::background::BackgroundTaskState<E>` owns an existing `TaskState<E>` and a pure expiry schedule. It uses the existing f64 `TaskId` and injected `TaskEnvironment`; no new dependencies, runtime, threads, sleeps, timers, disk access, or real user data.

- `dispatch(BackgroundCommand)` returns `BackgroundTransition { activated_count, history, effects }`.
- Commands: `CompleteNext`, `Toggle`, `ActivateTriggers`, `DeactivateTriggers`, `Restart`, `Cancel`, `CancelAll`, `ExpireDue`, `Undo`, `Redo`, `Delete`, `Clear`, `Replace`, `Edit`, `UpdateTriggers`, `AddBackground`, `AddTask`, `CaptureUndo`, `Save`.
- Effects: `Schedule { id, due_ms }`, `Cancel { id }`, `Persist { data }`, `Activated { count, message }`, `Sound { event }`, `Log { message }`.
- `timers()` exposes ID-keyed deadlines; `next_due_ms()` gives the next wakeup. `environment_mut()` permits updating an injected test/runtime clock. A UI can use one wakeup, refresh the clock, dispatch `ExpireDue`, and re-arm from `next_due_ms()`. Stale wakeups cannot expire a later deadline early.
- `state()` is read-only, preventing timer targets from silently drifting under unrelated structural edits. `trigger_rows(task, tasks)` exposes all nested background tasks, including dormant/completed ones, checked state, optional `HIGH`, `ACTIVE`/`DORMANT`, and edit availability.
- `TaskState::update_triggers` is also independently available. Ordered equality is significant; only changed selections capture `update task triggers` history and clear redo.

The UI must confirm delete/clear **before** dispatch. `Persist` requests rendering/progress refresh and supplies the already-captured, five-snapshot-capped save payload. The shell writes that payload through its persistence readiness/error gate; it must **not** record a second snapshot. Every effect owns its data from the moment the source callback occurred, including multiple immediate expirations within a single restart. No persistence I/O occurs in this module. `AddBackground` mirrors the raw state operation and needs an explicit `Save` (as the trigger controller's onChanged callback does).

## Source behavior preserved

Sources: unchanged `backgroundTaskController.ts`, `triggerController.ts`, `taskStateController.ts`, `taskWorkflowController.ts`, their real completion/creation/tree/persistence helpers, `backgroundTaskUi.ts`, and renderer wiring at `renderer.ts:68-79,103-115`.

- Complete-next activates the selected leaf's triggers and newly completed ancestors' triggers. Parent toggle recursively completes children but does **not** activate every descendant's triggers. Activation events/counts remain separate per source callback, followed by the original task-completed sound/persistence order.
- Lookup uses the first matching ID, but completion retains the actual selected object when IDs collide. Timer records overwrite by ID while retaining the actual last-scheduled object's structural identity. Tests include colliding roots, nested leaves under different parents, and colliding background timer targets.
- Activation skips already-active tasks, not completed dormant tasks. Unchecking deactivates only active, incomplete targets and clears activatedAt. Expiry leaves activatedAt intact and does not create history.
- Restart does not clear unrelated timers; it resets active incomplete tasks with missing/invalid activatedAt to a full duration, otherwise uses the remaining duration. Past deadlines expire synchronously and request persistence. Zero/null durations don't start timers. Negative durations expire immediately. Fractional/overflow timer delays match the observed fake timer runtime, including integer truncation and overflow to 1ms.
- Creation keeps the existing source-compatible option sanitization (`Number`, positivity, floor, priority): strings, booleans, empty arrays, null, missing fields and fractional/negative values are exercised through the real creation controller. Loaded typed expiry values support finite numeric/null values including negative, zero and fractional values. Canonical ISO timestamps, explicit timezone offsets, UTC ISO date-only strings, missing/empty/invalid timestamps and future activation times are supported.
- Successful undo/redo clears all timers, restores deep-cloned task state, restarts eligible timers, persists, and plays the appropriate sound. No-op history leaves timers untouched. Old deadlines are not restarted from "now" when activatedAt is valid.
- Delete cancels descendant IDs and adjusts surviving structural paths. Workflow clear is a no-op when progress is zero, even if dormant tasks exist.
- Raw `Replace` does **not** clear timers, matching `replaceTasks`: old closures can still log/request a save but must not mutate new same-ID tasks. The scheduler retains detached captured targets for this case. Template replacement must dispatch `CancelAll` first, matching renderer/template wiring.

## Oracle and verification

`tools/golden/background.gen.test.ts` imports and executes the real production controllers, DOM trigger/edit helpers and workflow. It freezes Date and Math.random, advances Vitest fake timers, and lets the **real timer closures** run. An observation-only activatedAt accessor identifies the task associated with a scheduled callback without changing its value. Deadlines come directly from the installed fake clock's actual timer records, not a copied delay algorithm. Effects/save payloads/timers are deep-copied immediately; every operation records tasks, active deadlines, effects, trigger metadata and history results.

The 15 input/expected pairs contain 110 transitions. The Rust test executes production commands and checks structured results plus exact JSON.stringify-compatible pretty bytes. Provenance hashes in each expected fixture cover the imported controller/helper sources; the TS read-only check compares regenerated bytes, including these hashes. The original 24 task fixture pairs are unchanged.

TDD evidence: the compiling identity adapter failed on `colliding-complete-next` before production implementation. A subsequent legacy date-only oracle case failed before adding UTC date-only support. No expected state was authored by copying Rust/source algorithms.

```sh
# Run read-only before any regeneration; checks provenance and output bytes.
BACKGROUND_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.background.config.ts
# Intentional oracle regeneration only:
npx vitest run --config tools/golden/vitest.background.config.ts
# Repeat read-only to verify deterministic output.
BACKGROUND_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.background.config.ts
cargo test --locked -p echosight-core
cargo fmt --all --check
cargo clippy --locked -p echosight-core --all-targets -- -D warnings
```

The isolated TS config runs the 15 generated scenarios plus the unchanged real dormant-edit trigger-controller regression (16 tests). A standalone strict TypeScript check also covers the generator/config; Electron sources/package/config are untouched.

## Explicitly deferred / bounded

- Actual GPUI wakeups, visual notification lifetime/animation, feedback, focus/modal behavior, audio playback, readiness-gated disk writes and Windows runtime integration remain shell work. Core parity does not prove these runtime effects.
- This is a bounded background workflow facade, not a replacement for every task UI operation. Reordering is not exposed through `BackgroundCommand`; future integration must preserve timer object identity when adding that operation, rather than mutating the underlying task tree behind the scheduler.
- Existing typed `Task`/`BackgroundOptions` decoding is unchanged. Arbitrary malformed loaded option objects, nonnumeric loaded expiry fields, non-string activatedAt and implementation-specific JavaScript Date formats/local-time parsing are not newly accepted. This slice proves source-tolerant **creation options** and the documented typed scheduling domain, not arbitrary raw-JS task loading. Keep the existing typed-load parity gate open for those records.
- Timer overflow behavior is grounded in the real installed Vitest fake-clock execution; native UI scheduling must honor the emitted deadlines, not reinterpret duration fields through another runtime's timer coercion.
