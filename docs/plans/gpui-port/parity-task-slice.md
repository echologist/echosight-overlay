# Phase 1: bounded task-domain slice

Status: **first task slice implemented; Phase 1 is NOT complete**. Base: `634d4eb`.
No Electron production sources, existing Vitest configuration, npm manifests,
UI, settings or template implementation changed. No new Cargo dependencies.

## Source contract and intentional boundaries

Authoritative source is `src/renderer/features/tasks/`, specifically
`taskCreation.ts`, `taskMigration.ts`, `taskTree.ts`, `taskTreeOperations.ts`,
`taskMutations.ts`, `taskCompletion.ts`, `taskProgress.ts`, and
`taskStateController.ts`. `src/shared/types.ts` defines typed task fields.
`taskPersistence.ts` is read to distinguish snapshot logic from I/O.
The renderer inventory/master-plan prose is context, not an alternative oracle.

Implemented in `crates/echosight-core/src/task/`:

- Typed tasks/options/modes, fractional `TaskId(f64)`, JSON-compatible integer
  formatting within the task ID range, injected clock + random source.
- Raw JSON migration preserves unknown fields, insertion order and ignored
  non-record array elements. It fills only the fields the TS migrator fills and
  returns its change flag. It does **not** silently validate/repair malformed data.
- Recursive find/parent/location/preorder flatten/remove, add, edit and delete;
  deletion strips references to all removed descendants, at every depth.
- Reorder, promote, sibling and subtask moves; exact operation/rejection strings.
- Complete-next leaf selection, toggle/descendant completion, upward completion
  and unchecking, root-filtered progress.
- Controller mutation labels, explicit capture/replace/clear, deep-owned undo and
  redo, redo invalidation, 20 entries per history stack, five newest snapshots.
  `record_auto_save_snapshot()` returns the payload an I/O adapter should save;
  snapshot recording occurs before a save attempt, like TS.

The surviving untracked `src/jscompat.rs` was read and reused, not overwritten.
The option-coercion oracle exposed two bugs in that partial helper: signed radix
strings (`+0x10`) must be rejected; arrays must stringify before numeric coercion
(`[true]` is not `true`). Both were corrected against generated fixtures. Its
pre-existing round/UTF-16 helpers remain outside this task slice's parity claim.

## Verified source quirks (do not “fix” during the port)

- Complete-next is **leaf-first**, skips completed ancestor subtrees, and does
  not complete an incomplete parent whose children were already complete.
  This differs from the master-plan shorthand “first incomplete ... pre-order”.
- Progress filters only root main/active-background tasks, then counts **every**
  descendant, including dormant background children of included roots.
- Deleting an absent ID still strips dangling trigger references but records no
  undo entry. Adding a child does not uncheck an already completed parent.
- A direct self-reorder fails and restores the task. Direct self-subtask succeeds
  and removes the detached task from the visible tree in TS. The UI drop-intent
  function prevents that call; Rust preserves the lower-level observable result
  without constructing a reference cycle. UI drop-intent remains pending.
- Unchecking a parent does not uncheck its descendants. Subtasks inherit mode
  and activation but get `activatedAt: null` and `backgroundOptions: null`.
- Migration defaults missing activation to true even for background records;
  explicit null is not missing and is preserved by raw migration.

## Oracle and RED/GREEN evidence

Generate using installed project dependencies, from the worktree root:

```sh
node_modules/.bin/vitest run --config tools/golden/vitest.tasks.config.ts
cargo test -p echosight-core
cargo fmt --all --check
cargo clippy -p echosight-core --all-targets -- -D warnings
```

The isolated configuration runs the generator plus the unchanged original
`tests/unit/tasks/taskStateController.test.ts`. It does not edit `package.json`
or `vitest.config.ts`. The generator imports and runs real TS modules, with a
fixed clock and deterministic random stream; no expected payloads are authored
by hand. The Rust interpreter in `tests/task_support/mod.rs` only dispatches to
production APIs and serializes results; it does not reimplement task rules.

Initial RED was observed **before** the Rust task implementation: a null-result
adapter compiled and failed `typescript_task_fixtures`, **22/22 fixture pairs**
different, exit 101. After implementation, **21/22** passed; the remaining
background-options fixture showed `actual 16, expected null` for `+0x10`.
After the helper correction all 22 passed, both structured values and exact
2-space/LF/no-trailing-newline bytes. This is fixture serialization parity,
not a claim that the full disk persistence pipeline is implemented.

Current fixture inventory (computed from inputs): **22 pairs**, comprising
**15 state scenarios / 216 operations**, **6 migration inputs**, and **27 option
coercion inputs in one fixture**. Includes a seeded 30-operation sequence,
23 edits followed by 22 undos and 22 redos, and saves with distinguishable
snapshot contents/timestamps beyond the five-entry limit.

Initial green checkpoint gates: Vitest **31 tests / 2 files**, Rust **7 unit
+ 1 fixture interpreter test**, no failed tests; core clippy with `-D warnings`
passed. Final verification and regeneration results are recorded below when
complete.

### Existing state-controller test mapping

| Original `test(...)` | Rust fixture evidence / boundary |
|---|---|
| edits a nested task in place without changing its state or hierarchy | `edit-preserves-state` (value/hierarchy parity; JS reference identity is not a Rust API contract) |
| edits and restores background text and priority without changing other fields | `edit-preserves-state`, `noop-and-redo-invalidation` |
| rejects empty, missing, and normalized no-op edits without history | `noop-and-redo-invalidation` |
| undo and forward restore task completion state | `toggle-parent-propagation`, `noop-and-redo-invalidation` |
| new mutations clear the forward stack | `noop-and-redo-invalidation` |
| delete undo and forward preserve cleaned trigger references | `delete-descendants-and-dangling-triggers` |
| saveTasks persists only the last five auto-save snapshots | `snapshots-five-and-deep-clones` (payload production, not I/O) |
| returns corrupt backup path from loadTasks | Pending persistence adapter |
| returns false when persistence reports save failure | Pending persistence adapter |

## Explicitly pending

- Background trigger activation, expiry scheduling/restart, UI side effects and
  sounds; no renderer workflow or background controller parity claimed.
- Drag/drop intent and placeholder presentation (`getTaskDropOperation`).
- Actual load/save I/O, corrupt-backup passthrough, save-ready guard, failed-save
  handling, atomic writes, data paths, bootstrap, and real Windows data fixture.
- Full arbitrary-JSON persistence/serialization parity. Raw migration handles
  malformed legacy arrays; typed `Task` deserialization intentionally requires
  actual task fields and enums. Raw migration does not imply all malformed
  values become valid typed tasks. Typed serialization uses schema key order,
  so arbitrary original key order on loaded typed tasks is not yet preserved.
- JS numeric formatting outside finite task-domain values, lone-surrogate text
  handling, and general JavaScript coercion are not a compatibility framework.
- Templates, settings/hotkeys, themes, string catalogue and all remaining Phase 1
  areas. This does not satisfy the Phase 1 all-Vitest or real-data acceptance gate.
