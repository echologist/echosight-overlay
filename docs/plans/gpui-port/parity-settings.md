# Settings and hotkey core parity

Scope: pure settings normalization, defaults, accelerator/display text rules and
recording state. The unchanged TypeScript sources are the oracle; no Electron
production source, package manifest, UI or platform implementation was changed.

## Public API / integration boundary

All APIs are re-exported from `echosight_core::settings`.

- `Settings::default()` is the **bootstrap** v2 document: transparency 70, theme
  `echosight`, sounds disabled at 60, portable `CommandOrControl+Shift+T/I/N/Z/Y`.
  `renderer_defaults(platform)` uses the renderer's `Ctrl` or `Cmd` labels instead.
  `HotkeySettings`, `SoundSettings` and `HotkeyAction` are typed, serde-compatible
  models. Action iteration follows the source's five-action order; labels retain
  `Forward Last Task Action` for redo.
- **Fallible API:** `migrate_settings(&raw_value, &fallback)` now returns
  `Result<SettingsMigration, jscompat::JsNumberError>`. Persist **only** on
  `Ok(migration)` with `changed == true`. On `Err`, surface the error and block
  migration persistence; there is no normalized document to save.
  `normalize_settings`, `normalize_settings_value`, and `normalize_percentage`
  also return `Result<_, JsNumberError>`. The typed normalizer constructs settings
  directly (no serde decode/expect). Fallbacks are already-usable typed settings,
  not arbitrary raw documents; downstream `Settings` structs remain constructible.
- `normalize_percentage(Option<&Value>, fallback)` distinguishes missing/undefined
  (`None`, fallback) from explicit null (zero); uses shared fallible
  `checked_js_number`, Math.round and trim semantics, with clamping to 0..100.
  JSON objects with an own non-callable `toString`, including nested array
  elements, produce the observed TypeError rather than NaN/defaults. Ordinary
  objects and non-callable `valueOf` without a shadowed `toString` still yield NaN.
- `convert_hotkey_format` and `normalize_accelerator_hotkeys` implement the **main
  process** conversion separately from settings normalization. They produce text,
  not a validated OS hotkey: registration and invalid-accelerator feedback remain
  the platform's responsibility.
- `primary_modifier_label`, `format_hotkey_for_display` and
  `hotkey_comparison_key` take an explicit navigator-style platform string. The
  shell should supply `MacIntel` on macOS, `Win32` on Windows or a Linux label;
  literal `Darwin` does not match the source's Apple-platform regex.
- `hotkey_recording_result(&KeyEvent, platform)` is the pure key-combination rule.
  `HotkeyRecorder` adds start/switch/cancel, duplicate rejection, deferred stop and
  ordered shell effects. Supply current normalized hotkeys as a JSON object to
  `handle_keydown` (or None to omit duplicate checking); its string-valued map
  contract is the same as TS `HotkeySettings`. `find_duplicate_hotkey_action`
  exposes the duplicate check independently.

Recorder shell contract:

1. On Record, pass the action string and previous display value to `record`.
   Pass None if the control cannot begin recording. Execute returned effects in
   order: Begin updates/focuses the control; SuspendGlobalHotkeys unregisters.
2. Route capture-phase keydowns through `handle_keydown`. ConsumeKeydown means
   both prevent default and stop propagation, including ignored keys. Alert shows
   the exact warning; Recorded writes the control and updates in-memory settings
   using `HotkeySettings::set`. It does **not** save or register the new shortcut.
3. Each ScheduleStop queues a 200 ms shell callback to `finish_deferred_stop`.
   End clears recording styling and optionally restores text;
   ReregisterGlobalHotkeys restores the shell's currently committed shortcuts.
   The core performs no OS calls or scheduling itself.
4. Same Record button, explicit Cancel/close, outside click, or Save invokes
   `stop()` and restores the old displayed value. Updating live preview, Save
   persistence, modal lifecycle and reset confirmation are separate UI work.

## Source quirks deliberately retained

- Legacy trimmed `backgroundColor` is a theme fallback only; nonempty `theme`
  takes precedence. Unknown top-level/hotkey/sound fields are dropped in normalized
  output. Unknown fields **alone do not** trigger persistence; any presence of
  `backgroundColor` does. Version and known fields use JS strict equality.
- Settings normalization trims hotkey strings but does not validate, deduplicate
  or canonicalize them. Missing/blank/nonstring hotkeys get the fallback.
- Main conversion replaces case-insensitive Ctrl/Control/Cmd/Command whole words
  and removes repeated exact CommandOrControl tokens. If the original string
  contains `CommandOrControl` anywhere, it skips alias conversion entirely.
  Empty `+` pieces are removed; duplicate Shift/Alt pieces are not.
- Display replacement is **case-sensitive**, leaves `Control` alone, and does not
  trim/reorder tokens. Duplicate checking formats, trims outer whitespace and
  lowercases only. Thus `CommandOrControl+Shift+Z` collides with the displayed
  equivalent; reordered modifiers/interior spaces and `Control+Shift+Z` need not.
  An unknown first matching action masks a later known duplicate in the raw TS
  recorder map; normalized settings never retains such unknown actions. Raw maps
  enumerate canonical uint32 index keys first (excluding 4294967295), then other
  keys in insertion order. Fixtures retain original JSON text so a later `"0"`
  key really tests Rust insertion order rather than JS's already-sorted output.
