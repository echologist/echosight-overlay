# Echosight: Electron to GPUI Port. Implementation Plan

> **For Hermes:** Execute phase by phase with `subagent-driven-development` (fresh subagent per task, spec-compliance review then code-quality review). Opus-class planning per phase, Sonnet-class implementation. Independent review for every lifecycle/window/UI task. This document is the master plan; Phases 1 to 6 each get a detailed bite-sized sub-plan (`docs/plans/gpui-port/phase-N-*.md`) written when that phase starts, because each depends on spike results.

**Goal:** Ship Echosight v3.0.0 as a native Rust/GPUI desktop overlay with zero loss of user-facing functionality versus v2.1.1 (Electron), reading users' existing data in place, with all 11 bundled themes ported to a new CSS-free Theme Format v2.

**Architecture:** Strangler port. (1) Pure-Rust `echosight-core` crate re-implements every domain rule (tasks, undo/redo, templates, settings, hotkeys, storage, bootstrap, theme loading/validation/sync, sound mixing) and is proven against golden fixtures generated from the TypeScript code. (2) `echosight-theme` defines Theme Format v2 (tokens + closed slot/state style DSL + images + sounds) plus a v1 to v2 converter; the 11 bundled themes are converted or re-authored. (3) `echosight-platform` owns the overlay window FFI (click-through, topmost, show/hide/move), global hotkeys, game monitor, single instance, audio, tray. (4) `echosight-ui` renders everything in GPUI with a small in-house component set (or gpui-component, decided by spike). Electron remains shippable from `main` until the port passes the parity gate; the port lives on branch `gpui-port`.

**Tech stack:** Rust (stable, pinned via `rust-toolchain.toml`), GPUI (git-pinned zed rev or `gpui-pre` snapshot, decided in Phase 0), serde/serde_json, `global-hotkey`, `sysinfo`, `rodio`, `tray-icon`, `dirs`, `raw-window-handle` + `windows`/`objc2`/`x11rb` for FFI, `insta` for golden tests, GitHub Actions (windows-latest, ubuntu-latest, macos-latest).

---

## 0. Decisions already taken (from Miles, 2026-11-09)

