# Theme v2 parity ledger — bounded core checkpoint

Base: `f1f8b21`; topic: `gpui-port-theme-v2`. This lane owns only new core `theme/v2`, the module export, v2 schema/tests/fixtures, and these new docs. It does not change Electron, v1 loading/sync/sounds/helpers, app/UI wiring, dependencies, bundled data, or existing golden fixtures.

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
| Authoring-schema consistency | 36 shared acceptance cases checked by Rust and independent Python JSON Schema 2020-12 validator; all slot names and supported style properties exercised |
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

Results at this checkpoint: **100 Rust test functions passed**, including **18 new v2 test functions**. The bundled test iterates eleven distinct source themes; the schema test iterates 36 acceptance cases. These are separate counts, not hundreds of independently named tests. Independent Python checker: 36 corpus cases, eleven converted structures, 23 source-file hashes verified. Clippy and formatting pass. No Cargo manifest/lockfile delta. Python `jsonschema` is a development-only independent checker, not a new runtime/Cargo dependency.

Frozen source copies intentionally preserve the original trailing whitespace (CS16 JSON and 98/XP CSS). Fixture-local `.gitattributes` disables whitespace lint only for copied `source.json`/`source.css`; their stronger byte/hash checks still run. All authored code, converted outputs and reports retain normal whitespace checks. Do not trim the copied sources to silence `git diff --check`.

The v2 tests cover malformed nested values without panics; metadata/asset strings beginning with `@`; unknown reference-like properties remaining warning-only; missing references, token/slot loops, deep inputs; inherited and simultaneous states; fill union replacement; opacity exclusions and no text fading; images and nine-slice validation; bevels; preserved v1 component-state quirks; custom token influence; reports for unknown selectors/properties/fonts/effects; variant colors/defu behavior; absent CSS and invalid converter inputs.

## Frozen conversion reports

Counts include loss warnings, font-availability notices, CSS-order caveats and unmapped metadata. They are **not** a visual-fidelity score.

| Source ID | Report entries | Provisional CSS-folder flag |
|---|---:|---|
| cs16 | 49 | no |
| dark | 23 | no |
| echosight | 145 | no |
| exiles_corruption | 39 | no |
| glass | 44 | no |
| light | 22 | no |
| minimal | 27 | no |
| winamp-classic | 74 | **yes** |
| windows-7 | 255 | **yes** |
| windows-98 | 124 | **yes** |
| windows-xp | 107 | **yes** |

Reports and converted structures live only under `tests/theme_v2_fixtures/<id>/`. Versions and authored IDs are unchanged. The exiles source is the actual single-file baseline, not the discarded sound-branch variant described by the older inventory.

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
