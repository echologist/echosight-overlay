# Template domain parity

Status: **core domain implemented and tested; GPUI/application wiring pending**.
This is not the finished application and does not complete the whole Phase 1 gate.
Baseline: `98af9e5`, topic branch `gpui-port-templates`.

## Implemented contract

`crates/echosight-core/src/template/` ports the pure behavior from
`taskTemplates.ts`, `templateOperations.ts`, `templatePersistence.ts`, and
`communityTemplates.ts`. Runtime trees reuse `task::model::Task`, `TaskId`,
`TaskMode`, `BackgroundOptions`, and injected `TaskEnvironment`.

- Normalize strings/legacy saved records, name limit 50 and task-text limit 200
  UTF-16 units, ECMAScript trimming, default name, numeric ID coercion/fallback,
  timestamps, strict boolean imported/community flags, and collection filtering.
- Save incomplete tasks only. A completed parent removes its entire subtree.
  Trigger indices use flat incomplete preorder, omit missing/completed targets,
  retain duplicate triggers, and resolve duplicate source IDs to the last index.
- Instantiate fresh runtime IDs in preorder, including grandchildren. Inherit
  background mode, reset completed/activation state, sanitize background options,
  and remap trigger indices by structural position rather than generated ID.
  Repeated clock/random values intentionally produce collisions, not repaired IDs.
- Name-based upsert removes **all** matching names and appends the replacement;
  name/ID lookup returns the first match; deletion removes all matching IDs.
- Import trims before the 100000 UTF-16-unit size check, uses the existing strict
  destr-compatible storage parser, validates shape, and requires one valid task.
  Unknown import metadata is ignored; task triggers are coerced/deduplicated.
- Export includes recursive task count, timestamp and description, uses format
  `2.0` iff any descendant is background (otherwise `1.0`), and retains the
  `poe2-template-<slug>.json` naming quirk. `export_template` returns both typed
  data and exact two-space/LF/no-final-newline bytes via the storage formatter.
- All six community records, descriptions, task strings, explicit empty child
  arrays, flags, and numbered previews are ported: League Start Essentials 8;
  Endgame Progression 9; New Character Setup 8; Currency Goals 8; HC/SSF
  Priorities 8; Crafting Checklist 8.
- Persistence result adapters return load fallbacks/save booleans plus the
  source diagnostic prefixes/messages, without doing I/O or publishing UI state.

Preserved quirks include empty templates saved when every task is complete
(the next normalization drops them), community empty children omitted on reload,
verbatim save names versus normalized import names, `Date.now()` template IDs
versus randomized runtime/fallback IDs, and fractional positive expiry values
flooring to zero during normalization then becoming null during instantiation.
Template nodes retain field order and absent versus null/empty optional fields
for exact export bytes; unknown task extensions survive typed round trips but
are deliberately discarded by the source-compatible normalizer.

## Integration API

Import modules from `echosight_core::template::{model, normalize, ops, transfer,
community, persistence}`.

1. Load `Storage::load(FileKind::Templates)` as raw JSON; pass successful values
   to `resolve_template_load` / `normalize_template_collection` with the shared
   environment. Do not directly deserialize legacy JSON into `TaskTemplate`.
2. Save: `create_template_from_tasks`, then `upsert_template_by_name` silently.
   For import: `parse_template_import`, then `replacement_prompt` using its
   normalized name. For community: index `COMMUNITY_TEMPLATES` safely, then ask
   the same prompt using the community name. Cancel by discarding the proposal;
   only create the imported/community template after confirmation, so cancellation
   consumes no clock/entropy. `create_imported_template` and
   `create_community_template` set their respective flags.
3. Persist the proposed collection, resolve the save result, and publish it to
   application state **only on success**. `resolve_template_save` also accepts a
   transport rejection. The existing Storage owns atomic writes/load-failure
   blocking, not this domain module. A tempfile roundtrip integration test
   exercises the existing store without using any real user directory.
4. Loading uses `instantiate_template_tasks` and the template name. The shell
   still owns confirmation, clearing background timers, capturing undo with
   `load template`, replacing the task state/current-template name, rendering,
   task persistence, and focus restoration. Selection parsing deliberately
   returns numeric zero; source controllers subsequently treat zero as no
   selection, so UI wiring must preserve that separate truthiness check.
5. Export writes `TemplateExport.bytes` with the suggested filename only after
   the native dialog choice. No picker, clipboard, download, or filesystem side
   effect is hidden in the pure API.

Still pending: GPUI views/dialog queue/strings, selection and modal behavior,
real storage/application startup wiring, native import/export file dialogs,
undo/timer/focus integration, and authorized real-user-data acceptance. Do not
interpret successful core tests as evidence for any of these runtime gates.

## Evidence and reproducibility

`tools/golden/templates.gen.test.ts` imports the unchanged production TS modules
and installed destr. Inputs are authored; expected data is obtained only by
executing those functions, with immediate JSON deep clones, frozen Date and
explicit entropy sequences (including all-repeated entropy). The fixture records
original-path SHA-256 hashes for source/dependency/generator/config provenance.

- RED checkpoint `75e30b8`: compiling fixture adapter, **20 failed / 0 passed**
  before production implementation.
- Current evidence: **153 golden operation cases**, plus **3 syntax-rejection**
  observations and **4 explicitly unsupported JS-value** observations.
- Rust: **27 template test functions** (20 golden groups, 7 integration guards).
  Full `cargo test --locked -p echosight-core`: **63 passed, 0 failed**.
- TypeScript isolated config: **2 passed, 0 failed**. Freshness mode and two
  explicit regeneration runs produce identical fixture bytes.
- `cargo fmt --all --check`, core all-target clippy with `-D warnings`, project
  TypeScript typecheck and a separate generator/config typecheck pass.
- Existing task/storage/theme fixtures, production Electron sources, manifests,
  dependencies and lockfiles are unchanged.

```sh
# Read-only provenance/expected-byte check BEFORE any regeneration:
TEMPLATES_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.templates.config.ts
# Explicit regeneration, only when intentionally updating source expectations:
npx vitest run --config tools/golden/vitest.templates.config.ts
cargo test --locked -p echosight-core
cargo fmt --all --check
cargo clippy --locked -p echosight-core --all-targets -- -D warnings
npx tsc --noEmit
npx tsc --ignoreConfig --noEmit --target ES2022 --module ESNext \
  --moduleResolution Bundler --strict --skipLibCheck \
  tools/golden/templates.gen.test.ts tools/golden/vitest.templates.config.ts
```

## Explicit compatibility boundaries

The raw boundary is the existing JSON `Value` compatibility slice. Lone UTF-16
surrogates (including splitting one at a text limit), nonfinite nested JSON
numbers, and parser recursion-limit inputs return `TemplateError::Unsupported`
instead of silently replacing/truncating data or treating it as corrupt. Even
an unused nonfinite import field can therefore fail closed. The oracle records
real TS acceptance separately; these are **not** claimed as parity passes.
Top-level destr `undefined`/NaN/infinity values are recognizable shape failures,
so they return the source `Invalid JSON template format` error.

Stable domain/destr errors are matched exactly. JSON syntax is rejected, but
serde parser diagnostic text is not fabricated to imitate a particular V8
version's `JSON.parse` wording. Exact engine-specific syntax-error text remains
an explicit compatibility gap. Typed runtime APIs expect normalized template
nodes; legacy string nodes enter through normalization, not unchecked typed
serde decoding. Arbitrary JS objects/functions/custom coercion are not a native
storage contract.