| # | Decision | Consequence |
|---|---|---|
| D1 | Theme Format v2 = JSON tokens + per-component style DSL. Raw CSS (`cssFile`, `customCSS`) is deprecated. | All 11 bundled themes are ported into v2 (7 via converter + review, 4 CSS themes re-authored by hand). Old v1 themes still load (token part only) with a visible warning. |
| D2 | Strangler strategy: core first with golden tests, then GPUI shell. Electron stays shippable. | `main` keeps the Electron app until the parity gate. No Electron code is deleted before Phase 7. |
| D3 | Windows parity first, Linux as dev platform, macOS last. | Phase 0 spikes run on Windows (Miles's machine) and Linux X11 (agent box). macOS gated to Phase 5/6 via CI builds + a later manual pass. |
| D4 | Miles can smoke-test on Windows himself. | Every window/hotkey/click-through task ends with a Windows manual check script, not just CI. |
| D5 | New branch, never direct to `main`. Plain commits, **no `Co-Authored-By` trailer**. | Integration branch `gpui-port` off `origin/main`; per-phase PRs target `gpui-port`; one final PR `gpui-port` to `main`. |

## 1. Baseline and evidence (frozen)

- Repo: `https://github.com/echologist/echosight-overlay`, local `/home/agent/code/echosight-overlay`.
- **Port baseline = `origin/main` @ `d96c2d5` (v2.1.1).** It contains: Windows-only tray (`src/main/tray/appTray.ts`: Show / Hide / Exit, tooltip `Echosight`, left-click = show, icon `assets/tray-icon.ico`), simplified theme sync (legacy single-file to folder migration removed), `CHANGELOG.md`.
- `feat/exiles-corruption-sounds` @ `5a63e5c` (folder theme with 4 WAVs) was **discarded by Miles on 2026-11-09** (PR #15 closed, branch deleted). `exiles_corruption` stays the single-file JSON theme on `main`; no bundled theme ships sounds except winamp-classic (`complete.wav`).
- Local worktree note: the 172 "modified" files are file-mode-only changes caused by filesystem ACLs (`git diff --stat` shows 0 insertions/deletions; `core.fileMode=false` shows clean). Do not commit them. Use `git -c core.fileMode=false` or set it in the new worktree.
- Baseline gates verified on 2026-11-09: `tsc --noEmit` clean, Vitest 34 files / 145 tests pass (on 5a63e5c; main has +appTray tests).
- Four evidence reports (every claim cited `file:line`, Verified vs Inferred) live next to this plan and must be copied into the repo in Task 0.4:
  - `echosight-gpui-port/echosight-renderer-inventory.md` (UI behaviours, strings, persistence contract, 19 browser-only dependencies, test checklist)
  - `echosight-gpui-port/echosight-theme-inventory.md` (theme format as implemented, CSS pipeline, per-theme mechanisms, v2 proposal, survival estimates)
  - `echosight-gpui-port/echosight-platform-inventory.md` (data paths, atomic JSON storage, bootstrap/migrations, IPC contract, hotkeys, lifecycle, menu, placement, CI/release, 15-item compatibility checklist)
  - `echosight-gpui-port/gpui-capability-research.md` (GPUI verdict table against zed `4ed72e0`, crate ecosystem, risks)
- Prior art: LLM Wiki `wiki/projects/echosight/Echosight Architecture And Maintenance Baseline.md` (Sept 2026 evaluation recommended Tauri 2 over a native Rust UI; Miles has now chosen GPUI knowingly. The risk list from that evaluation still applies: packaged Windows click-through, topmost over borderless games, focus restoration, hotkeys, data-path compatibility).

## 2. Hard constraints the port must honour (parity contract)

Everything below is Verified in the inventories unless marked.

### 2.1 Data compatibility (no user loses data)
1. Same directory: `<appData>/echosight/data/` (`%APPDATA%\echosight\data`, `~/Library/Application Support/echosight/data`, `~/.config/echosight/data`). Same files: `current_tasks.json`, `templates.json`, `settings.json`, `themes/`. (Path is Inferred from Electron semantics; **Task 0.6 verifies it on Miles's Windows machine before anything else.**)
2. Serialization: 2-space indent, LF, no trailing newline, UTF-8 no BOM, key insertion order preserved (`serde_json` with `preserve_order` + custom pretty printer), f64 ids round-trip (`1749938954072.5596`).
3. Atomic write: temp `.<name>.<pid>.<uuid>.tmp` in same dir, exclusive create, fsync, rename; per-path write serialization; destination untouched on failure.
4. Parse leniency mirrors `destr` strict: trimmed input, bare `null/true/false/number` accepted at top level, empty/truncated = corrupt, `__proto__`/`constructor` keys = corrupt.
5. Quarantine: copy bytes to `<file>.corrupt-<ISO ts with : and . replaced by ->`, write fallback (`{tasks:[],currentTemplate:null,snapshots:[]}` / `[]` / `null`), return `corruptBackupPath` only for tasks; block saves after failed load until a load succeeds.
6. Bootstrap: create-if-missing only; seed tasks/templates from bundled defaults; settings from code defaults (v2, transparency 70, theme `echosight`, sounds off/60, `CommandOrControl+Shift+T/I/N/Z/Y`).
7. Settings tolerance: missing `settingsVersion`, legacy `backgroundColor` to `theme`, missing hotkeys filled, clamp 0..100 and round, hotkeys in any of `CommandOrControl|Ctrl|Control|Cmd|Command` spellings, persist normalized v2 only when changed.
8. Tasks: tolerate legacy tasks lacking `children/mode/triggers/activated/activatedAt/backgroundOptions`; snapshots capped at 5 newest; `currentTemplate` nullable.
9. Themes dir sync: overwrite a user copy only when bundled `id`+`author` match and bundled `version` is strictly greater (numeric dotted compare, unparsable = equal). v2 bundled themes get a bumped version so they replace the v1 copies.
10. Second launch reveals the running overlay and clears "manually hidden".

### 2.2 Behaviour (from renderer inventory sections 4 to 8, 11, 12)
- Task tree: ids `now_ms*1000 + rand(0..999)` (f64), subtasks inherit mode, background children activated iff parent activated, delete strips descendant ids from all `triggers`, no cross-list moves, no move into own subtree, legacy migration fills defaults and triggers a save.
- Undo/redo: 20 deep per stack, deep-clone snapshots of `{tasks,currentTemplate}`, redo cleared on new mutation, 10 labelled operations, undo/redo restart expiry timers and play `undo`/`redo` sounds.
- Progress: counts main tasks (all depths) + activated background tasks (all depths); `completed/total*100`.
- Complete-next: first incomplete main task in pre-order; recursive completion of descendants; activates triggered background tasks; `taskCompleted` sound; 2 s "✓ Completed: <t>" feedback.
- Background tasks: activation on trigger completion, `expiresAfterMinutes` timers keyed by id, `priority` badge `HIGH`, toast `N background task(s) activated!`, `backgroundActivated` sound. Dormant tasks visible only in Configure Triggers.
- Templates: incomplete tasks only, flat pre-order `triggerIndices`, fresh ids on load, upsert by name (Save overwrites silently; import/community confirm `Template "<n>" already exists. Replace it?`), export version `2.0` iff any background task, filename `poe2-template-<slug>.json`, import limits (empty, >100000 chars, shape, >=1 valid task), 6 community templates (League Start Essentials 8, Endgame Progression 9, New Character Setup 8, Currency Goals 8, HC/SSF Priorities 8, Crafting Checklist 8).
- Drag and drop: midpoint/half-width intent (reorder vs make-subtask vs promote), nearest-row snapping within 50 px, placeholder row, intent labels `Drop here to reorder` / `Drop to make subtask` / `Make subtask of target` / `Reorder within same level` / `Promote to higher level` / `Reorder as sibling`.
- Dialogs: single queued dialog; danger confirms focus Cancel and ignore bare Enter; Escape always cancels; Enter on a focused control is not confirm; modal stack with Tab trap; exact titles and strings from inventory section 11.
- Settings: transparency slider (disabled with `Not supported by this theme` when `supportsTransparency:false`), theme chooser dialog (preview, details, swatches, Apply disabled when current), sounds enabled + volume, 5 hotkey recorders (accept primary modifier, Alt, or F1..F12; duplicate rejection; recording suspends global hotkeys), Reset hotkeys confirm, Open themes folder, Reload themes (`Themes reloaded! Found N themes.`), Save / Cancel (Cancel does not revert live preview: keep as-is for parity, log as a follow-up issue).
- Overlay chrome: header drag region, `(Interactive Mode - <hk> to exit)` / `<hk> to edit` hints, minimize (hide + manuallyHidden), reset position, toggle interactive, quit confirm `Quit Echosight`.
- Click-through mode: header, template section, delete buttons, checkboxes hidden; container background per theme `clickThrough` slot; mouse passes through.
- Window: 500x700, top-right of primary work area inset 20 px, min 450x600, frameless, transparent, topmost, skip taskbar, all workspaces, not minimizable/maximizable; geometry not persisted.
- Global hotkeys: 5 actions; toggle-visibility works while hidden, the other four only when visible; re-registered on change; suspended during recording; registration result logged.
- Game monitor: every 3 s, process name containing `pathofexile` (case/punctuation-insensitive); start = show + interactive unless manually hidden; stop = hide and reset flags; still-running = re-assert topmost.
- Lifecycle: single instance, quit on all-windows-closed except macOS, macOS `activate` reopens, cleanup unregisters hotkeys/stops monitor/destroys tray.
- Windows tray (v2.1.1): Show / Hide / Exit, tooltip `Echosight`, left-click shows. (macOS/Linux: none today; the port may enable it everywhere later, out of scope for parity.)
- Sounds: 4 events, gate `settings.sounds.enabled && theme.sounds.enabled != false`, volume `clamp(settings/100, .6) * clamp(theme, 1) * event`, wav/ogg/mp3/m4a, missing files ignored.
- Branding: keep `assets/icon.png` (the Echosight eye) and `tray-icon.ico` unchanged.

### 2.3 Accepted deltas (the only intentional behaviour changes; each needs a CHANGELOG line)
| Delta | Why | Mitigation |
|---|---|---|
| `cssFile` / `customCSS` no longer applied | No CSS engine (D1) | v2 converter + per-rule diagnostics; v1 themes load tokens-only with warning `This theme uses CSS, which Echosight 3 does not support. Run the converter.` |
| Backdrop blur per element (glass, echosight/exiles/win7 accents) | GPUI has no backdrop-filter | `effects.windowBlur: "system"` maps to `WindowBackgroundAppearance::Blurred` (Win acrylic / mac vibrancy / KDE). Glass becomes "window-level blur". |
| `text-shadow` / `filter` glows (cs16, echosight, exiles, winamp) | not in GPUI | `text.shadow` slot property rendered as an offset duplicate text run (1 px hard shadows reproduce exactly; soft glows approximate). |
| `@import` Google Fonts (echosight) | no network CSS | Bundle Cinzel, Inter, JetBrains Mono (OFL) in the binary; theme refers to them by family name. |
| Wayland: no global hotkeys, topmost only on wlr-layer-shell compositors | `global-hotkey` and GPUI limits | Document; provide `ECHOSIGHT_FORCE_X11=1` (run under XWayland) which restores Electron-era behaviour. |
| Native alert/confirm look | GPUI prompts are in-window on Linux anyway; today they are already in-window HTML | None (same UX class). |

Quirks to **preserve** for parity (fix later as issues, not in this port): confirm texts hard-code `Ctrl+Shift+Z`; Settings Cancel does not revert live preview; Save Template silently overwrites; no UI for `expiresAfterMinutes`; `poe2-template-` export prefix.

## 3. Target repository layout (on branch `gpui-port`)

```
echosight-overlay/
  Cargo.toml                  # workspace
  rust-toolchain.toml         # channel = "1.98.1" (match pinned GPUI rev; bump together)
  .cargo/config.toml          # [target.x86_64-pc-windows-msvc] rustflags for subsystem windows in release
  crates/
    echosight-core/           # no GPUI, no FFI. Domain + storage + theme loading/validation/sync + converter
      src/{lib.rs, task/{model,ids,tree,mutations,completion,background,migration,progress,state (undo/redo/snapshots)}.rs,
           template/{model,ops,transfer,community}.rs, settings/{model,normalize,hotkeys}.rs,
           storage/{json,atomic,quarantine,paths,bootstrap}.rs, theme/{v1,v2,validate,library,sync,assets,sounds,convert}.rs,
           window/placement.rs, strings.rs}
      tests/golden/           # fixtures generated from TS (see Phase 1)
    echosight-theme-cli/      # `echosight-theme convert|validate|list` (thin bin over core)
    echosight-platform/       # window FFI (click-through, topmost, show/hide/move), global hotkeys, game monitor,
                              # single instance, audio, tray, app dirs
    echosight-ui/             # GPUI views + components + theme-to-style mapping + preview
    echosight/                # bin: wiring, main.rs, embedded assets (fonts, bundled data, icons)
  data/themes/                # bundled themes, now v2 (same folder, bumped versions)
  assets/{icon.png, icon.icns, tray-icon.ico, fonts/}
  docs/themes.md              # rewritten for v2; docs/themes-v1-migration.md
  docs/plans/gpui-port/       # this plan + evidence reports + phase sub-plans + spike reports
  .github/workflows/rust.yml  # new; build.yml (Electron) untouched until Phase 7
  src/, tests/, package.json  # Electron app, untouched until Phase 7
```

## 4. Phases, milestones, gates

| Phase | Milestone | Gate to pass before next phase | Rough effort |
|---|---|---|---|
| 0 | Branch, workspace, **Windows + Linux spikes** (S1..S7), GPUI source decision, ADRs | All go/no-go spikes green on Windows (Miles) and X11 (agent) | 1 to 2 weeks |
| 1 | `echosight-core` at parity with golden tests | 100% of ported Vitest behaviours green + golden fixtures byte-equal; reads a real v2.1.1 data dir | 2 to 3 weeks |
| 2 | Theme Format v2 spec + converter + 11 bundled themes ported | Every bundled theme validates as v2; converter diagnostics reviewed; side-by-side screenshots approved by Miles | 2 weeks |
| 3 | `echosight-ui` feature complete (headless GPUI tests) | Renderer inventory section 12 checklist 100%; string catalogue 100% | 4 to 6 weeks |
| 4 | `echosight-platform` + app wiring | Windows manual QA script 100%; Linux X11 90% | 2 to 3 weeks |
| 5 | Parity verification + perf + macOS pass | Parity matrix signed off; budgets met | 1 to 2 weeks |
| 6 | Packaging, CI, release pipeline for Rust | Artifacts produced from a `v3.0.0-rc.1` tag on all 3 OS; release invariants preserved | 1 to 2 weeks |
| 7 | Cutover: remove Electron, docs, CHANGELOG, wiki, PR to `main`, v3.0.0 | PR review + Windows smoke on the released binary | 1 week |

Effort is for agent-driven execution with Miles reviewing; wall-clock depends on review cadence. Phase 3 is the long pole.

---

## Phase 0: Branch, workspace, spikes, decisions

### Task 0.1: Create the integration branch in a separate worktree
**Objective:** isolate port work from the dirty local checkout.
```bash
cd /home/agent/code/echosight-overlay
git fetch origin
git worktree add -b gpui-port /home/agent/code/echosight-gpui origin/main
cd /home/agent/code/echosight-gpui && git config core.fileMode false
git log --oneline -1   # expect d96c2d5
```
Branch protection (Miles, GitHub UI): require PR for `main`; `gpui-port` accepts PRs from `gpui-port/<topic>` branches. Commit message convention: `feat(core): ...`, `feat(ui): ...`, `spike(win): ...`, `docs(port): ...`. **No trailers.**

### Task 0.2: Sounds branch decision (done)
Miles discarded `feat/exiles-corruption-sounds` (PR #15 closed). Theme inventory is frozen at `main` d96c2d5: 7 single-file JSON themes + 4 folder themes.

### Task 0.3: Workspace skeleton
Files: `Cargo.toml` (workspace, `resolver = "3"`, members above, `[workspace.package] version = "3.0.0-dev" license = "MIT" edition = "2024"`), `rust-toolchain.toml`, `.gitignore` (+`/target`), empty crates with `lib.rs` and one passing test each.
Verify: `cargo build --workspace && cargo test --workspace` on Linux; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`.
Commit: `chore(port): add Rust workspace skeleton`.

### Task 0.4: Copy evidence into the repo
Create `docs/plans/gpui-port/` with this plan as `00-master-plan.md` and the four inventories. Commit `docs(port): add GPUI port master plan and evidence reports`.

### Task 0.5: Pin the GPUI source (ADR-001)
Options (from the research report): (a) git dependency `gpui_platform` + `gpui` on a zed rev; (b) `gpui-pre` / `gpui-pre-platform` crates.io snapshots (weekly, Longbridge); (c) `gpui-ce` fork.
Procedure: try (a) and (b) in `crates/echosight-ui` hello-world; record clean build time, incremental build time, binary size (release, stripped), and whether `examples/window_shadow.rs`-style CSD works. Write `docs/plans/gpui-port/adr-001-gpui-source.md`. Default recommendation: **(a) git rev pinned in `Cargo.toml` with `[patch]`-free exact `rev`**, refreshed deliberately once per phase, unless gpui-component is adopted (then its `gpui-pre` pin wins, ADR-002).

### Task 0.6: Verify the real Windows data path (Miles, 5 minutes)
On the Windows machine with Echosight 2.1.1 installed: confirm `%APPDATA%\echosight\data\current_tasks.json` exists and zip the whole `echosight` folder as `fixtures/windows-userdata-v2.1.1.zip` (strip personal task text if wanted). This becomes a Phase 1 golden input. If the folder has a different name, section 2.1 item 1 changes and a path-migration task is added.

### Spikes S1..S7 (go/no-go; each is a tiny bin under `crates/echosight-platform/examples/`)

Every spike produces `docs/plans/gpui-port/spike-SN.md` with: zed rev, OS build, exact observations, screenshots/video, verdict. Windows spikes are run by Miles from a `cargo build --release` artifact produced in CI (`workflow_dispatch`), so no Rust toolchain is required on his machine; include that workflow in Task 0.3 (`.github/workflows/rust.yml` with a `spike` job building `--example sN`).

| Spike | Question | Mechanism to test | Acceptance (Windows unless stated) |
|---|---|---|---|
| **S1 window** | Frameless, transparent, topmost, skip-taskbar, resizable with min size, header drag | `WindowOptions { titlebar: None, kind: WindowKind::PopUp, window_background: Transparent, is_resizable: true, window_min_size: 450x600, window_bounds: Windowed(top-right calc) }`, `div().window_control_area(WindowControlArea::Drag)` | Appears top-right inset 20 px; per-pixel alpha visible over desktop; stays above a borderless-fullscreen PoE (or any borderless game) and above Explorer; no taskbar button; drag by header; edge resize works **with `PopUp` style 0** (research flags this unknown; if resize fails, test `WindowKind::Normal` + manual `SetWindowPos(HWND_TOPMOST)` + `WS_EX_TOOLWINDOW` via raw handle) |
| **S2 click-through** | Runtime toggle of mouse passthrough while GPUI keeps rendering | raw `HWND` from `window.window_handle()`; toggle `WS_EX_TRANSPARENT` (+`WS_EX_LAYERED`) via `SetWindowLongPtrW(GWL_EXSTYLE)`; test with and without `WS_EX_LAYERED` because GPUI uses `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition | Toggle 20 times; clicks pass to the game underneath; rendering, alpha and hover state all survive; no flicker; keyboard focus does not get stolen when toggling off |
| **S3 show/hide/move** | Per-window hide/show and reposition without recreating the window | `ShowWindow(SW_HIDE/SW_SHOWNOACTIVATE)`, `SetWindowPos`, `SetForegroundWindow` on the raw HWND; fallback = close + reopen window | Hide/show 50 cycles with state intact; reset-position moves to top-right; show re-asserts topmost; GPUI does not log errors or drop input after re-show |
| **S4 hotkeys** | `global-hotkey` inside GPUI's run loop, fired while the game has focus | create `GlobalHotKeyManager` inside `Application::run`, poll `GlobalHotKeyEvent::receiver()` from a GPUI background task; register `Ctrl+Shift+T/I/N/Z/Y`; unregister/re-register at runtime | All 5 fire while PoE is focused (borderless); re-register after change works; suspension during recording works; registration failure for an already-taken combo is reported, not fatal |
| **S5 input widgets** | Text input, select, slider, checkbox, scroll inside the S1 window | (a) gpui-component `Input/Select/Slider/Switch/Scrollable` on `gpui-pre`; (b) hand-rolled `EntityInputHandler` from `examples/input.rs` | IME/typing/selection/clipboard in the input; mouse wheel scroll; measure added compile time and binary size for (a). Output = **ADR-002 component source** |
| **S6 audio + monitor + tray + single instance** | Supporting crates coexist with GPUI | `rodio` plays wav/ogg/mp3/m4a with volume; `sysinfo` lists processes and matches `pathofexile` within 3 s; `tray-icon` menu Show/Hide/Exit on Windows; single instance via named mutex (hand-rolled, `CreateMutexW`) reveals the first instance | All green; CPU idle < 1% with monitor polling |
| **S7 Linux X11 + Wayland** (agent box + Miles if available) | Same as S1..S4 on X11 (override-redirect PopUp) and Wayland (LayerShell) | X11: `xfixes` input region for click-through, `_NET_WM_MOVERESIZE`; Wayland: `WindowKind::LayerShell(Overlay)` + `set_input_region`, `global-hotkey` unavailable | X11 parity with Electron on a compositing WM; Wayland limitations documented; `ECHOSIGHT_FORCE_X11` works |

**Go/no-go rule:** S1, S2, S3, S4 must all pass on Windows. If S2 or S3 fail with raw-handle FFI, the fallback is to contribute `PlatformWindow::set_mouse_passthrough`, `set_position`, `hide/show` upstream to GPUI (small, self-contained, Zed accepts platform PRs) and pin to a fork rev until merged. If even that is unworkable, stop and revisit the Tauri 2 option with Miles; do not start Phase 3 on a shaky window layer. Phase 1 (pure Rust) can start in parallel with spikes because it does not depend on them.

### Task 0.7: ADR-003 window strategy, ADR-004 Wayland policy, ADR-005 data compatibility
Short decision records capturing spike outcomes, written before Phase 4.

---

## Phase 1: `echosight-core` with golden parity tests

### 1.0 Golden fixture harness (do this first)
The parity proof is: the same inputs produce byte-identical JSON outputs in TS and Rust.
- Add `tools/golden/` (TypeScript, inside the existing Vitest setup, on branch `gpui-port`, no changes to `src/`): scripts that import the real modules (`taskStateController`, `taskTemplates`, `templateOperations`, `settingsDomain`, `hotkeys`, `themeValidation`, `themeLibrary` pieces, `windowPlacement`, `taskProgress`, `taskMigration`, `communityTemplates`) and run **scenario scripts** (JSON: a list of operations) to emit `crates/echosight-core/tests/golden/<area>/<scenario>.{input,expected}.json`.
- Scenario coverage must include every test in `tests/unit/{tasks,settings,main,themes}` plus: 30-step random mutation sequences with seeded ids and fixed clock (inject `now()` and `random()`; the TS code uses `Date.now()`/`Math.random()` directly in `taskCreation.ts:7-9` and `templateOperations.ts:35,44,54`, so the harness monkeypatches `Date.now`/`Math.random` deterministically), template round-trips with triggers at depth 3, legacy settings/task files, corrupt files, every bundled theme through validation/variant expansion.
- Rust side: `tests/golden.rs` iterates fixtures, feeds `input` through the same operation interpreter, compares `serde_json::Value` structurally **and** the pretty-printed bytes (format parity for files that are written to disk).
- Commit fixtures; regenerate only via `npm run golden:gen` with a diff review.

### 1.1 Module port map (TS to Rust), each with its own task + tests
| TS source | Rust module | Notes |
|---|---|---|
| `shared/types.ts`, `settingsDefaults.ts`, `errors.ts` | `core::{task::model, template::model, settings::model}` | `Task.id: f64` newtype `TaskId(f64)` with ordered-float semantics; serde `#[serde(default)]` for legacy tolerance; `Theme` v1 as `serde_json::Map` + typed known fields |
| `taskCreation.ts`, `taskTree.ts`, `taskTreeOperations.ts`, `taskMutations.ts` | `task::{ids, tree, mutations}` | injected `Clock` + `Rng` traits |
| `taskCompletion.ts`, `taskProgress.ts`, `taskMigration.ts` | `task::{completion, progress, migration}` | |
| `backgroundTaskController.ts` (logic only) | `task::background` | pure: returns `Vec<ExpiryToRestart>`; timers live in ui |
| `taskStateController.ts`, `taskPersistence.ts` | `task::state` | undo/redo stacks, snapshots, save-ready flag, `corruptBackupPath` passthrough |
| `taskTemplates.ts`, `templateOperations.ts`, `templatePersistence.ts`, `communityTemplates.ts` | `template::{ops, transfer, community}` | 6 community templates as `const` data, test asserts names/counts |
| `settingsDomain.ts`, `main/utils/hotkeys.ts`, `hotkeyDisplay.ts`, `hotkeyRecorder.ts` (combo logic) | `settings::{normalize, hotkeys}` | both accelerator spellings, recorder key mapping table, duplicate detection, platform label |
| `jsonStorage.ts`, `storageIpc.ts`, `runtimeDataBootstrap.ts`, `dataPaths.ts`, `pathSafety.ts` | `storage::{json, atomic, quarantine, bootstrap, paths}` | `destr`-compatible lenient parser front-end; `dirs::config_dir()/echosight/data` with `ECHOSIGHT_DATA_DIR` override; mac: `~/Library/Application Support/echosight` |
| `themeValidation.ts`, `themeHelpers.ts`, `themeLibrary.ts` (main@d96c2d5 version), `themeAssetLoaders.ts` | `theme::{v1, validate, library, sync, assets}` | v1 loader kept for backwards compat; sync rule; asset discovery and type inference; path containment via `Path::components()` + `canonicalize` (fixes the lexical-only check, counts as hardening not behaviour change) |
| `themeSounds.ts` (resolution + mixing) | `theme::sounds` | pure math; playback in platform |
| `windowPlacement.ts` | `window::placement` | 3 asserted examples |
| strings in inventory section 11 | `core::strings` | single source of truth for UI text; test that every string from the catalogue exists |

### 1.2 Real-data acceptance
`cargo run -p echosight-theme-cli -- doctor <path-to-unzipped v2.1.1 data dir>` loads tasks/templates/settings/themes from Miles's Windows fixture (Task 0.6) and from `~/.config/echosight` on Linux without warnings; then writes them back to a temp dir and diffs byte-for-byte (expect zero diff except normalisation that TS would also perform, which the golden harness proves).

Gate: all golden tests green, `cargo test -p echosight-core` includes >= the 145 Vitest behaviours (track with a checklist file `docs/plans/gpui-port/parity-core.md`, one line per Vitest `it(...)` name mapped to a Rust test name).

---

## Phase 2: Theme Format v2 and bundled theme port

### 2.1 Spec (`docs/themes.md` rewrite, with JSON Schema in `crates/echosight-core/schema/theme-v2.schema.json` via `schemars`)
Adopt the proposal in the theme inventory section 6.2 with these fixed vocabularies:
- `format: 2`, `id`, `name`, `version`, `author`, `description`, `supportsTransparency`.
- `tokens`: `colors` (flat + `text`, `surface` (was `background`), `border` groups), `opacity` (`appliesTo` list, default all `surface.*` and window/section fills; replaces the `calc(var(--user-transparency))` idiom), `fonts` (`ui`, `header`, `mono`, each `{family: [..], file?}`, `sizes`, `weights`), `radius`, `shadows`, `bevels` (`{light, dark, innerLight?, innerDark?, width}`).
- `slots` (closed list): `window.interactive`, `window.clickThrough`, `header`, `headerTitle`, `templateSection`, `tasksSection`, `sectionTitle`, `taskRow`, `taskText`, `checkbox`, `subtaskRow`, `backgroundRow`, `badge.high`, `badge.active`, `badge.dormant`, `button`, `button.primary`, `button.danger`, `button.icon`, `input`, `textarea`, `select`, `slider`, `progressBar`, `progressFill`, `progressText`, `toast`, `contextMenu`, `contextMenuItem`, `dialog.scrim`, `dialog.window`, `dialog.title`, `dialog.text`, `dialog.hint`, `dialog.button.primary`, `dialog.button.secondary`, `dialog.danger.title`, `communityTemplateCard`, `divider`, `dragPlaceholder`, `clickThroughHint`.
- Slot properties (closed): `fill` (color | `{color, alpha}` | `{gradient: {angle, stops: [[color, pos]..]}}` | `{radial: ...}`), `border {color,width,style}`, `bevel`, `radius`, `shadow[]` (outer only), `padding`, `margin`, `size/minSize/height`, `text {color,size,weight,family,transform,letterSpacing,strikethrough,shadow}`, `ring {color,width,style,inset}`, `image {asset, mode: stretch|cover|nineSlice, insets}`, `opacityAffects: bool`.
- States (closed): `hover`, `active`, `focus`, `disabled`, `checked`, `completed`.
- `@token.path` references, `$extends: "<slot>"`.
- `images` (explicit asset mapping; the v1 filename convention `main_bg`, `titlebar_bg`, `button_normal/hover/pressed`, `progress_bg/fill` becomes defaults the converter emits).
- `effects: { windowBlur: "none"|"system" }`, `motion: { hoverMs, progressFillMs, toastMs }`.
- `sounds`: unchanged from v1. `variants`: unchanged merge semantics (defu-equivalent deep merge).
- Validation: unknown slot/property = **warning** (forward compat); `cssFile`/`customCSS` present in a `format: 2` file = **error**; `format` missing = v1 path.

### 2.2 Converter (`echosight-theme convert <in> <out-dir>`)
- Lossless for: colors, fonts, effects.borderRadius/boxShadow/buttonShadow/glow, `styles.interactive|clickThrough.*` to window/header/section slots, `components.*` (+`states`) to slots, asset filenames to `images`, `sounds`, `variants`.
- `customCSS`/`cssFile`: a small CSS tokenizer (rules, selectors, declarations only; no cascade) + a selector-to-slot table for the known modal/dialog family and the generated selectors (`.overlay-container.interactive .header`, `.task-item`, `.modal-content`, `.modal-btn.primary`, ...) + a declaration-to-property table (color, background(-color/-image gradient), border, border-radius, box-shadow outer, font-*, text-transform, letter-spacing, padding, text-shadow). Everything else goes into `<out>/CONVERSION-REPORT.md` as `unsupported: <selector> { <decl> }`.
- Tests: converting each of the 7 JSON themes yields a v2 that validates; report lists are snapshot-tested with `insta`.

### 2.3 Port the 11 bundled themes (version bump to trigger sync)
| Theme | Method | Target fidelity (from inventory 6.3) | Special work |
|---|---|---|---|
| dark, light, minimal | converter + review | ~98..100% | none |
| cs16 | converter + review | ~92% | 1 px hard text shadow via `text.shadow`, drop checkbox drop-shadow |
| echosight | converter + hand tune | ~80% | bundle Cinzel/Inter/JetBrains Mono (OFL, add `assets/fonts/LICENSES`), hairline to `divider`, `windowBlur: system` |
| glass | converter + hand tune | ~55% | `windowBlur: system`; accept loss of nested per-section blur (document in CHANGELOG) |
| exiles_corruption | converter + hand tune | ~85% | radial modal gradient via `radial` (no sounds; branch discarded) |
| winamp-classic | hand-authored | ~90% | SVG skins via `images` (stretch for bg/titlebar, nineSlice for buttons), uppercase header, hover inversion, `complete.wav` |
| windows-98, windows-xp | hand-authored | ~93..95% | bevel tokens (raised/sunken), dotted focus ring approximated by `ring.style: dashed`, pixel font: check licence of "Pixelated MS Sans Serif" (98.css ships it under MIT? verify) else fall back to system `MS Sans Serif`/`Tahoma` |
| windows-7 | hand-authored | ~75% | glass overlay as second fill layer in `window.interactive`, footer hairline as `divider`, checkbox glyph app-drawn, multi-stop gradients with positions, focus ring via `ring` |

Acceptance per theme: side-by-side screenshot (Electron 2.1.1 vs GPUI build, interactive + click-through + settings modal + a dialog) committed under `docs/plans/gpui-port/theme-parity/<id>/`; Miles approves each. Tests: `bundled_themes_validate_v2`, `bundled_themes_have_bumped_versions`, `no_bundled_theme_uses_css`, modal slots present for every theme (replaces `themeModalStyles.test.ts`).

### 2.4 Theme docs
`docs/themes.md` (v2 authoring), `docs/themes-v1-migration.md` (what changed, converter usage, FAQ "why no CSS").

---

## Phase 3: `echosight-ui` (GPUI)

Sub-plan written after ADR-002. Structure regardless of component source:

- `style/`: `ThemeRuntime` = resolved v2 theme (tokens dereferenced, user opacity applied) with `fn slot(&self, Slot, State) -> ResolvedStyle` and `impl ResolvedStyle { fn apply(self, div: Div) -> Div }` mapping to GPUI (`bg`, `border_*`, `rounded_*`, `shadow`, `text_color`, `font_*`, gradient via `linear_gradient`, image via layered `img()`, bevel via nested borders, text shadow via duplicated text run).
- `components/`: `TextInput` (or wrapper), `Select`, `Checkbox` (app-drawn, themeable glyph), `Slider`, `Switch`, `Button`, `ScrollList`, `ContextMenu` (anchored overlay closing on outside click), `Toast` (3000 + 300 ms exit), `DialogStack` (modal stack with focus trap, Escape/Enter rules from inventory 8.1/8.2, danger focus-Cancel), `HotkeyRecorder`, `DragList` (pointer-based drag with intent rules and 50 px snapping, placeholder row).
- `views/`: `OverlayRoot` (interactive vs click-through visual state), `Header`, `TemplateBar`, `MainTasks`, `BackgroundTasks`, `TaskRow`, `ProgressBar`, `AddTaskInput`, dialogs: `EditTask`, `ConfigureTriggers` (with nested edit of dormant tasks), `Settings`, `ThemeSelection` (live preview renders sample widgets with the candidate theme; replaces scoped-CSS hack), `CommunityTemplates`, `ImportTemplate` (paste or file), `AlertConfirm`.
- `app_state.rs`: `Entity<AppState>` holding core `TaskState`, templates, settings, theme runtime, expiry timers (`cx.spawn` + `Timer`), feedback timer, and the `persist()` funnel = `render; progress; save` after every mutation (no debounce, same as today).
- `ports.rs`: trait `PlatformPort { focus_window, hide, reset_position, toggle_interactive, quit, set_hotkeys, set_hotkey_recording, open_path, play_sound, prompt_open_file, prompt_save_file }` so the UI is testable with a fake.
- Headless tests with `#[gpui::test]` + `VisualTestContext`: port every DOM-level Vitest in `tests/unit/{tasks/rendererTaskEditing,tasks/triggerController,settings/settingsUi,settings/hotkeyRecorder,ui/dialogService,ui/modalUi,themes/themeSelectionUi}` to interaction tests (`simulate_click`, `simulate_keystrokes`), asserting view-model state and rendered strings.
- Fonts embedded with `include_bytes!`, registered via `cx.text_system().add_fonts`.

Gate: `docs/plans/gpui-port/parity-ui.md` checklist (every bullet of renderer inventory section 4 to 8 and every string of section 11) ticked with a test name or a screenshot reference.

---

## Phase 4: `echosight-platform` and app wiring

- `overlay_window.rs`: `OverlayWindow` wrapping `WindowHandle` + raw handle; `set_click_through(bool)`, `show/hide`, `set_position`, `assert_topmost`, `focus`, with `cfg(windows)` (`windows` crate), `cfg(target_os = "macos")` (`objc2-app-kit`: `ignoresMouseEvents`, `setLevel`, `setFrameOrigin`, `orderOut/orderFront`), `cfg(linux)` X11 (`x11rb` xfixes input region, `_NET_WM_STATE_ABOVE` as belt-and-braces) and Wayland (`set_input_region`; move/hide unsupported, documented).
- `hotkeys.rs`: `global-hotkey` manager; converts `CommandOrControl+Shift+T` strings to `HotKey` (Ctrl on Win/Linux, Cmd on mac); registration report logged exactly like today; suspend/resume for recording.
- `game_monitor.rs`: `sysinfo` refresh of process names every 3 s on a background thread, channel to the app; same `pathofexile` normalisation; `onStarted/onStopped/onStillRunning` semantics.
- `single_instance.rs`: Windows named mutex `Local\Echosight.SingleInstance` + a named pipe/`WM_COPYDATA` to signal reveal; Linux/mac abstract unix socket at `$XDG_RUNTIME_DIR/echosight.sock`.
- `audio.rs`: `rodio` `OutputStream` lazily created; cache decoded sounds per event; volume from core.
- `tray.rs`: `tray-icon` on Windows only for parity (menu Show / Hide / Exit, tooltip `Echosight`, click = show); feature-flag `tray-everywhere` for later.
- `app_menu.rs`: macOS menu with the 5 hotkey items + Quit (GPUI `cx.set_menus`).
- `main.rs` sequence mirrors `index.ts`: single-instance check, data bootstrap, theme sync + load, settings load + hotkey normalise, open window (hidden), register hotkeys after first frame (no 2 s sleep needed; log registration result), start monitor, tray, show after first paint; lifecycle handlers (`on_window_closed`, mac reopen, quit cleanup).
- Windows release build: `#![windows_subsystem = "windows"]`, embedded icon via `winresource`, console logging to `%APPDATA%\echosight\logs\echosight.log` (rolling, since Electron users had DevTools logs; keep `RUST_LOG`).

Windows manual QA script (`docs/plans/gpui-port/qa-windows.md`): 40 numbered steps covering every item in section 2.2 against a borderless PoE/any game, performed by Miles on each Phase 4 build.

---

## Phase 5: Parity verification, performance, macOS

- Parity matrix `docs/plans/gpui-port/parity-matrix.md`: rows = section 2.2 items + 2.3 deltas; columns = Windows / Linux X11 / Linux Wayland / macOS; cells = test name, QA step, or `n/a (documented)`.
- Data: start the Rust build against a copied v2.1.1 data dir on all platforms; verify tasks/templates/settings/themes identical in UI; then run Electron 2.1.1 again against the same dir and confirm it still opens it (forward compatibility for a rollback).
- Budgets (measure, publish in CHANGELOG): cold start < 1 s, idle CPU < 1% with monitor, idle GPU near 0 when not dirty (GPUI `inactive_frame_interval`), RSS < 150 MB, binary < 40 MB stripped (Electron portable was ~100 MB+). Not gates unless grossly exceeded; report honestly.
- macOS: CI builds x64 + arm64; one manual pass by Miles or a tester for: `Accessory` activation policy (no dock icon), `PopUp` level above fullscreen spaces, `ignoresMouseEvents`, Carbon hotkeys permission prompt, Gatekeeper (unsigned, same as today).
- Accessibility note: GPUI has AccessKit only via gpui-component; document as a known gap vs Chromium.

---

## Phase 6: Packaging, CI, release

New `.github/workflows/rust.yml` (SHA-pinned actions, `permissions: contents: read`, `concurrency` group, no `${{ }}` inside `run:` scripts, same hardening style as `build.yml`):
- `check`: fmt, clippy `-D warnings`, `cargo test --workspace` on ubuntu (headless GPUI tests), `cargo deny check` (licenses/advisories; replaces `npm audit` gate with an allowlist file `deny.toml`).
- `build` matrix: `windows-latest` (`cargo build --release` + `cargo-wix`? No: keep **portable single `.exe`** for parity; embed `data/` and fonts with `include_dir!`/`rust-embed`), `macos-latest` (two targets `x86_64-apple-darwin`, `aarch64-apple-darwin`, `cargo-bundle` to `.app`, `create-dmg` to `Echosight-3.0.0-x64.dmg` and `Echosight-3.0.0-arm64.dmg`), `ubuntu-latest` (`cargo-deb` for `.deb`, `linuxdeploy`/`appimagetool` for AppImage with the required system libs policy documented).
- `release` on `v*` tags: same invariants as today (tag == `v<Cargo.toml version>`, no existing release, exactly 1 exe, 2 dmg incl. one `-arm64.dmg`, 1 AppImage, 1 deb, `if-no-files-found: error`, `persist-credentials: false`), plus SHA-256 checksums file (new, cheap).
- Port `tests/unit/ci/workflow.test.ts` invariants to a Rust test (`crates/echosight/tests/workflow_invariants.rs` parsing YAML with `serde_yaml`) for the new workflow.
- Rehearse with `v3.0.0-rc.1` on a throwaway tag (pre-release), then delete.

---

## Phase 7: Cutover

1. Final PR `gpui-port` to `main`: remove `src/`, `tests/`, `electron.vite.config.ts`, `vitest.config.ts`, `tsconfig.json`, `package.json` (keep a tiny one only if docs tooling needs it), `.github/workflows/build.yml`, `dist-electron/` from git if tracked. Keep `tools/golden/` TS scripts? No: freeze fixtures, delete the generator, note the Electron tag `v2.1.1` as the way to regenerate.
2. `README.md`: stack, build instructions (`cargo build --release`, platform deps), Theme v2 pointer, platform notes (Wayland). `docs/index.html` landing page: fix the `echoesight-overlay` link typo while touching it, platform list unchanged.
3. `CHANGELOG.md` 3.0.0: port summary, accepted deltas (section 2.3), performance numbers, migration notes (none for data; themes auto-updated by version; custom CSS themes need the converter).
4. LLM Wiki: update `Echosight.md` and `Echosight Architecture And Maintenance Baseline.md` (new stack, new source-of-truth boundaries, new safe-update workflow), add `Source - 2026-xx-xx GPUI port` entry, `log.md`.
5. Tag `v3.0.0` from `main`; verify the release job; Miles installs the portable exe over a real v2.1.1 data dir as the last smoke test.
6. Post-release issues to open (not in scope): Settings Cancel revert, hard-coded `Ctrl+Shift+Z` in confirms, Save Template overwrite confirm, expiry UI, tray on all platforms, Wayland GlobalShortcuts portal, code signing.

---

## 5. Risks and mitigations (ranked)

| # | Risk | Likelihood | Mitigation |
|---|---|---|---|
| 1 | GPUI has no release channel; API churn (platform crates split, blade to wgpu, `WindowKind` changes in the last year) | High | Pin exact rev; upgrade once per phase in a dedicated PR; keep FFI behind `echosight-platform` so churn is localised |
| 2 | Click-through / move / hide need unsafe FFI on Win/mac/X11; `WS_EX_LAYERED` vs DirectComposition interaction unverified | Medium | Spikes S2/S3 are go/no-go before UI work; upstream fallback; worst case Tauri re-evaluation |
| 3 | Topmost tied to `WindowKind::PopUp` (tool window, non-activating on mac, override-redirect on X11: no WM focus) | Medium | S1 tests focus/typing in a PopUp; if typing breaks on X11, use `Normal` + `_NET_WM_STATE_ABOVE` + skip-taskbar atoms via raw handle |
| 4 | Theme fidelity complaints (glass, win7) | Medium | Side-by-side approvals per theme in Phase 2; honest CHANGELOG |
| 5 | Phase 3 scope (dialogs, DnD, recorder, preview) underestimated | Medium | Headless test-first per view; the renderer inventory is the spec; sub-plan per view cluster |
| 6 | Wayland users lose hotkeys/topmost | Low (PoE audience is Windows) | Documented; `ECHOSIGHT_FORCE_X11`; later portal work |
| 7 | Windows SDK/MSVC/CMake build complexity in CI | Low | Follow zed's `windows.md`; cache `target/`; nightly `cargo build` canary job |
| 8 | Data corruption during cutover | Low | Core writes are atomic and golden-tested; rollback test in Phase 5 proves Electron still opens the files |
| 9 | Font/asset licensing (Cinzel/Inter/JetBrains Mono OFL fine; "Pixelated MS Sans Serif" uncertain) | Low | Verify in Phase 2; fall back to system fonts |

## 6. Open questions for Miles (answer before the phase that needs them)

1. (Phase 0) Sounds branch: answered, discarded.
2. (Phase 0) OK to run the Windows spikes from CI-built artifacts, or do you want a Rust toolchain on the Windows PC (`rustup` + VS Build Tools + CMake)?
3. (Phase 2) Keep the `glass` theme at all, given its defining blur becomes window-level only? Recommended keep, with the delta noted.
4. (Phase 2) Who owns the hand-authored win98/xp/7/winamp v2 files: agent drafts + your screenshot approval (default), or you want to author them?
5. (Phase 4) Tray on all platforms in 3.0.0 (cheap with `tray-icon`) or Windows-only parity? Default: Windows-only, flag for later.
6. (Phase 6) Version `3.0.0` and app id `com.poe2tools.task-overlay` kept? Default yes (mac bundle id continuity).
7. (Phase 7) Is `docs/index.html` (landing page) in scope for a refresh beyond link fixes? Default no.

## 7. Verification summary (what "done" means)

- `cargo test --workspace` green on all three OS runners, including golden fixtures and headless GPUI interaction tests.
- `docs/plans/gpui-port/parity-core.md`, `parity-ui.md`, `parity-matrix.md` fully ticked, each with evidence.
- Windows QA script passed by Miles on the release candidate against a real game.
- 11 bundled themes approved via side-by-side screenshots.
- Rust build opens a real v2.1.1 data dir unchanged; Electron 2.1.1 can still open the files afterwards.
- Release workflow produces exe / 2 dmg / AppImage / deb from a tag with the fail-closed checks preserved.
- README, docs/themes.md, CHANGELOG, LLM Wiki updated. No `Co-Authored-By` trailers anywhere in `git log gpui-port`.

Nothing in the repository was modified while producing this plan. The plan and evidence reports are at `/home/agent/code/.hermes/plans/2026-11-09_185953-echosight-gpui-port.md` and `/home/agent/code/.hermes/plans/echosight-gpui-port/`.
