# Template UI — runnable development checkpoint

Initial workflow base: `46789aad6c64cb9c9615f0cb4126b565187ce5eb`.
Multiline follow-up base: `e6d0e926792075ace53055ad8e27812472b5b117`,
local unpublished branch `gpui-port-template-multiline`.
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

## Multiline import editor contract

The import editor now opts into `TextInput::multiline(8)`. The default task/name
controls retain their single-line normalization, UTF-16 maxlength and Enter-submit
behavior. There is **one authoritative editable buffer**: file input uses
`set_text`, both clipboard paths use `replace_text_in_range`, and Import passes
`editor.text()` directly to the unchanged core parser. No raw shadow, edit
reconciliation, capture-paste interception or truncated preview remains.

- CRLF and CR normalize to LF, never to spaces. This matches an actual headless
  Chrome `HTMLTextAreaElement.value` probe of the source textarea assignment
  (`templateUi.ts::setTextControlValue`). Literal JSON string newlines still fail
  JSON parsing. Source files are never rewritten.
- Every LF creates a logical display line, including consecutive/trailing empty
  lines. GPUI's pinned `shape_line` owns glyph layout and x hit testing per logical
  line; byte start offsets and line height provide y hit testing. Long lines use
  horizontal scrolling, not wrapping. The eight-row viewport scrolls vertically;
  wheel browsing does not snap back to an unchanged caret. Editing/navigation
  reveals the caret. Only visible lines are painted through the viewport mask.
- Enter inserts LF; the explicit Import button submits. Home/End are line-local;
  Up/Down retain the preferred x position across short lines. Shift extends the
  selection; Ctrl+Home/End (and Shift variants) address the whole buffer.
- Selection, cut/copy/paste and UTF-16 IME ranges span lines and non-BMP text.
  CRLF composition selection offsets are adjusted for the exact normalization.
  The plural bounds API returns per-line rectangles; the singular candidate-UI
  API returns the first rectangle rather than spanning unrelated intervening text.
- No import maxlength is imposed by the editor. The unchanged parser checks the
  complete trimmed buffer at 100000 UTF-16 units. Oversize input remains editable;
  it is not truncated into accepted data.

## Validation (2026-11-10)

All Rust commands used the existing uncommitted local build shim and the newly
created, lane-exclusive target
`CARGO_TARGET_DIR=/home/agent/.hermes/cache/scratch/template-multiline-e6d0-fresh`.
No copied workspace artifacts, dependency/manifest changes or native claims.

- RED: dispatched import paste exposed normalized spaces instead of LF in
  `template_multiline_buffer_is_visible_editable_and_roundtrips`. Separate REDs
  caught missing vertical selection, missing wheel scrolling and CRLF composition
  selection displacement. All now GREEN.
