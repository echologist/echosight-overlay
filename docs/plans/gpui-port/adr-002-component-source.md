# ADR-002: Input/component source (S5)

Status: **proposed — retain ADR-001 and use a small hand-rolled component set**.
Build and headless interaction evidence is available; Windows UI/IME acceptance is
**not verified**. This is not a Phase 3 production-component acceptance gate.

## Context and scope

Master-plan S5 compares Longbridge's `gpui-component` with a GPUI-native
`EntityInputHandler` input, plus select, slider, switch, scroll and dialog probes.
ADR-001 pins both `gpui` and `gpui_platform` to Zed
`4ed72e0135aaab74a43914491c915dd6cbd10c67`; this spike **does not change that pin**.
No Electron, app entry-point, core-domain or workflow changes are needed.

This recovers the interrupted S5 work rather than restarting it. The existing
`text_input.rs` was initially unreferenced and had never passed compilation. It is
now exported, compiled and tested. The original bytes were preserved in the local
recovery scratch directory before edits. The library comparison stays in scratch,
not in the workspace dependency graph.

## Verified dependency/API outcomes

### A. `gpui-component = 0.9.1` with the Zed git pin

The previous `s5a` trial attempted this crates.io patch (alongside ADR-001's three
ordinary patches and the pinned GPUI/platform dependencies):

```toml
[patch.crates-io]
gpui-pre = { git = "https://github.com/zed-industries/zed", package = "gpui", version = "=0.2.2", rev = "4ed72e0135aaab74a43914491c915dd6cbd10c67" }
gpui-pre-macros = { git = "https://github.com/zed-industries/zed", package = "gpui_macros", rev = "4ed72e0135aaab74a43914491c915dd6cbd10c67" }
gpui-pre-sum-tree = { git = "https://github.com/zed-industries/zed", package = "sum_tree", rev = "4ed72e0135aaab74a43914491c915dd6cbd10c67" }
```

The first attempt without `version = "=0.2.2"` failed because the git repository
contains more than one candidate package named `gpui`. Adding that selector let
resolution succeed, but **did not replace gpui-component's GPUI**. Recovery reran
`cargo metadata --locked --format-version 1` on the preserved manifest/lockfile:

```text
git+https://github.com/zed-industries/zed?rev=4ed72e0135aaab74a43914491c915dd6cbd10c67#gpui@0.2.2
registry+https://github.com/rust-lang/crates.io-index#gpui-component@0.9.1
registry+https://github.com/rust-lang/crates.io-index#gpui-pre@0.3.12
```

A new, deliberately negative API probe replaced the scratch trial's empty main:

```rust
fn use_pinned_app(cx: &mut gpui::App) {
    gpui_component::init(cx);
}
fn main() { let _ = use_pinned_app; }
```

`cargo check --locked` failed with **E0308**, `expected gpui::app::App, found App`;
the compiler identified the registry `gpui-pre` App as expected and the git GPUI
App as supplied. Thus **the tested patch recipe is not an interoperability solution**.
Two GPUI crate identities can coexist in a Cargo graph; their App/Window/entity
objects cannot be passed interchangeably. This narrows ADR-001's wording that a
binary "cannot contain both": the verified problem is the type boundary, not
mere graph coexistence.

This does **not** prove that every possible fork or manifest rewrite would fail.
No broad vendoring/renaming exercise or alternate GPUI pin was attempted.

### B. `gpui-component = 0.9.1` on its matching snapshots

The preserved separate `s5a-pre` trial uses:

```toml
[dependencies]
gpui = { package = "gpui-pre", version = "=0.3.12", default-features = false, features = ["font-kit", "x11", "windows-manifest"] }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.12", default-features = false, features = ["x11"] }
gpui-component = "=0.9.1"
env_logger = "0.11"
log = "0.4"
```

Its real scene compiles `InputState`/`Input`, `SelectState`/`Select`,
`SliderState`/`Slider`, `Switch::on_change`, `overflow_y_scrollbar`,
`WindowExt::open_dialog`, and `Root`. Recovery reran Linux `cargo check --locked`
and Windows-target `cargo +1.99.1 check --locked --target x86_64-pc-windows-msvc`:
**both passed**. The first Windows attempt used the `stable` toolchain alias whose
Windows std target was absent; the installed explicit workspace toolchain fixed
that environment error without source changes.

