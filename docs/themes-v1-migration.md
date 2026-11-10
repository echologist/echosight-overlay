# Migrating v1 themes to the CSS-free format

The v1 Electron files in `data/themes` are **untouched**. Format 2 is a new, renderer-independent contract described in `themes-v2.md`. Conversion produces a reviewable starting point, not an approved port. Do not overwrite an installed or bundled v1 theme just because its converted JSON validates.

## Command-line usage

Build with `cargo build --locked -p echosight-theme-cli`; the binary is `echosight-theme`. Every path is explicit and relative paths are relative to the caller's working directory:

```sh
echosight-theme validate ./my-theme.json
echosight-theme --json validate ./my-theme-folder
echosight-theme convert ./my-theme-folder ./new-v2-folder
echosight-theme convert ./custom.json ./new-custom-v2
echosight-theme --json list ./themes
```

`--json` may occur before or after the command; `--` ends option parsing for filenames beginning with `-`. `--help` prints the usage contract. Exit codes: **0** error-free, possibly with warnings; **1** document/format/I/O/conversion failure; **2** bad arguments or unimplemented command. `doctor` returns 2 and explicitly says no scan was performed.

JSON stdout is one object with `ok`, `command`, and `diagnostics` (each has `severity`, `code`, `location`, `message`, `value`). Successful validate adds `format` and `id`; convert adds `format`, `id`, `output`, `provisional`; list adds `themes`, whose rows contain `id`, `name`, `format`, `source`, `variant` (null for base). JSON errors are also emitted on stdout. Human output sends warnings/errors to stderr. Conversion success never means CSS visual equivalence.

`validate` uses the reviewed v1 validator (including variant configs) or v2 parser/reference/union validator. It validates the document, not external asset availability; it never reads or executes CSS. V2 CSS properties are forbidden by the v2 contract. A successful v1 validation does not promise successful conversion or report conversion losses. Format absent/1 is v1, 2 is v2; unsupported formats are errors, not downgraded.

`list` is read-only: sorted immediate ordinary lowercase `.json` files and folders containing exactly `theme.json`, no seeding or bundled sync. It includes valid declared variants before their base, canonical numeric variant keys before insertion-order string keys, duplicate IDs replacing their existing row without moving it. Unlike the legacy runtime library, this authoring inventory includes declared variants of single-file themes as well. Invalid candidates produce diagnostics and nonzero status while valid rows remain available; symlink entries are not ordinary discovery entries. Missing `theme.json` in a candidate folder is a diagnosed error. Listing is not a runtime chooser.

`convert` runs the actual pure converter, reads a declared CSS file as UTF-8 data, and writes **both `theme.json` and `CONVERSION-REPORT.md`** into a new output directory. Folder inputs and explicit `theme.json` inputs discover supported immediate image/audio files; ordinary standalone JSON does not scan unrelated sibling assets. Only files referenced by converted image/sound positions are copied, byte-for-byte with exact names (including Winamp SVGs and `complete.wav`); unused images retain converter loss diagnostics. Base assets referenced by variants are inherited. Explicit nested CSS paths are supported after containment checks; v1 itself rejects nested sound filenames. No arbitrary recursion, CSS execution, font/network download or runtime build-machine paths are introduced.

The pure API has one inherited asset set, not per-variant asset-directory overrides. A supported asset under a declared `variants/<id>` folder therefore causes an explicit `variant-assets` failure with its path and no output: author that override in v2 rather than silently substituting base bytes. This boundary does not affect ordinary base-asset inheritance.

### Filesystem safety and limits

Input folders authorize their existing root; single files authorize their parent for explicitly referenced files. Every read is a bounded regular-file read through the existing `ThemeRoot` containment check plus stored-root revalidation. Asset/CSS paths reject absolute paths, traversal, backslashes, drive/stream colons and NULs; symlink escapes are rejected. Stable authorized directory aliases work. The output parent must already exist. Existing output files/directories, including dangling symlinks, are refused; folder conversions cannot write inside their source folder or to a source-containing destination. A standalone JSON file may convert to a new sibling directory.

After complete conversion, output validation and asset reads, files are staged in an exclusively-created private sibling directory and fsynced. The output directory is exclusively created, then files are hard-linked without replacement, with `theme.json` last as the publication marker. Failures clean only owned paths. Portable whole-directory atomic visibility and power-loss durability are **not** promised: a crash may leave a private staging directory or an output directory without a manifest. Filesystems without hard-link support fail rather than fall back to overwriting. Static/sequential containment is checked; concurrent hostile path swaps and native Windows junction behavior remain outside acceptance.