- `cargo fmt --all --check`: pass.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo test --locked --workspace`: **263 passed, 1 existing ignored**;
  UI **88 passed** = all **79 previous tests unchanged** plus **9 new tests**
  (3 application and 6 reusable editor tests). All 14 previous template workflows
  and 65 prior UI tests, including S5 single-line controls, remain intact.
- Tests dispatch real GPUI keyboard, mouse and wheel input. They assert logical
  line layout/height, distinct y coordinates, cross-line selection rectangles,
  scrolled XY hit testing, caret reveal, non-BMP IME replacement, clipboard
  roundtrip, independently editable adjacent spaces/newlines, file/clipboard
  malformed JSON remaining invalid until corrected, and the exact parser limit.
  Import-modal tests exercise Tab/Escape, fixed footer and wheel isolation at
  450×600 / 500×700 logical sizes and scales 1 / 1.5 / 2. These are headless
  layout/event proofs, **not native raster/screenshot or native IME evidence**.
- `cargo check --locked -p echosight --all-targets --target x86_64-pc-windows-msvc`
  and the equivalent `-p echosight-platform` command: pass (cross-check only).
- `cargo build --locked -p echosight -p echosight-platform --example s8_task_app --bin echosight`:
  pass; actual Linux application and S8 binaries linked.
- Unchanged isolated TS/source/golden suites: **134 passed** (themes 15,
  templates 2, settings 16, background 24, theme-library 13, template-ui 11,
  tasks 33, storage 20). Read-only golden flags enabled where supported;
  original TS and golden fixture diffs remain empty.
- Both linked executables launched from an unrelated scratch cwd: no arguments
  exit **2**; explicit missing `--data-dir` opens the task window and stays alive
  until the 3-second timeout (**124**). Neither missing root was created. Logs
  explicitly state **NOT visual evidence**.

Local reproducibility logs: `template-multiline-{red,first-green,clippy,workspace,build}.log`,
`template-multiline-windows-{app,platform}.log`, and `template-multiline-ts-*.log`
under `/home/agent/.hermes/cache/scratch/`. The DOM probe is
`template-newline-dom.html`; the startup cwd is `template-multiline-smoke-2rfrezt1`.
Independent review of this follow-up is pending with the parent lane. The prior
workflow review covered the superseded fail-closed raw-shadow adapter, not this
new multiline implementation.

## Empty-buffer placeholder correction (2026-11-10)

Independent component review found a shared-editor panic, not a template workflow
contract change: with `.multiline(8).placeholder("JSON\nshare code")`, dispatched
Down then Backspace used placeholder byte offset 5 to edit the empty buffer.
The current import placeholder is single-line and did not trigger this case.
`TextLayout.text` identifies the content revision, but its shaped lines display
the placeholder when content is empty. The revision check therefore did not make
those display offsets valid editable positions.

The correction makes vertical navigation a no-op for an empty buffer, matching
its existing mouse-hit-test guard. Nonempty layout/navigation, placeholder
rendering, default single-line controls, template workflows, core contracts and
all other production files remain unchanged from `79582c7610faa87413e6690712fe7ca294bfdc93`.

Validation used `CARGO_BUILD_JOBS=2`, the existing local build shim and separate,
fresh, uncopied targets for the failing and corrected source:
`template-placeholder-red-NgiORS` and `template-placeholder-green-YB7rLj` under
the scratch directory. No dependencies or manifests changed.

- RED: `cargo test --locked -p echosight-ui --lib placeholder -- --nocapture --test-threads=1`
  ran **4 tests, all failing**. Both uninterrupted dispatched Down/Backspace and
  Shift+Down/Backspace reproduced `end byte index 5 is out of bounds for string of
  length 0` at `replace_text_in_range`. The invariant matrix observed `5..5`
  instead of `0..0`; the nonempty control preserved navigation until clearing,
  then observed an invalid `0..5` selection in the empty buffer.
- GREEN: the identical command ran **4 passed, 88 filtered out**. New coverage
  checks Up/Down and both Shift variants, empty/Unicode/multiline placeholders,
  UTF-8 and UTF-16 zero selections, typing and grapheme Backspace after arrows,
  preferred-column movement, forward/reversed cross-line selection, deletion,
  and returning to the placeholder after clearing real content.
- `cargo test --locked --workspace`: **267 passed, 1 existing ignored**;
  UI **92 passed**, including all **88 existing tests unchanged**.
- `cargo fmt --all --check` and
  `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.

Local logs are `template-placeholder-{red,green,fmt,workspace,clippy}.log` under
`/home/agent/.hermes/cache/scratch/`. This is headless dispatched-event evidence,
not native font/raster/IME acceptance. Linked-app and Windows cross-compilation
checks for this correction remain the parent's combined-candidate gate; the
previous revision's checks above are not claimed as this revision's results.
Independent delta review is also left to the parent. No push or merge performed.

## Explicit remaining boundaries

- The editor uses horizontal scrolling for long logical lines, not soft wrapping.
  Native scrollbar cosmetics, native clipboard/IME and pixel-level textarea
  appearance remain acceptance work; multiline raw-buffer editing is implemented.
- Successful operations use the existing status surface, and errors use form/
  template error surfaces; the renderer's shared queued alert service is not
  implemented here. This is not a claim of complete modal-alert parity.
- Native file-picker interaction/overwrite consent, clipboard, IME, DPI and
  screenshots need real Windows/native acceptance. The test platform dispatched
  prompt callbacks; Linux shim startup and selector bounds do not prove these.
- Parent S8/background native acceptance remains pending. No theme/settings UI,
  native overlay, tray/hotkey or audio implementation is included.
- No core template blocker found. No pushes, merges or changes to main.