- A primary modifier or Alt accepts any non-modifier main key; F1..F12 also work
  unmodified. Shift-only combinations warn; plain typing is ignored. Ctrl and
  Meta together produce one primary token. Space/arrows, one-UTF-16-unit uppercasing
  (including expanding ß), punctuation and non-BMP keys follow source mapping.
  The assembled token list decides modifier presence: synthetic main key `Ctrl`
  on Win32 / `Cmd` on MacIntel is accepted even when every modifier flag is false.
- **Escape does not cancel recording in the shipped recorder.** Bare Escape is
  consumed and ignored, Shift+Escape warns, primary/Alt+Escape can be recorded.
  This is explicitly covered, not inferred from the general modal Escape rule.
  The window capture handler in `staticControls.ts` stops propagation before the
  document modal handler. To add Escape-to-cancel would require an accepted delta;
  cancellation through `stop()` is supported without changing this behavior.
- Successful input leaves recording active until its 200 ms callback. The TS
  timeout is not cancelled on switch/stop: an old callback can stop a new recording
  without restoring its placeholder. Multiple successes before the timeout are
  possible. The state machine intentionally preserves these tested timing quirks.

## Evidence and reproducible checks

`tools/golden/settings.gen.test.ts` imports real `settingsDefaults.ts`,
`settingsDomain.ts`, `main/utils/hotkeys.ts`, `hotkeyDisplay.ts`, `hotkeyRecorder.ts`
(and the real settings UI helpers). Only the DOM, navigator platform, clock/timers
and callback sinks are supplied by the harness. No business logic is mocked or
copied into an oracle. Existing main hotkey, settings-domain, settings-UI and
recorder unit tests also run unchanged in the isolated config.

Fixture: `crates/echosight-core/tests/golden/settings/parity.json`.
It records SHA-256 hashes of source modules, original tests, generator and config.
Read-only freshness check compares both provenance and all generated bytes:

```sh
SETTINGS_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.settings.config.ts
# Only after intentionally accepting source changes:
SETTINGS_GOLDEN_CHECK=0 npx vitest run --config tools/golden/vitest.settings.config.ts
cargo test --locked -p echosight-core
cargo fmt --all --check
cargo clippy --locked -p echosight-core --all-targets -- -D warnings
```

Verified in this lane:

- RED checkpoint: nine compiling Rust fixture adapters failed assertions before
  production settings code was added (`4c8a793`). GREEN uses only production APIs.
- 9 Rust settings tests; 1,302 oracle cases: 1 bootstrap, 8 platform defaults,
  260 settings/migration cases, 90 percentage cases, 60 accelerator conversions,
  5 accelerator maps, 200 display cases, 624 key combinations, 54 recorder traces.
  Rejected cases record actual TS error name/message, with no invented `changed`
  decision or serialized settings. The review-fix RED run failed all four affected
  groups (percentages, settings, key results and raw-map recorder traces).
  Settings cases check exact `JSON.stringify(..., null, 2)` bytes as well as
  values, migration decisions and normalized fixed points.
- The isolated Vitest suite passes 5 files / 16 tests (15 original tests plus the
  oracle generator). Golden case counts are not Vitest test-function counts.
- Read-only source freshness passes. A temporary copied source module was mutated
  without changing its behavior: the freshness check rejected the hash change and
  left the fixture byte-identical. No production source was modified for this probe.
- Two successive regenerations were byte-identical. Final fixture SHA-256:
  `e52e9e63594d40be4f0e7d075aff9bfc988723509ea74195a3b7d437d94a58b1`.
- Generator/config typecheck passes with the installed TypeScript compiler:
  `npx tsc --noEmit --ignoreConfig --target es2022 --module esnext --moduleResolution bundler --types node,vitest/globals --skipLibCheck --lib es2022,dom tools/golden/settings.gen.test.ts tools/golden/vitest.settings.config.ts`.

JSON Value is the supported settings-input boundary. Runtime-only JS objects,
getters, symbols, explicit nonfinite numbers and lone-surrogate strings are not
representable here; Number-coercion outcomes from JSON strings/arrays/null are
covered. On the TS side, direct nonfinite numeric inputs use the fallback; the
Rust Value API cannot receive those nonfinite numbers.
Malformed nonstring values in an *unnormalized* recorder hotkey map are outside
its typed-string contract; normalize settings before starting recording.

## Still pending (not implied by green core tests)

- Parent integration and real GPUI settings controls consuming these APIs; no
  other lane should consume this topic branch before integration.
- Live preview, slider/theme capability UI, save/cancel and reset confirmation,
  alert presentation, focus/key capture, callback scheduling and modal lifecycle.
- Global-hotkey suspend/re-register execution, accelerator registration failures,
  platform event-to-KeyEvent mapping and persisted-hotkey/menu wiring.
- Storage/bootstrap application wiring using explicit authorized user-data paths;
  real user-file and Windows/macOS/Linux runtime acceptance.

No new dependency/framework, global registration implementation, shared checkout
change, push or merge is part of this slice.
