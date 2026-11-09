# v1 filesystem theme-library parity

## Delivered boundary

`echosight_core::theme::{ThemeLibrary, ThemeRoot}` loads real custom themes from an **explicit, existing, authorized directory**. `initialize(bundled_root, loaded_at)` synchronizes bundled files then discovers themes; `reload(loaded_at)` clears stale entries and rediscovers without syncing. The caller owns directory creation and supplies the ISO timestamp. There is no guessed app-data path, access to live user data, UI integration, or v2 rendering in this slice.

Open JSON maps, the reviewed v1 validator, `create_variant_theme`/defu helper and numeric version comparator are reused unchanged. Reads use `storage::parse_json`; unsupported JS-only values retain that parser's documented fail-closed boundary. Sync copies original bytes, **not** reserialized JSON: the TS synchronizer uses `copyFile`, not `writeJsonFile`. This module does not introduce another JSON writer; future authoring saves must use the existing `storage::AtomicWriter`.

### Behaviour proved against unchanged TypeScript

| Behaviour | Evidence |
|---|---|
| Single `.json` files, folder `theme.json`, invalid/corrupt/missing files, BOM and prototype-pollution rejection | `actual_typescript_filesystem_pipeline`, discovery case |
| Warnings and invalid variant messages, open unknown fields, disabled fields retained (the source has **no disabled-theme filter**) | discovery case |
| Lexical directory ordering, last duplicate wins **without moving its original Map position**, variants inserted before base | discovery case |
| Numeric variant-key ordering, array-valued variants quirk, defu maps, inherited metadata/assets, variant asset overrides | discovery case; previous helper fixtures remain unchanged |
| Safe nested variant paths, staged variants discarded when a later path escapes the folder | discovery case |
| Lowercased extensions, uppercase-suffix basename quirk (`CAPS.PNG`), duplicate asset names, preview/documentation priority, metadata | discovery case |
| All frozen bundled themes copied into isolated directories: **11 themes, 7 JSON + 4 folders** | bundled case; source bytes checked before/after in TS and Rust |
| No resurrection/migration of discarded Exile folder or sounds | bundled baseline assertions; only existing default-tree entries are synchronized |
| Sync only for matching id/author and strictly newer numeric dotted version; same/older/custom/invalid versions retained | sync case; includes suffix parsing, absent/null/object/numeric author semantics |
| Missing files/folders copied recursively; custom extra files retained; written bytes compared, not just parsed objects | sync case, complete recursive destination snapshot |
| Declared CSS only, normalized CSS requests, local URL rewrite, raw CSS asset delivery, binary/base64/MIME delivery | `actual_typescript_css_and_asset_delivery_edges` and discovery/bundled deliveries |
| JS whitespace and UTF-16 URL-rewrite quirks; `.css` dotfile is not a CSS extension | asset edge cases |
| Enabled gates, filename normalization, event gain, global/theme gain clamping and nonfinite settings fallback | `actual_typescript_sound_resolution`, `actual_typescript_nonfinite_volume_fallbacks_and_all_events` |
| Actual asset loader chained into the real TS sound controller, missing file/non-audio ignored | resolved-sound cases in `actual_typescript_css_and_asset_delivery_edges` |

The source exports only the definition resolver, not the volume helpers. The oracle therefore invokes the **actual sound controller** with a test audio sink; it does not copy private sound math. Rust provides resolution and mixing only, with no platform dependency or duplicate cache/player. Platform remains responsible for decode, audio-device ownership, cache invalidation, restart-at-zero, gain updates and playback.

`ThemeRoot::load_css` and CSS-type `load_asset` return **compatibility data only**. GPUI must never execute or apply this CSS. Token conversion, the visible v1 CSS warning, v2 visual fidelity, and bundled v2 themes remain separate work.

## Intentional containment hardening (not claimed TS equality)

The source `pathSafety.ts` is lexical-only. The master plan explicitly calls for canonical containment in the port. `theme_library_containment.rs` tests that boundary separately:

