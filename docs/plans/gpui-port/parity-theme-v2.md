# Theme v2 parity ledger — bounded core checkpoint

Base: `f1f8b21`; topic: `gpui-port-theme-v2`. This lane owns only new core `theme/v2`, the module export, v2 schema/tests/fixtures, and these new docs. It does not change Electron, v1 loading/sync/sounds/helpers, app/UI wiring, dependencies, bundled data, or existing golden fixtures. The converter correction branch `gpui-port-theme-v2-converter-fixes` starts at `889c02f` and changes only the converter, conversion/bundled tests, converted snapshots/reports and migration/parity docs; validator/resolver/schema corrections are a separate lane. The follow-up `gpui-port-theme-v2-css-fixes` starts at combined `0db66023fc8a6581d7b5078fe2d09b4fbf296b70` and owns only CSS/converter quality corrections, converter tests, changed conversion reports and these migration/parity docs.

## Delivered and exercised

| Area | Evidence |
|---|---|
| CSS-free authoring contract | `schema/theme-v2.schema.json`, interpreted directly by core; `docs/themes-v2.md` |
| Raw-value validation / safe diagnostics | Required metadata, shape/range/color/path checks, unknown-key warnings, CSS rejection, malformed-value matrix, depth/node limits |
| References and inheritance | Missing/cyclic refs/extends rejected; alias paths, inherited states, replacement versus defu semantics tested |
| State and opacity resolution | Six-state deterministic precedence, fill-only opacity, exclusions, inherited/aliased surface tokens, referenced mode opacity |
| GPUI-facing API | `Theme::slot` -> `ResolvedStyle`; typed fills/gradient stops/text/borders/bevels/images/shadows; resolved supported JSON for remaining style fields |
| Conversion | Pure `convert_v1(ConversionInput)` -> validated `Conversion`, deterministic diagnostics/report; no CSS execution or filesystem side effects |
| Source-driven behavior | Test changes custom theme tokens under a non-bundled ID and observes changed resolved output; no static ID lookup |
| Bundled structures | All eleven source copies convert, validate, resolve and JSON-roundtrip; output/report snapshots checked without regeneration |
| Source integrity | Eleven source JSON and four CSS files compared byte-for-byte; independent SHA-256 manifest verifies all 23 files in original `data/themes` |
| Authoring-schema consistency | 70 shared acceptance cases checked by Rust and independent Python JSON Schema 2020-12 validator; all slot names and supported style properties exercised |
| Previous core behavior | Full core suite and existing task/template/settings/storage/theme/library golden consumers pass; existing golden bytes unchanged |

## Executed gates

```sh
cargo test --locked -p echosight-core
cargo clippy --locked -p echosight-core --all-targets -- -D warnings
cargo fmt --all --check
git diff --check
python crates/echosight-core/tests/theme_v2_contract_check.py
cargo test --locked -p echosight-core --test theme_v2_bundled -- --nocapture
```

Results after the CSS quality corrections on the combined candidate: **132 Rust test functions passed**, including **19 converter test functions** (the previous 16 plus three quality regressions with their controls). The bundled test iterates eleven distinct source themes; the schema test iterates 70 acceptance cases. These are separate counts, not hundreds of independently named tests. Independent Python checker: 70 corpus cases, eleven converted structures, 23 source-file hashes verified. Clippy and formatting pass. No Cargo manifest/lockfile delta. Python `jsonschema` is a development-only independent checker, not a new runtime/Cargo dependency.

Frozen source copies intentionally preserve the original trailing whitespace (CS16 JSON and 98/XP CSS). Fixture-local `.gitattributes` disables whitespace lint only for copied `source.json`/`source.css`; their stronger byte/hash checks still run. All authored code, converted outputs and reports retain normal whitespace checks. Do not trim the copied sources to silence `git diff --check`.

The v2 tests cover malformed nested values without panics; metadata/asset strings beginning with `@`; unknown reference-like properties remaining warning-only; missing references, token/slot loops, deep inputs; inherited and simultaneous states; fill union replacement; opacity exclusions and no text fading; images and nine-slice validation; bevels; preserved v1 component-state quirks; custom token influence; reports for unknown selectors/properties/fonts/effects; variant colors/defu behavior; absent CSS and invalid converter inputs.

## Frozen conversion reports

Counts include loss warnings, font-availability notices, CSS-order caveats and unmapped metadata. They are **not** a visual-fidelity score. Programmatic comparison against `889c02f`: 909 → 688 entries, exactly 232 false empty `malformed-declaration` entries removed and 11 precise effect utility-selector notices added; every other diagnostic-code count is unchanged. That prior correction intentionally regenerated all eleven snapshots. The follow-up quality correction compares against `0db6602`: **688 → 689** entries, exactly one new `mixed-gradient-opacity` warning for exiles_corruption; all eleven theme payloads are unchanged. Baseline snapshots and then all eleven post-fix conversions/reports were checked read-only (each conversion twice) before the intentional update. Only the exiles converted wrapper/report changed; all eleven final snapshots were verified read-only with `UPDATE_THEME_V2_FIXTURES` unset.

