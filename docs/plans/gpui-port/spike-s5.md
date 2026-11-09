# Spike S5: Portable input and bounded controls

**Verdict: compilation and headless interaction pass; Windows UI/IME acceptance
pending.** No display was available on the Linux host. No Windows executable was
linked or run here. ADR-002 is a conditional recommendation, not a visual verdict.

## Build and run

Pinned GPUI: Zed `4ed72e0135aaab74a43914491c915dd6cbd10c67` (ADR-001 unchanged).
Example: `crates/echosight-platform/examples/s5_input_handrolled.rs`.
Scene: `crates/echosight-ui/src/spikes/`; reusable input: `src/components/text_input.rs`.

```sh
cargo run -p echosight-platform --example s5_input_handrolled
cargo build --release -p echosight-platform --example s5_input_handrolled
cargo test -p echosight-ui
```

The example reuses S1's `common::window_options`: transparent, frameless PopUp,
500x700, minimum 450x600, top-right work-area placement. Its header provides a drag
region and Quit. It does **not** re-run the S1 native topmost/click-through probes.
There is no persistence, task mutation, sound playback or production-app wiring.

The existing workflow accepts arbitrary example names; no workflow change is
required. After the reviewed commit is integrated/pushed, the parent can dispatch:

```sh
gh workflow run rust.yml --ref gpui-port -f spike=s5_input_handrolled
```

Download `spike-s5_input_handrolled-windows`. Bundle the executable with this manual
script, `crates/echosight-ui/THIRD_PARTY.md` and `LICENSE-APACHE`. These notices are
also embedded in the binary and printable with `s5_input_handrolled.exe --licenses`.
The existing workflow uploads the exe only, so the handoff archive should add the
notice files. Do not infer that a Windows artifact already exists from this command.

There are no runtime repository paths or loose image/font assets. License text is
embedded with `include_str!`; controls are drawn by GPUI using installed system
fonts (Segoe UI on Windows, Noto Sans on Linux, system font on macOS). Native
Unicode glyph fallback still depends on the OS fonts. Theme font bundling is not
part of S5. Launch from an unrelated directory as part of the manual check.

## What the spike implements

- Single-line input, placeholder, focus ring, selection, caret and horizontal
  clipping/scrolling. UTF-8 storage; UTF-16 conversion at the input-handler boundary.
- Left/right and Backspace/Delete operate on extended grapheme clusters; Home/End,
  Shift+arrows/Home/End, Ctrl/Cmd+A/C/X/V, Enter submit, Escape event, mouse
  positioning/drag selection and double-click select-all. The caret does not blink.
- Maximum **100 UTF-16 code units**, including typing, paste, programmatic replacement
  and IME preedit/commit. Never truncates inside a Unicode scalar. A 2-unit emoji
  cannot fit in a 1-unit remainder. Truncation may split a multi-scalar grapheme;
  no claim of grapheme-count maxlength is made. CR/LF become spaces on insertion.
- Priority select (Low/Medium/High): click reveals an **inline option row**, not a
  production popover; arrows/Home/End update it, Enter/Space toggle, Escape closes.
- Volume slider: click/drag, arrows and Home/End, clamped 0..100; state only.
- Sounds switch: mouse or Space/Enter; state only.
- Wheel-scrolled 50-row viewport, using GPUI's scroll handling.
- Dialog probe: background controls are temporarily removed from the dispatch tree,
  Cancel initially focused, Tab/Shift+Tab trapped, Escape cancels, Enter never
  confirms danger, Confirm click/Space increments a probe counter, focus returns to
  the opener. This is **not** a queued/nested production modal stack or dimmed overlay.

Production accessibility semantics/screen-reader behavior, editor undo/redo, word
movement, context menu, RTL layout and native composition cancellation remain outside
this bounded implementation. Do not mark renderer parity from this spike.

## Executed verification

Host: Linux x86_64, Rust `1.99.1 (58cf8f9c3 2026-10-13)`, no DISPLAY or
WAYLAND_DISPLAY. Builds used the previously documented local GPUI/ALSA/resource
compiler shims via `source ~/.local/gpui-shim/env.sh` (not repository dependencies).