- Every exposed read is checked against the explicitly authorized root and, for assets/CSS/metadata, the theme directory. Forged outside `folderPath`, parent traversal and outside symlink targets are refused.
- An explicitly selected root alias is authorized once; lexical theme paths are preserved separately from canonical containment identities (also avoiding Windows extended-prefix mismatches). The alias regression covers folder metadata, variant assets and CSS delivery.
- Asset discovery advertises only contained **regular files**, not directories with supported-looking extensions. Source filesystem scans can advertise those directories.
- Sync rejects existing destination symlinks, including dangling symlinks and symlink ancestors, and rejects symlinks encountered inside a bundled folder. It does not recursively follow linked folders or copy outside data.
- Variant backslashes are treated as separators on all hosts, preventing a Linux-authored traversal from becoming dangerous on Windows. Safe normalized nested paths are retained.
- Raw asset paths containing `..` are rejected even if a normalization could land inside; the actual discovery pipeline never generates such paths. CSS requests are normalized before containment checks.

These are static-tree checks, **not** a hostile concurrent-filesystem sandbox: canonicalize/read has a TOCTOU window; hardlinks and malicious concurrent renames are not addressed. Roots must be user-authorized/trusted against concurrent manipulation. Sync is byte-preserving but not a directory transaction: an I/O failure can leave earlier copied files in place. The Rust sync API returns an error and initialization records it then still loads themes; exact Electron OS-specific exception strings and recursive-copy failure continuation are not claimed as parity.

No `CHANGELOG.md` or master-plan edits were made in this exclusive lane. The integrating owner should carry the above hardening into the accepted-deltas/release documentation.

## Oracle and gates

- `tools/golden/theme-library.gen.test.ts` imports and runs the actual TS library, loaders and renderer sound controller with real temporary files.
- `tools/golden/vitest.theme-library.config.ts` additionally runs the existing library, asset-loader and sound unit tests unchanged.
- Fixture: `crates/echosight-core/tests/golden/theme-library/pipeline.json`. Three filesystem scenarios, 80 definition/gate/mixing cases, 24 tagged-volume/event cases, 10 direct asset/CSS cases, and four real filesystem sound resolutions. These are **case counts**, not test-function counts.
- Original source/dependency/bundled-file SHA-256 hashes are recorded, as are generator/config hashes. Temp roots are replaced with `$ROOT`; timestamps are fixed. Each generator invocation executes twice and compares exact fixture bytes.
- Check mode validates provenance **before** generation and compares without writing. Separate mutation probes alter only copied TS/defu/destr files and demonstrate freshness rejection. Original bundled/source hashes are checked after execution.
- A compiling red Rust adapter failed before implementation. Additional real-oracle regressions caught and corrected CSS JS whitespace/UTF-16 semantics, hidden `.css` extension detection and numeric-author strict equality.

Run from the topic worktree (installed npm dependencies required):

```sh
# Read-only first; intentionally fails if sources/dependencies/fixtures are stale.
THEME_LIBRARY_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.theme-library.config.ts
# Only when deliberately updating reviewed fixtures:
npx vitest run --config tools/golden/vitest.theme-library.config.ts

cargo fmt --all --check
cargo clippy --locked -p echosight-core --all-targets -- -D warnings
cargo test --locked -p echosight-core
cargo check --locked -p echosight-core --all-targets --target x86_64-pc-windows-msvc
THEMES_GOLDEN_CHECK=1 npx vitest run --config tools/golden/vitest.themes.config.ts
```

Verified locally on Linux: **45 core tests**, core clippy, workspace formatting, theme-library freshness/determinism/mutation checks, previous themes read-only freshness, targeted TypeScript typecheck, and core Windows cross-target check (all targets, no warnings). The new lane has nine Rust test functions; the isolated TS config has 13 tests. Windows runtime/filesystem acceptance and platform audio playback are **not verified** here. Existing theme golden fixtures and Electron sources are unchanged.