| Source ID | Report entries | Provisional CSS-folder flag |
|---|---:|---|
| cs16 | 39 | no |
| dark | 12 | no |
| echosight | 117 | no |
| exiles_corruption | 30 | no |
| glass | 34 | no |
| light | 11 | no |
| minimal | 17 | no |
| winamp-classic | 49 | **yes** |
| windows-7 | 211 | **yes** |
| windows-98 | 91 | **yes** |
| windows-xp | 78 | **yes** |

Reports and converted structures live only under `tests/theme_v2_fixtures/<id>/`. Versions and authored IDs are unchanged. The exiles source is the actual single-file baseline, not the discarded sound-branch variant described by the older inventory.

## Source-observed converter regression provenance

The eight `observed_*` tests in `theme_v2_conversion.rs` were first run against production APIs at `889c02f4f7cd7d0ad754de54bfbbe9289267073c`: **eight failed, four pre-existing tests passed**. They now pass. Expectations come from the unchanged TypeScript modules below, executed with installed TypeScript/defu, then inspected through Chrome computed styles—not from a handwritten CSS oracle. The same TS/Chrome harness was rerun from this correction worktree against 16 base/variant cases. These are browser style-engine observations, **not GPUI or native visual acceptance**.

Paths in this table are under `src/renderer/features/themes/` at that frozen revision; no Electron source is modified.

| Regression input / source authority | Observed v1 result | Converter outcome / genuine limit |
|---|---|---|
| User-opacity RGBA window → variant `#ffffff`; `themeCssBaseStyles.ts` `generateInteractiveStyles`, main `themeHelpers.ts` `createVariantTheme` | At user opacity .25, opaque variant computes `rgb(255, 255, 255)` | Explicit false clears inherited generated fill opacity; transparent surface-token variants still fade |
| Component background `linear-gradient(red, blue)` followed by `backgroundColor: green`; same `.header` custom CSS; `themeCssComponents.ts` `generateComponentProperties`, `themeCssBaseStyles.ts` `generateCustomCSS` | Background image remains `linear-gradient(rgb(255, 0, 0), rgb(0, 0, 255))`, with `rgb(0, 128, 0)` color underneath | Retain gradient, report the unrepresentable color underlay (`background-layer-loss`); also guarded across recognized CSS rules |
| Component state insertion order active:red, hover:blue; `themeCssComponents.ts:28-45` | Simultaneous active+hover computes blue | Fixed v2 precedence still yields red; `simultaneous-state-order` explicitly records both orders/outcomes, rather than claiming parity |
| Primary UI font size 12px and button component 20px; `themeApplicator.ts:35-36`, `themeRuntime.ts:121-124`, `themeCssSelectors.ts:22-29` | Plain button 12px; `.template-btn` 20px | Preserve plain button 12px; `font-selector-precedence` reports collapsed class-specific outcome |
| `bevelInset: 1px 2px red`, radius 9px, text shadow; `themeCssBaseStyles.ts:48-89,120-159` | Input/textarea/select outer shadow `rgb(255, 0, 0) 1px 2px 0px 0px`, radius 9px; buttons receive text shadow | Apply known control/button-family targets; report arbitrary generated utility-class targets only |
| `titlebar_bg.svg`; `themeCssBackground.ts:71-102` | Header background-size `cover` | Emit header image mode `cover`, not stretch |
| Button color red, states:null; `themeCssComponents.ts:23-50` and `themeCssHelpers.ts` `isRecord` | Base red preserved; only a record activates the state branch | Preserve base for null/array/scalar states; never falsely emit `v1-ignored-base` |
| `button { color: red; }`; actual generated stylesheet appended to Chrome | Valid red declaration | Ignore empty declaration trivia; retain errors for nonempty malformed declarations |

The simultaneous-state probe uses CDP `CSS.forcePseudoState(['hover','active'])` on the actual generated stylesheet. DOM `matches(':hover')` / `matches(':active')` remain false; the computed background is blue. This tests CSS evaluation, not event dispatch.

Source SHA-256 values (read-only provenance, not replacement fixtures):

```text
themeCssComponents.ts 9a26ddbb81667b9daa1f7b1c6044f707aa2290d513bf677a4caab506c0cdd2bb
themeCssBaseStyles.ts 3217139b77df068f09934f4ca11ea56fc17837e854b9dd9b0d7ec1bd87fa0a2c
themeCssBackground.ts a52d46b7e9291619d97849ae3bf0f8a248139317d48e807173ee63641f2f5d23
themeRuntime.ts 3b778944f5438a0ee609ac75dd422a6c71e94eebec599bb2ef1b8b648c2cf7c0
themeApplicator.ts 3a0a3ce9e2747e652fa9cadd13afae2d702e396eb76f3f4e85f06b9c2a998c84
themeCssSelectors.ts 099203ceacdaf38f501e74eaaa4936960a500c61a50f3573e2b2aa13e3d09c9b
```