Limits fail explicitly rather than silently truncate: 1 MiB per JSON/CSS file, 1,024 directory entries, 32 MiB per copied asset, 64 MiB aggregate copied assets, 16 MiB aggregate list input, and 512 aggregate CLI diagnostics. V1 validation additionally bounds work before its unbounded legacy validator (4,096 nodes, 64 levels, 256-byte keys); conversion and v2 parsing retain their reviewed core allocation/diagnostic budgets. The ordinary eleven-theme corpus fits these limits. Budget/validation/conversion failures produce no publishable partial theme.

## Pure core API

```rust
use echosight_core::theme::v2::{convert_v1, ConversionInput, Theme};
# fn convert(source: &serde_json::Value, css_text: Option<&str>, asset_files: &[String])
# -> Result<(), Vec<echosight_core::theme::v2::Diagnostic>> {
let result = convert_v1(ConversionInput {
    document: source,
    css_file: css_text,
    asset_files,
})?;
let converted_json = serde_json::to_string_pretty(&result.theme).unwrap();
let report = result.report_markdown();
let _validated = Theme::parse(&result.theme)?;
// Caller writes converted_json + report into a NEW output directory,
// and safely copies assets after explicit containment checks.
# let _ = (converted_json, report);
# Ok(()) }
```

`Conversion` exposes `theme`, sorted `diagnostics` (with source locations and values), and `provisional`. A declared `cssFile` makes `provisional=true` whether or not its text was supplied. Missing CSS text produces `missing-css`; the converter never finds files itself. It also never downloads fonts, runs CSS/JavaScript, changes authored IDs/versions/authors, updates the user directory, or changes a theme based on a known bundled ID. Sounds are copied unchanged. Conversion errors return diagnostics rather than a misleading partially valid success. Exhausting the shared 512-diagnostic limit, including combined variant or output-validation diagnostics, returns `Err` with a `budget` error and retained earlier losses; no converted theme is returned.

The CLI adapter implements the explicit-root I/O boundary above; the core API remains filesystem-free. No version bump or bundled sync change is performed. Both executable and pure conversion harnesses are available:

```sh
cargo test --locked -p echosight-theme-cli
cargo test --locked -p echosight-core --test theme_v2_bundled -- --nocapture
```

The CLI harness copies all eleven frozen sources plus real declared CSS/assets into disposable roots, invokes the actual executable from an unrelated directory, compares reviewed output structures and exact Markdown reports, and verifies all copied source bytes remain unchanged. The core harness additionally runs conversion twice, resolves output and checks serialization round trips. Snapshot regeneration is explicit (`UPDATE_THEME_V2_FIXTURES=1`); review the resulting diff, never regenerate merely to make a failing test green.

## What is actually converted

| v1 source | v2 output / boundary |
|---|---|
| `id/name/version/author/description/supportsTransparency` | Preserved values; adds `format:2` |
| `colors` | `tokens.colors`, renaming top-level `background` to `surface`; bounded color parser |
| background `rgb/rgba` and shipped user-opacity idiom | Explicit `tokens.opacity.appliesTo` paths; own alpha retained |
| background hex/named colors | Not included in converter opacity list, matching v1 variable injection; no arbitrary fade inferred |
| `fonts.primary.family` | `tokens.fonts.ui.family`, comma-split fallback list; slots reference it; availability warning, no downloads |
| `fonts.primary.sizes/weights` | Numeric px/unitless tokens and header/task/progress/control references; plain button/select UI size wins conflicting component size, with a selector-precedence loss report |
| `effects.borderRadius` | Radius token used by all button slots (including icon/dialog), input/textarea/select and progressBar |
| outer `boxShadow`, `buttonShadow`, `textShadow`, `glow`, outer bevel-named values | Shadow tokens; progressBar box shadow, all button shadows, task/progress/all-button text shadows, and input/textarea/select `bevelInset` outer shadows applied. Utility-class targets require explicit slot references (`effect-utility-selector`); unsupported inset/filter values reported |
| `styles.interactive.container/header/templateSection/tasksSection` | Corresponding fixed slots |
| `styles.clickThrough.container` | `window.clickThrough` |
| `styles.clickThrough.tasksSection` | **Reported**, not silently applied to interactive `tasksSection` |
| `components.button/input/select/progressBar/progressFill/taskItem/checkbox` | Fixed slot mapping (`taskItem` → `taskRow`), supported declarations and states |
| component `states` | hover/active/focus/disabled; only an object state map suppresses sibling base declarations (`v1-ignored-base`). Null, arrays and other non-objects retain base declarations. Conflicting reversed simultaneous-state order is specifically reported |
| `main_bg` | Window images for both modes, stretch |
| `titlebar_bg` | Header image, cover (as in the actual v1 background generator) |
| `button_normal/button_hover/button_pressed` | Button base/hover/active images, cover |
| `progress_bg/progress_fill` | Progress bar/fill images, cover |
| `sounds` | Verbatim JSON; playback remains in the sound subsystem |
| `variants` | Expand with the existing v1 defu helper, convert effective data, derive a v2 patch; explicit fill opacity flags clear obsolete inherited flags. Report unsupported array replacement and inherited-key deletion |

