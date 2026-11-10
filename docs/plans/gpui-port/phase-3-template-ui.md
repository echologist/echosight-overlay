# Phase 3: runnable template UI

Frozen base: `46789aad6c64cb9c9615f0cb4126b565187ce5eb`; isolated `gpui-port-template-ui`. No pushes, merges, core/data changes or native-acceptance claims.

## Checked source contract

- [x] Read master §2, ADR-001, renderer inventory template/dialog entries and actual renderer wiring. The actual controller is `createTemplateController` (`templateController.ts`), not an imaginary templateWorkflowController.
- [x] `saveTemplateAction` calls `createTemplateFromTasks` / `upsertTemplateByName` without confirmation. `saveTemplateCollection` publishes only after `saveTemplateState` succeeds; select resets on publication.
- [x] `loadTemplateAction` conditionally confirms `hasIncompleteTasks`, clears timers, captures `load template`, instantiates, replaces, renders, saves. Renderer `replaceTasks` migrates, but instantiation already supplies defaults. **No Restart on template load**; undo/redo restart in the background facade.
- [x] `deleteTemplateAction` always danger-confirms and only removes after successful save.
- [x] `createTemplateTransferController` fills the import draft on file selection; imports parse with `parseTemplateImport`, confirm replacement before creating, and close only on success. Export uses actual JSON bytes and filename helpers.
- [x] `createCommunityTemplateController` lists six core definitions, previews, confirms replacement, and stays open after import.
- [x] Native adapter uses core `resolve_template_load` (fallible) and `resolve_template_save`. A failed load is not an empty writable collection. Explicit dev root only; missing directory stays missing at startup.

## Implementation / RED–GREEN commits

1. [x] Add dispatched Save/load/delete tests using real Storage and on-disk reload; first run fails for missing UI. Implement template state/readiness/error separate from tasks, off-thread loading and FIFO successful-publication saves; template bar and bounded dialogs. Load uses existing TaskClock and BackgroundCommand CancelAll/CaptureUndo/Replace/Save.
2. [x] Add dispatched import/community/export/cancel/failure/nested-focus tests before implementation. Wire injected narrow file-picker port with real GPUI open/save prompts and existing native clipboard TextInput paste; import actual bytes, export actual file after save consent. Core alone owns data algorithms. Preserve community modal and top-only Escape.
3. [x] Execute unchanged TS controller workflows in isolated Vitest config; pin side-effect ordering, prompts, silent overwrite, failed publication, selection reset, and community lifecycle. No src modifications.
4. [x] Retain all existing tests; run fmt, locked full clippy/tests, Windows app/platform checks, link app and S8. Smoke from unrelated cwd: no args exit 2; explicit missing dev root stays absent. Report counts from output and native/deferred scope in parity-template-ui.md.
5. [x] Prepare and verify the exact staged allowlist for a conventional local commit as milano <152300831+echologist@users.noreply.github.com>, without trailers. Report the resulting commit SHA in the handoff rather than claiming it exists in this pre-commit plan. Test evidence, source matrix and remaining native gates are recorded in parity-template-ui.md.