The prior release binary and build log survived. The log records completion and
`/usr/bin/time` elapsed **3:36.72**, with a stripped Linux binary of **31,144,872
bytes**; recovery independently checked that byte size. Cache state/complete raw
build output is not established by the short recovered log, so this is **not a
controlled clean-build measurement**. The trial's input validator counts Rust
scalars, not UTF-16 units: compilation is not maxlength or IME parity proof.
Neither Windows execution nor native input interaction was performed on this path.

### C. Hand-rolled input on the unchanged git pin

The workspace example is `s5_input_handrolled`. Its input derives from the pinned
Zed `crates/gpui/examples/input.rs`, retains Apache-2.0 licensing, and is exported
from `echosight-ui::components::text_input`. The small companion controls live in
`echosight-ui::spikes`, not in core or the production application.

Recovery corrected the partial implementation's conflicting manual `IntoElement`
implementation (the pin has a blanket implementation for `Element`), test reads
(`Entity::read_with` for `VisualTestContext`), whole-text replacement length budget,
and scalar-vs-UTF-16 maxlength mismatch. It also added single-line normalization
at every insertion boundary and clipped horizontal scrolling to keep the caret
visible. Empty-input IME caret geometry and commit-by-unmark change notification
also have regression tests. The initial max-length test's suffix expectation was
corrected: 98 `a`s
plus `bcd`, capped at 100, ends in `bc`, not `ab`.

`gpui`'s `test-support` feature is enabled **only as a UI dev-dependency**; tests
are attached to exported modules and actually execute. Cargo.lock consequently
adds upstream test-support dependencies; there is no gpui-component/gpui-pre
addition to the workspace. See `spike-s5.md` for gates and behavioral coverage.

A Linux release build on recovery completed in **162.76 seconds** (`time` wall),
producing **20,182,904 bytes**, with workspace thin-LTO/strip/panic-abort settings.
It was the first release build in this worktree, after debug/check builds and with
warm package caches. This and the recovered component result are **raw observations,
not an isolated added-compile-time benchmark**: scene, dependency graph and cache
conditions differ. A final release rebuild after empty-input caret/unmark fixes
produced **20,183,032 bytes**. No incremental-time advantage is claimed.

## Recommendation (inference, not a measured UX verdict)

Keep ADR-001 and proceed with a narrowly owned hand-rolled set, **conditional on
Windows input acceptance**. It avoids a source migration solely to obtain a few
controls and already exercises the needed key-dispatch and input-handler APIs.
Do not adopt gpui-component by assuming the patch worked, and do not vendor its
entire tree to hide the type mismatch.

The trade-off is maintenance of text editing, IME behavior, accessibility,
selection and focus. The spike is not a finished editor or component framework.
If native IME/accessibility proves too costly or unreliable, re-open ADR-001 and
this ADR together: adopting matching `gpui-pre`/platform snapshots is a verified
**compilable alternative**, not a verified better Windows UI.

## Evidence and remaining gates

- Repo source: `crates/echosight-ui/src/components/text_input.rs`,
  `src/spikes/{mod.rs,s5_tests.rs}`, and
  `crates/echosight-platform/examples/s5_input_handrolled.rs`.
- Local retained prior evidence: `~/.hermes/cache/scratch/s5a/{Cargo.toml,Cargo.lock}`,
  `s5a-pre/{Cargo.toml,Cargo.lock,src/main.rs,build.log,target/release/s5_input_component}`.
  Prior interrupted transcript: `~/.hermes/cache/delegation/live/deleg_71b0cc8b/task-3.log`.
- Recovery logs: `~/.hermes/cache/scratch/s5-recovery/` (component metadata, negative
  API compile, both cross-checks, Linux build, clippy and release build). These are
  local audit pointers, **not portable artifact download links**.
- Required before accepting S5: a Windows CI-linked executable, manual script
  results, native clipboard and IME/candidate-window checks, DPI/resize and visual
  review. A Linux headless test or Windows cross-check cannot satisfy those gates.
- gpui-component added compile cost under matched controlled conditions remains
  unmeasured. No benchmark conclusion depends on the recovered elapsed time.