Asset filename matching uses the caller-provided relative filenames, sorted and deduplicated. The converter does not invent nine-slice insets. Unrecognized non-audio assets are reported; audio is left to the unchanged sound/file-library contract. The pure converter never copies assets.

### CSS boundary: intentionally small, not a cascade engine

A quote/parenthesis-aware tokenizer splits rules, selector lists and declarations. Comments and whitespace-only declarations (including a trailing semicolon) are trivia, not malformed losses. Nonempty declarations without a colon still produce `malformed-declaration`. At-rules (including imports, font-face, media/keyframes), nested rules, malformed tails, unsupported selectors and unsupported declarations are reported with their source content. Nothing is executed.

Supported selectors are a fixed table independent of theme ID:

- `.overlay-container.interactive` and `.overlay-container.click-through` map to the window slots.
- Optional `.overlay-container.interactive ` prefix followed by `.header`, `.header h2`, `.template-section`, `.tasks-section`, `.section-title`, `.task-item`, `.task-text`, `.task-checkbox`, `.progress-bar`, `.progress-fill`, `.progress-text`, `button`, `.template-btn`, `.add-btn`, `.header-btn`, `input`, `textarea`, `select`, `.context-menu`, `.context-menu-item`.
- `.modal`, `.modal-content`, `.modal-title`, `.settings-title`, `.modal-content h4`, `.app-dialog-message`, `.trigger-description`, `.community-template .description`, `.import-hint`, `.settings-help`, `.hotkey-help`, `.range-labels`, `.community-template .task-count`, `.community-template`, `.modal-btn`, `.modal-btn.primary`, `.modal-btn.secondary`, `.modal-content-dialog.app-dialog-tone-danger .modal-title`.
- One final `:hover/:active/:focus/:disabled/:checked` or `.completed` suffix maps to the matching state. Descendant state selectors, combined pseudo-classes and pseudo-elements are not inferred. The unspecialized `.modal-btn` writes both dialog button slots.

Supported declarations: background (bounded color or linear gradient), background-color (bounded color), color, border (nonnegative px width + solid/dashed/dotted + color, or none), border-color/width/radius, padding/margin, height, font-size/weight/family, text-transform, letter-spacing, line-through/none text-decoration, outer box-shadow/text-shadow, outline and outline-offset. Camel-case JSON style keys are converted to kebab case. `background-image:asset:name` is bounded to images already identified by the filename mappings; arbitrary border-image requires authoring insets and is reported.

Linear gradients accept degrees or `to top/right/bottom/left`, a default CSS downward angle, optional percentage stops, and evenly interpolate omitted stop positions. Hard stops are retained; empty items (including adjacent or trailing commas) and out-of-range/decreasing stops are rejected with the original declaration in an `unsupported-property` loss, rather than repaired or clamped. Numeric lengths are px or unitless; percentages, em/rem, min/max size CSS, layout/flex/grid, transitions, filters, backdrop filters, borders on only one edge and inset shadows are reported. The v2 data contract supports radial fills and bevels, but **this converter does not infer them from arbitrary radial CSS or inset-shadow stacks**. Author those explicitly using the v2 schema.

A later `background-color` must not replace a preceding gradient image. For mapped JSON declarations or recognized CSS rules on a slot, the converter keeps the gradient and emits `background-layer-loss` with the omitted underlay color and retained fill: the fixed v2 contract has one fill, not separate gradient and color layers. A subsequent `background` shorthand still resets the fill. This bounded tracking does not add CSS specificity, inheritance or arbitrary image-layer evaluation.

