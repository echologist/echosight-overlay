# Template UI — runnable development checkpoint

Base: `46789aad6c64cb9c9615f0cb4126b565187ce5eb`, branch `gpui-port-template-ui`.
The real `echosight` and `s8_task_app` now expose template Save/Load/Delete,
file/clipboard Import, native-save Export and all six community definitions.
This is a working workflow checkpoint, **not complete native/visual parity**.
Core algorithms, renderer source, data/defaults and golden fixtures are unchanged.

## Source-contract matrix

Authority is inventory sections 4–8/11 plus the actual renderer controllers;
there is no `templateWorkflowController` in this source tree.

| Actual source / function | GPUI adapter / observed contract | Evidence |
| --- | --- | --- |
| `templateController.ts`: `saveTemplateAction`, `saveTemplateCollection`; core `create_template_from_tasks`, `upsert_template_by_name`, `resolve_template_save` | Save incomplete tasks only, preorder trigger indices; same-name Save overwrites silently. Publish and reset selection only after successful disk save. | Dispatched save/reopen/load, overwrite and rejected-save/FIFO tests; real renderer tests. |
| `templateController.ts`: `loadTemplateAction`; `taskWorkflowController.ts`: `replaceTasks` | Confirm only if incomplete tasks; `CancelAll`, `CaptureUndo("load template")`, fresh-ID instantiation, `Replace`, explicit `Save`. Set currentTemplate. **No Restart on template load**; undo/redo use existing background facade restart. | Nested triggers, currentTemplate on disk/reload/history; actual TS timer/history ordering. |
| `templateController.ts`: `deleteTemplateAction` | Always danger-confirm. Cancel/Enter do not delete; successful persistence precedes removal. | Dispatched cancel then confirm and renderer workflow tests. |
| `templateController.ts`: selected ID lookup | Preserve falsy zero ID and duplicate-ID-first-match quirks. | Dispatched and actual-controller tests. |
| `templateTransferController.ts`: `importTemplate`, `handleFileImport`; core `parse_template_import` | File picker reads bytes into draft, never overwrites source. Clipboard inserts at selection. Core validates empty/UTF-16 size/shape/valid tasks. Same-name import asks exactly `Template "<name>" already exists. Replace it?`. | Native test-platform prompt dispatch; disk source unchanged; malformed/oversize/empty input; nested cancel/retry. |
| `templateTransferController.ts`: `exportTemplate`; core export payload/filename functions | Version 2.0 iff background tasks; `poe2-template-<slug>.json`; actual pretty JSON file written only after native save prompt returns a path. Canceled prompt writes nothing. | File bytes, v2 nested background tasks, reimport; filename/picker errors; actual TS export spy. |
| `communityTemplateController.ts`: `createCommunityTemplateController`; core `COMMUNITY_TEMPLATES` | All six definitions, preview and same-name confirmation; community remains open after import. | All-six dispatched imports, replacement and preview; real TS lifecycle tests. |
| core `resolve_template_load` / `resolve_template_save` | Separate `Option<collection>` readiness and template error channel; failed template load never becomes writable empty state. Real Storage async FIFO saves, failed write retains previous published collection. | Malformed and normalization-failure files unchanged; task edits still work; preload guard, explicit-dev-root missing directory and failure/retry tests. |
| renderer `dialogService.ts`; existing GPUI modal stack | Bounded scrolling and fixed footer, Tab trap, top-only Escape, danger Cancel/no bare Enter, opening modal cancels drag; expiry can update tasks behind nested modal. | Dispatched expiry/preview focus restore and footer bounds; inherited task tests retained. |

`ports.rs` is only the injectable prompt seam. Normal app uses GPUI
`prompt_for_paths` / `prompt_for_new_path` and the native clipboard; it is not a
fake export event. Quit waits for template persistence and active native transfer.
The existing anchored `TaskClock` supplies template IDs and export time.

## Validation (2026-11-10)

All Rust commands used the local uncommitted environment shim and a separate
`CARGO_TARGET_DIR=/home/agent/.hermes/cache/scratch/template-ui-46789-fresh`.

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo test --locked --workspace`: **254 passed, 1 existing ignored**;
  UI **79 passed**, including **14 new dispatched template tests**.
  All 240 pre-existing tests retained; existing task and
  background test files are unchanged.
- `cargo check --locked -p echosight -p echosight-platform --all-targets --target x86_64-pc-windows-msvc`: pass. This is cross-checking, not Windows linking/execution.
- `cargo build --locked -p echosight -p echosight-platform --example s8_task_app --bin echosight`: pass; actual Linux app and S8 binaries linked.
- Isolated Vitest source/golden suites: **134 passed**: themes 15, templates 2,
  settings 16, background 24, theme-library 13, new template-ui 11, tasks 33,
  storage 20. Golden check flags enabled; golden fixture diff empty.
- Both linked executables launched from an unrelated scratch cwd: no arguments
  exit **2**; explicit missing `--data-dir` opens the task window and stays alive
  until the 3-second timeout (**124**). Neither missing root was created.
  Headless logs explicitly state **NOT visual evidence**.

Repro logs (local scratch, not committed): `template-ui-workspace.log`,
`template-ui-ts-*.log`, `template-ui-windows-{app,platform}.log`, and
`template-ui-final-smoke-308pdhix/results.json` under
`/home/agent/.hermes/cache/scratch/`.

Independent read-only review initially caught multiline input being normalized
into valid JSON after an ordinary edit. A failing dispatched regression was added
before fixing it. The adapter now retains raw input through typing, deletion and
subsequent paste using post-edit UTF-16 caret/IME range offsets; it fails closed
rather than parsing normalized fallback data. Regression covers both actual file
read and clipboard, non-BMP text, adjacent space/newline deletion, corrective
newline deletion, and unchanged source-file bytes. Follow-up independent review
passed with no security or logic blockers; extra IME/mismatched-suffix regression
coverage was a nonblocking suggestion. Review reports: `template-ui-review.json`
and `template-ui-review-fixed.json` in the same local scratch directory.

## Explicit remaining boundaries

- The import control still **renders one line**, not the renderer's full textarea.
  Raw CR/LF are retained separately. Replacing a visually identical normalized
  space cannot express every newline edit; complex unreconcilable edits fail
  closed and require reloading the draft. A dedicated raw multiline control is
  the remaining UI parity work, not a core parser change.
- Successful operations use the existing status surface, and errors use form/
  template error surfaces; the renderer's shared queued alert service is not
  implemented here. This is not a claim of complete modal-alert parity.
- Native file-picker interaction/overwrite consent, clipboard, IME, DPI and
  screenshots need real Windows/native acceptance. The test platform dispatched
  prompt callbacks; Linux shim startup and selector bounds do not prove these.
- Parent S8/background native acceptance remains pending. No theme/settings UI,
  native overlay, tray/hotkey or audio implementation is included.
- No core template blocker found. No pushes, merges or changes to main.
