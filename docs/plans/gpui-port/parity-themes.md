# Theme parity: bounded v1 validation and pure helpers

Status: **partial Phase 1 slice; not a completed theme loader**.

Baseline: Electron v2.1.1 sources at this worktree's starting commit
`634d4ebac8462a2c84124befcdd8cda0cf069010`, with bundled themes frozen from
`main` `d96c2d5`. Exile's Corruption remains `data/themes/exiles_corruption.json`;
its discarded sounds branch is not included. No Electron source or theme visual
changes are part of this slice.

## Implemented contract

- [x] Validate raw JSON before interpretation, including primitive/array roots,
  missing required identity, null and wrong-type optional fields, nested CSS
  values, file-path checks, sound definitions, exact errors/warnings and their order.
- [x] Base validation warns on malformed variants without rejecting the base;
  variant validation checks only the shipped override fields. Non-object variant
  early returns intentionally omit the `Skipping invalid variant` warning.
- [x] Preserve all unknown fields and warning-only malformed values; no optional
  fields synthesized. The model is an open `serde_json::Map`, not a lossy typed
  schema. Replaced the interrupted, uncompiled 21 KB model and future-module
  declarations with this small representation and only implemented modules.
- [x] Asset type precedence, supported extensions, audio-extension detection and MIME
  **for genuine extension strings** (empty or leading `.`), not arbitrary JS property names.
- [x] Variant construction with identity/description fallbacks, section whitelist,
  conditional truthiness, removal of `variants`, and nonmutation of the base.
- [x] Actual installed `defu` semantics: override-first array concatenation, recursive
  object merge, ignored null overrides, retained null defaults, and override-side
  `__proto__`/`constructor` filtering (including default-side retention quirks).
- [x] Library's numeric version comparator: decimal `parseInt` prefixes, suffixes,
  signed parts, zero-padded missing components, invalid versions compare equal,
  numeric difference rather than an ordering enum or semver interpretation. Finite
  parsed components can produce an infinite difference; the oracle retains its sign.
- [x] All 11 bundled themes validate and round-trip their complete fields. Rust
  reads the current bundled files and checks against the embedded TS fixture input.

JSON boundary: these APIs accept JSON values, not arbitrary JS runtime objects.
`undefined`, functions, symbols, NaN and Infinity are not representable JSON inputs.
Missing optional fields remain distinct from explicit null. Input numbers already
represent the JSON value; this slice does not implement the storage parser.
JS whitespace and integer-key enumeration are reproduced where validation depends
on them. Extension checking follows host Node path semantics; unsafe-path rejection
recognizes both POSIX and Windows absolute paths on every host. The committed
fixtures were executed on Linux; no Windows runtime verification is claimed.

## MIME call boundary (verified, not an accepted delta)

The only production `getMimeType` call is in
`src/main/themes/themeAssetLoaders.ts:92`, passing `asset.extension`:

- `src/main/index.ts:247-251` supplies the actual library map to IPC;
  `src/main/ipc/themeIpc.ts:79-82` selects from that map by ID, not a caller-supplied
  theme object.
- `themeLibrary.ts:177-189` replaces single-file JSON `assets` with `{}`.
- `themeLibrary.ts:215` replaces folder JSON `assets` with discovery results.
  `themeLibrary.ts:234-254` builds each extension with
  `path.extname(file).toLowerCase()`, filters supported extensions, and stores it.
- Variants clone these assets; `createVariantTheme` does not apply an `assets`
  override. `themeLibrary.ts:337-343` adds only assets from the same discovery method.

Thus bare `__proto__` and `constructor` cannot reach the MIME lookup as extensions
through this production path. A real-TS temporary-filesystem regression covers
single-file/folder/variant declarations with forged extensions, extensionless
prototype-named files, and discovered assets loaded through `loadThemeAssetData`.
It verifies the existing boundary; it does not implement a Rust loader.

Direct TS calls outside that boundary do return `Object.prototype` and the `Object`
function, respectively, because `MIME_TYPES` is an ordinary object. The type-aware
`mime-boundary.json` records runtime type and identity, **not fake MIME strings or
JSON serialization of those objects/functions**. Rust tests classify these cases
as outside the extension contract and separately pin Rust's string fallback. No
arbitrary-string parity, new user-approved delta, or reproduction of unsafe JS
lookup behavior is claimed. Dotted suffixes such as `.__proto__` and `.constructor`
remain in contract and return the normal fallback in both implementations.

## Golden evidence

`tools/golden/themes.gen.test.ts` imports real `themeValidation.ts` and
`themeHelpers.ts` functions and the installed `defu`. The private version comparator
is transpiled from the exact `themeLibrary.ts` source tail and evaluated in a VM;
no replacement oracle or library I/O mock produces its expected values.
`provenance.json` schema 2 retains the original source/dependency path keys and
SHA-256 hashes, Node/platform and category counts. It also hashes the MIME caller
chain, generator/config and installed TypeScript transpiler. Check mode verifies
all source hashes before comparing every regenerated fixture byte-for-byte, then
checks schema/platform/counts. Node version is informational; checks do not rewrite
fixtures. Regression tests mutate copied TS and dependency files at their original
relative paths and prove stale hashes fail. No new dependency is needed. Plain
`cargo test` checks the fixtures, not these hashes: run the Node check gate too.