| Gate | Actual result |
|---|---|
| `cargo test -p echosight-ui` | **23 passed**, 0 failed/ignored: 18 input regressions, 4 scene-interaction regressions, 1 pre-existing crate smoke test; 0 doc-tests |
| `cargo test --workspace --locked` | Passed from a fresh staged-index export, including the final 23 UI tests; no untracked source dependency |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo build -p echosight-platform --example s5_input_handrolled` | Passed, Linux linked executable |
| `cargo build --release -p echosight-platform --example s5_input_handrolled` | Passed; stripped Linux executable 20,183,032 bytes after final fixes; earlier build raw wall 162.76 s, not a comparative incremental benchmark |
| `cargo check -p echosight-platform --example s5_input_handrolled --target x86_64-pc-windows-msvc` | Passed; **type-check only**, not Windows linking or execution |
| Release executable `--licenses` from unrelated scratch cwd | Exit 0, Apache license text present |
| Release executable startup from unrelated scratch cwd | Logged `S5 window opened`; event loop terminated after a bounded 3-second smoke. No display, screenshot or visual approval |
| Matching-snapshot gpui-component scratch scene | Linux check and Windows-target check passed; preserved Linux release exists; see ADR-002 |
| gpui-component with the tested git-pin patch | Graph retains two GPUI identities; real App boundary probe fails E0308; see ADR-002 |

The `#[gpui::test]` tests render through GPUI's **test platform**. Keyboard tests
use `simulate_keystrokes`/`simulate_input`, not a duplicate text model. Clipboard
assertions use the test platform clipboard, not the Windows system clipboard.
IME tests call the real `EntityInputHandler` methods against a test window; they do
not start an OS IME or validate TSF/candidate placement visually.

Verified headless cases include insertion in the middle via key dispatch, deletion
of flags/combining marks/ZWJ emoji, directional selection and replacement,
Ctrl/Cmd cut/copy/paste, newline normalization, exactly-100 limits including astral
characters, repeated set/clear, Enter/Escape events, blur/click refocus, mouse drag,
preedit replacement/commit/unmark notifications, empty-field IME caret bounds,
UTF-16 surrogate/out-of-range conversion and
selection clamping at the limit. A narrow 240px window exercises long-input caret
visibility and Home returning to the origin. Scene tests drive real tab, click,
slider-drag and scroll events, including modal trapping/restoration and Enter safety.

## Windows manual acceptance script

Record Windows build, display scale, keyboard layout, IME name/version, artifact
commit and screenshots/video. These rows are intentionally **not pre-filled as passes**.

1. Extract exe plus notices to a folder without the checkout, launch from a different
   working directory. Check all controls at 100%, 150% and 200% DPI; resize to the
   450x600 minimum. Drag header; ensure Quit works. Look for clipped labels/actions.
2. Type `hello`, move left twice, type `X`, Backspace, Delete. Try Home/End and Shift
   selection in both directions. Replace the selection; double-click selects all;
   drag selection outside/re-enter the field. Blur by clicking another control,
   then click input and type. Verify caret, selection and focus ring match actions.
3. Ctrl+A/C/X/V with Notepad in both directions, including `héllo`, `你好`, `😀`,
   `🇳🇱`, `é`, `👩‍💻` and multiline text. Backspace/Delete should remove a grapheme
   as a unit; newline paste should remain single-line (spaces in this probe).
4. Paste 150 ASCII letters: counter caps at 100. Replace all with 60 `😀`: counter
   is 100 and 50 emoji remain. Enter 98 ASCII letters followed by `😀`: 100 units;
   further typing is rejected until deletion/selection frees room. Verify cursor
   stays visible across the full line and Home/End scroll correctly.
5. Enable a real Chinese/Japanese/Korean Windows IME. Compose, change candidates,
   commit, cancel, move selection, then repeat near the 100-unit limit and after
   focus changes. Candidate UI should be anchored at the visible caret (also after
   horizontal scrolling). No duplicated/dropped text, unexpected task submission,
   broken surrogate or panic. **This is the central unverified S5 gate.**
6. Tab/Shift+Tab through input, priority, volume, sounds and dialog opener. Priority
   click/keyboard updates agree; slider click/drag and arrows clamp at 0 and 100;
   switch click/Space agree. Scroll to row 50 and back by wheel.
7. Open dialog: Cancel focused. Cycle Tab/Shift+Tab repeatedly; hidden background
   controls must not react. Escape/Cancel return focus to opener. Enter on Confirm
   must not confirm; Confirm click/Space increments once. Repeat with existing
   input text and ensure its state survives.

| Check | Windows result | Evidence/notes |
|---|---|---|
| Portable launch, fonts, DPI, resizing | Not run | |
| Typing/deletion/selection/mouse/focus | Not run | |
| Native cross-application clipboard + Unicode | Not run | |
| Max100 + horizontal caret visibility | Not run | |
| Native IME composition/cancel/candidate placement | Not run | |
| Select/slider/switch/scroll | Not run | |
| Dialog focus trap, restoration, danger Enter safety | Not run | |

## Fallback / next decision

If native input behavior fails, retain the reproducer and fix/retest the narrow
handler before accepting S5. If the maintenance cost is unacceptable, explicitly
revisit ADR-001/002 and evaluate the already-compilable gpui-component snapshot
path on Windows. Do not silently switch the GPUI source, claim that a successful
resolver unified types, or treat a cross-target check as a Windows UI pass.
