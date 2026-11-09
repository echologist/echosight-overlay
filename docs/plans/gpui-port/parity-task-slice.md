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

Current fixture inventory (computed from inputs): **24 pairs**, comprising
**17 state scenarios / 232 operations**, **6 migration inputs**, and **27 option
coercion inputs in one fixture**. Includes a seeded 30-operation sequence,
23 edits followed by 22 undos and 22 redos, and saves with distinguishable
snapshot contents/timestamps beyond the five-entry limit.

Initial green checkpoint gates: Vitest **31 tests / 2 files**, Rust **7 unit
+ 1 fixture interpreter test**, no failed tests; core clippy with `-D warnings`
passed.

### Initial slice verification (before duplicate-ID regression)

- `cargo fmt --all --check`: exit 0.
- `cargo test -p echosight-core`: exit 0; 7 unit tests and 1 integration test
  passed, 0 failures. The integration test compares every one of the 22 pairs.
- `cargo clippy -p echosight-core --all-targets -- -D warnings`: exit 0.
- Regenerated via the isolated Vitest config: 2 files / 31 tests passed;
  SHA-256 comparison proved **all 44 fixture files unchanged**.
- Aggregate SHA-256 (sorted JSON name-to-SHA256 map, Python `json.dumps` default
  separators): `f1316977384aacdcf13e4a435ebb38c4f5ba928e7b77e1102356e031233090dd`.
- `git diff --check`: exit 0. No Cargo manifest or lockfile change was needed.
- Optional standalone TypeScript typecheck was **blocked before execution** by
  the tool lifecycle scanner: installed `typescript/lib/_tsc.js` exceeds its
  1 MiB scan cap. This is not a typecheck pass. Real TS execution via Vitest
  succeeded; the required Rust gates were rerun separately and passed.
- Independent review/integration belongs to the parent lane. No push or merge,
  Windows runtime validation, full workspace build, or full Phase 1 acceptance
  is claimed by this slice.

### Reviewed duplicate-ID regression

Review of `ade7ef9596a260fdc8ad0bd0dd03384d94479658` found that complete-next
selected a leaf but then re-found it by ID for mutation/result. With repeated
clock/random inputs, that resolves an earlier task instead of the selected leaf.
The fix retains its structural index path through mutation and result cloning.
ID creation and ID-based parent-completion logic are unchanged.

Two new fixtures, `complete-next-duplicate-root-ids` and
`complete-next-duplicate-nested-ids`, execute the unchanged TS controller to add
`First` and `Second` with `nowMs = 1800000000000`, `random = 0.017`. Both receive
ID `1800000000000017`; complete-next twice must return First then Second and
complete both leaves. Each scenario also checks exhausted selection, undo, redo,
and the save payload. The nested fixture puts the leaves in separate branches:
TS's ID-based parent lookup completes the first branch but leaves the second
branch and root incomplete even after both leaves complete. That quirk remains
in the oracle and Rust implementation; following structural parents instead
would be an unauthorized behavior change.

Generator operations now accept explicit `nowMs` and `random` overrides; absent
values retain the original clock schedule/random stream. An explicit random
value is constant for the operation and does not consume the fallback stream.
The Rust adapter reads the same inputs. Expected results were emitted by Vitest,
not manually written.

- **RED before production changes:** `cargo test -p echosight-core --test
  task_parity -- --nocapture` exited 101. Exactly **2/24** fixture pairs failed;
  both reported `$[3].result.task.text: actual "First", expected "Second"`.
- **GREEN:** `cargo test -p echosight-core` exited 0 (7 unit tests and 1 fixture
  interpreter test; all 24 pairs match structured values and exact bytes).
- `cargo fmt --all --check` and
  `cargo clippy -p echosight-core --all-targets -- -D warnings`: exit 0.
- Isolated TS execution: **33 tests / 2 files passed** on both generation runs.
  SHA-256 comparison verified all **44 original fixture files unchanged** and
  all **48 current files byte-identical after regeneration**.
- Current aggregate SHA-256 (same sorted name-to-hash JSON method as above):
  `781a59c9ee458fba99d2ca95f2b19dd7ddce6f19d0714ed90e90dd7a548642c0`.
- Oracle provenance: unchanged TS sources from the reviewed revision above.
  `taskCompletion.ts` SHA-256:
  `f80cd1813629622e1d797659baddd17551db1bc51d5d3745563c480d163cea94`;
  updated `tools/golden/tasks.gen.test.ts` SHA-256:
  `20ed8608aee34ac74f05f62484d6dbc5490d8c84dc9a11348b2e1e1ee9301370`.

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