Numeric expectations use `{kind: "finite", value}` or `{kind: "nan"}`, `{kind:
"+infinity"}`, `{kind: "-infinity"}`. The real comparator matrix includes opposite
signed decimal strings of magnitude `1e308`: both components parse finitely but
subtraction overflows in each direction. Rust compares finite values exactly,
checks NaN by predicate, and checks signed infinities explicitly. The codec tests
all four tags; the current comparator matrix produces 439 finite and two infinite
results, **no NaN** (invalid/nonfinite parsed components compare equal).

| Fixture file | Cases | Rust regression |
|---|---:|---|
| validation.json | 246 | `raw_theme_validation_matches_typescript` |
| variants.json | 94 | `variant_validation_matches_typescript` |
| merges.json | 12 | `variant_creation_matches_typescript_without_mutation` |
| defu.json | 36 | `defu_merge_matches_installed_dependency` |
| helpers.json | 150 | `asset_helpers_match_typescript` |
| extensions.json | 1 | `asset_helpers_match_typescript` |
| mime-boundary.json | 25 | `mime_parity_is_bounded_to_path_extensions` |
| versions.json | 441 | `version_comparison_matches_real_library_source` |
| bundled.json | 11 | `all_eleven_bundled_themes_validate_and_preserve_fields` |
| **Total** | **1016** | **8 theme regression tests** |

Regression-first evidence: after writing the Rust oracle comparisons, executable
placeholder implementations produced **6 failed / 1 passed** tests. Failures were
wrong validation errors, variant errors, variant output, defu output, asset metadata
and version difference. The bundle pass at RED only demonstrated unchanged JSON,
not validation correctness. The real implementation then passed all 7 regressions.
The boundary/oracle follow-up first produced 3 Rust failures (bare `wav` in the
extension-only matrix, missing boundary evidence, and untyped numeric expectations).
The numeric-encoding and provenance/fixture-check regressions also failed before
the check implementations; regeneration from real TS made the Rust tests pass
without changing Rust helper behavior.

The isolated Vitest config also runs both original reference suites unchanged:

| Original TS test | Coverage |
|---|---|
| accepts a minimal valid theme | validation fixtures |
| reports invalid required fields and known sections | validation fixtures, ordered errors |
| validates optional theme sounds | validation fixtures (not playback) |
| rejects unsafe css file paths | validation fixtures |
| reports invalid variants without rejecting the base theme | validation fixtures |
| validates variant override shape before creating variant themes | variant fixtures |
| accepts all bundled themes | bundled regression |
| creates defu-merged variant themes without mutating the base theme | merge regression |
| classifies theme assets and resolves theme directories | asset classification covered; directory resolution **pending** |

## Reproduction and verified gates

Run from the repository root:

```sh
# Verify committed provenance and expected output without rewriting them:
THEMES_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.themes.config.ts
# Explicitly regenerate only when the source/boundary change has been reviewed:
npx vitest run --config tools/golden/vitest.themes.config.ts
npx tsc --ignoreConfig --noEmit --target ES2022 --module ESNext --moduleResolution Bundler --strict --skipLibCheck tools/golden/themes.gen.test.ts tools/golden/vitest.themes.config.ts
cargo test -p echosight-core
cargo fmt -p echosight-core --check
cargo clippy -p echosight-core --all-targets -- -D warnings
```

Observed: Vitest **3 files / 15 tests passed**; Rust **3 existing placement tests +
8 theme regressions passed**, no doctests; fmt and clippy clean. Generator typecheck
passed. Repeat fixture generation produced byte-identical SHA-256 hashes for all
10 JSON files (9 fixture categories plus provenance). Unrelated existing fixture
files (`validation`, `variants`, `merges`, `defu`, `extensions`, `bundled`) remain
byte-identical to the pre-review commit. No additional Cargo dependency
or lockfile changes were necessary.

## Explicitly pending (do not mark loader parity finished)

- [ ] `ThemeLibrary`: filesystem discovery/load, per-theme failure isolation,
  duplicate IDs, default selection, ordering, attaching paths/previews/docs/assets.
- [ ] Bundled sync/version/author ownership decisions and filesystem mutation.
- [ ] `getThemeDirectory`, filesystem asset discovery, asset/CSS reads, base64,
  MIME delivery, canonical containment and symlink checks.
- [ ] Sound resolution, enablement and volume mixing; controller caching/playback.
- [ ] Theme IPC / app integration and actual user-data round trips.
- [ ] Windows runtime checks and host-specific path edge-case fixtures.
- [ ] Theme v2 schema, converter, CSS interpretation, all visual/theme changes and
  screenshot acceptance. None are implemented or claimed by this slice.