Additional production-API guards cover transparent token variants and opaque state overrides; omitted inherited values with `variant-inherited-value` and the existing legitimate `variant-array-replacement`; background shorthand reset/order. Controls keep aligned/disjoint/identical state outcomes warning-free, preserve quoted URL delimiters and full unsupported font-face content, and verify source input immutability. The quality correction makes the shared tokenizer preserve comma-list empties; empty declarations are still ignored at the declaration boundary and whitespace-separated lengths still coalesce repeated whitespace. Empty gradient stops are rejected, not repaired.

## Source-observed CSS quality closures

All three new named production regressions were run before implementation on `0db6602`: **three failed, the previous 16 passed**. Separate production probes plus unchanged TS generators and Chrome reproduced the same three failures. After the fixes, all three and all eight original regressions pass. The six source hashes above still match. The actual-source harness reran the original 16 base/variant cases, 11 quality/control base/variant cases, and 18 additional controls; Chrome/CDP results are browser style-engine evidence, not GPUI/native acceptance.

| Regression / test in `theme_v2_conversion.rs` | Actual v1 observation | Bounded closure |
|---|---|---|
| `observed_mixed_gradient_opacity_reports_per_stop_loss` | At .25 opacity, `linear-gradient(var(--bg-primary), red)` with primary rgba alpha .8 has stop alphas `[.2, 1]`; direct user-transparency rgba + red has `[.25, 1]` | Typed `Theme::slot(...).fill()` still yields `[.2, .25]` / `[.25, .25]`. Exact-path `mixed-gradient-opacity` records the source value, intended per-stop multiplier flags, actual v2 flags and converted fill. The v2 contract has one fill alpha, not independent user-opacity flags per stop. |
| `observed_empty_gradient_items_are_rejected_without_repair` | Adjacent/trailing/leading/whitespace-only gradient items compute `backgroundImage:none` | Reject rather than silently producing two valid stops. `unsupported-property` retains the declaration and its exact JSON/CSS source path. Nested rgb commas, quoted family commas, repeated length whitespace and empty declaration trivia remain valid controls. |
| `observed_duplicate_css_properties_keep_final_occurrence_order` | `background:red; background-color:green; background:blue` computes blue | Removing a replaced property before reinserting it retains final-occurrence order. Reverse longhand interleaving and border/border-color controls also match Chrome. Duplicate losses explicitly disclose unsupported invalid-value fallback and `!important` precedence, in addition to the existing generic CSS importance warning. |

Homogeneous all-token and all-opaque gradients are warning-free for mixed opacity. Active nested variable fallbacks are classified by their selected source; unused fallback transparency text cannot cause opaque stops to fade. Both JSON properties and recognized CSS declarations are exercised. Chrome controls also confirm the still-unsupported duplicate cases: `background:red !important; background:blue` and `background:red; background:bogus` stay red in v1, whereas the bounded mapper discards the earlier declaration and reports that limitation. No full cascade engine or new schema semantics are claimed.

## Remaining gates / deliberate limits

- **No visual parity established.** No GPUI style application, native screenshots or Windows acceptance performed by this core lane.
- Parent integration must wire format dispatch, CLI safe input/output, theme chooser warnings, runtime caching and GPUI slot-to-widget mapping. Existing v1 filesystem behavior is untouched.
- All four CSS-folder themes require manual authoring and maintainer approval: winamp skin layering/shadows; 98/XP bevels, focus rings, fonts/dimensions; Windows 7 radial caps, Aero layering and pseudo-element replacements. The seven JSON themes also require conversion-report and screenshot review.
- Converter is a small tokenizer/table mapper, not CSS cascade/specificity. Inset stacks, arbitrary radial CSS, layout, border sides, transitions, filters/backdrop filters and unsupported selectors remain explicit losses. The v2 contract can represent radial fills/bevels for manual authoring.
- No font files are bundled or downloaded; no system blur inferred; no image insets guessed. Asset existence, MIME and canonical containment remain the filesystem adapter's job.
- Mode-specific section styling cannot be represented by the fixed `tasksSection` slot alone; converter reports click-through section differences instead of overwriting interactive style.
- Variant array replacement cannot be expressed by defu; reports require a new named token/reference rather than a silent concatenation masquerading as replacement.
- Do not bump bundled versions or overwrite Electron themes until the separate manual port/acceptance gate passes. `docs/themes.md` remains v1 until cutover.

API instructions and precise syntax boundaries are in `docs/themes-v2.md` and `docs/themes-v1-migration.md`. The first commit is the runnable validator/resolver checkpoint; the next adds the converter and independently checked source/report fixtures.