The only calculated alpha syntax accepted is the shipped `rgba(..., var(--user-transparency[,fallback]))` or `calc(var(...) * number)` / `calc(number * var(...))`; the explicit reference becomes user-opacity application. Converted fills carry explicit `opacityAffects` booleans so an opaque variant/state resets a preceding generated flag, while references to generated opacity-bearing surface tokens still fade. Mixed gradients cannot preserve per-stop user-opacity behavior: `mixed-gradient-opacity` records the exact source path/value, the source per-stop opacity flags, the v2 flags and the converted fill. A true flag means that stop's own alpha is multiplied by user opacity. V2 applies the multiplier to every stop whenever any stop needs it; unaffected source stops therefore also fade. Homogeneous gradients do not receive this loss. Classification follows the selected color variable/reference/fallback, not an unused fallback containing transparency text. No new per-stop-opacity schema is introduced. Other calc expressions are losses. Supported CSS variables are known color tokens and the listed generated shadow tokens. Unknown color variables only use a supplied supported fallback, otherwise they are reported.

For each nonempty CSS source there is a `css-source-order` warning: recognized declarations are applied in source order, **not by CSS specificity/importance**. `!important` is stripped for translation. Duplicate property names are collapsed to their last occurrence and retained declarations are processed in **final-occurrence order**, so `background:red; background-color:green; background:blue` ends blue. `duplicate-declaration` records the discarded value and explicitly warns that **invalid-value fallback and `!important` precedence are not preserved**: a later invalid declaration may discard an earlier valid fallback, and an earlier important declaration may lose. These are bounded-mapper limitations, not a claim to implement a cascade engine. Grouped selectors are expanded individually, including a per-declaration diagnostic for every unsupported selector. This is sufficient to produce editable dialog/component slots, not to assert CSS cascade equivalence.

Generated JSON has separate, specific losses that are not covered by the generic CSS warning:

- `simultaneous-state-order` identifies pairs of conflicting component states whose v1 insertion order reverses fixed v2 precedence, including the source/v2 orders and merged outcomes. Individual states remain intact. Already-aligned order, disjoint declarations and identical outcomes do not receive this warning.
- `font-selector-precedence` identifies conflicting component font sizes and `fonts.primary.sizes.ui`. The late v1 font stylesheet wins on plain button/select elements; class-specific selectors keep the component size. The converter preserves the plain-element default and reports the unavailable selector distinction. Input attribute selectors and pseudo-state rules are not indiscriminately reset.
- `effect-utility-selector` identifies generated `.text-shadow`, `.bevel-inset`, `.bevel-outset` or `.glow` applications with no fixed utility-class slot. The token and known control applications are preserved; arbitrary utility-class targets need manual slot references.

Variants retain defu's override-first array concatenation. A converted CSS/string override sometimes becomes a v2 array replacement (font families or gradient stops). If the replacement cannot be expressed by defu, the converter retains the inherited array and emits `variant-array-replacement` with both requested and inherited values. Author a new named token and reference it in that variant. When the effective converted variant omits another inherited object key, `variant-inherited-value` identifies the retained value and its exact variant output path; omission is not a defu deletion. The report is part of the output contract: dropping it hides real losses.

## Eleven fixtures do not mean eleven approved themes

All eleven bundled source copies produce structurally valid v2 output and reports under `crates/echosight-core/tests/theme_v2_fixtures`. The four CSS-folder themes are explicitly provisional:

- **winamp-classic:** review SVG layering, title/button/progress scaling, borders and hard text shadows; preserve complete.wav.
- **windows-98:** hand-author raised/sunken bevel tokens from inset stacks, focus rings, pixel-font choice/licensing, exact control dimensions.
- **windows-xp:** same bevel work plus asymmetric title radii, title gradient, font and focus treatment.
- **windows-7:** hand-author Aero layers, radial caps, hard-stop gradients, checkbox glyph/pseudo-element replacements and window-level blur decision.

The other seven also require report review and native visual approval. Glass still needs an explicit system-blur choice; echosight needs licensed font assets; exiles needs radial/glow decisions; CS16 needs bevel/filter review. No blur is silently turned into a different effect, and no approximation is called equivalent.

Acceptance remains side-by-side Electron/GPUI screenshots for interactive, click-through, settings and dialogs, approved by the maintainer. Until then, do not bump bundled versions, overwrite `data/themes`, or call these converted structures “ported themes.”
