# Phase 1 recovery checkpoint

Status: combined candidate on `gpui-port-phase1-recovery`, not yet approved or landed on `gpui-port`. This is a partial core port and S5 evaluation, not a runnable replacement application.

## Frozen candidates

- Tasks: `ade7ef9596a260fdc8ad0bd0dd03384d94479658`; scope and deferred work in `parity-task-slice.md`.
- Storage: `64134f944faad72d6158e9326e698015707db22d`; scope and compatibility bounds in `parity-storage.md`.
- Theme v1 helpers/validation: `ba959e54888b83414498a1840bfa3b62fc639c21`; scope in `parity-themes.md`.
- S5 input: `59b225e03dff98c7b2822f868bfb373fdd414867`; decision and acceptance script in `adr-002-component-source.md` and `spike-s5.md`.

The original interrupted work was preserved and completed in separate worktrees. Neither `main` nor Electron production sources/configuration, the existing Electron workflow or eye icons changed. No live user-data directory was accessed.

## Parent integration rehearsal, 2026-11-09

- Combined all four frozen candidates in a separate worktree. The only textual conflicts were the appended `echosight-core/src/lib.rs` module exports; retained every module.
- Regenerated the combined lockfile through Cargo (storage adds `ryu-js` to the input lane's lockfile).
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: 81 tests passed (35 core, 23 platform, 23 UI). Counts refer to Rust test functions, not distinct golden cases.
- `cargo check -p echosight-platform --all-targets --target x86_64-pc-windows-msvc`: passed. This is compilation, not Windows execution/linking acceptance.
- Regenerated each oracle on the combined tree using the three isolated `tools/golden/vitest.*.config.ts` files: tasks 31 tests, storage 20 tests, themes 10 tests passed. `git diff --exit-code -- crates/echosight-core/tests/golden` was clean afterward.
- Added-line scan of `crates/` and `tools/`: no secret-literal matches, local agent/shim paths, unsafe blocks or dynamic-eval matches.
- Four independent read-only reviews are pending. Do not push this candidate or claim acceptance until their concrete findings are addressed and affected gates rerun.

Local Linux/Windows cross-check commands require the existing machine-local shim. It is not part of the repository. Build caches are disposable and are not acceptance artifacts.

## Next gates, in order

1. Collect independent reviews, reproduce concrete blockers, add failing regressions, fix, and rerun combined gates. Keep review fixes scoped and preserve source behavior unless Miles explicitly accepts a delta.
2. Review the final combined diff and source/fixture integrity. Publish through a GPUI topic PR, never directly to `main`; no author trailers.
3. Build `s5_input_handrolled` on Windows CI and send the packaged executable plus `spike-s5.md`. Native clipboard, IME candidate placement, DPI and visual results require Miles. Headless tests cannot close those rows.
4. Continue core parity in bounded slices: background triggers/expiry effects; templates/import/export/community data; settings/accelerators/recording normalization; theme filesystem loader/sync/assets/sound resolution; storage/domain integration and shared serialization. Generate each oracle from the real TS functions before implementing Rust behavior.
5. Assemble `parity-core.md` mapping every original Vitest behavior to a Rust test. A large golden-case count is not a substitute for this checklist.
6. Identify the actual packaged Windows data directory or obtain an explicitly selected sanitized copy. Perform read-only doctor inspection and temp-directory round trips before any in-place access. The initially suggested `%APPDATA%\\echosight` location was not found by Miles; no replacement identity is proven.

## Remaining full-parity gates

- No templates/settings/background runtime port in these candidates.
- No task persistence wiring or arbitrary loaded JSON property-order proof.
- Valid JS values outside `serde_json::Value` fail closed without destructive quarantine; full compatibility remains unresolved.
- Theme discovery/sync, containment and delivery, sound resolution, v2 conversion, all bundled visual ports and UI integration remain pending.
- S5 native Windows acceptance, detailed earlier Windows spike rows, and actual Linux desktop checks remain pending.
- The production Rust application and theme CLI remain scaffolds. Do not describe this checkpoint as a usable Echosight 3 build.
