# Theme CLI parity checkpoint (Phase 2.2)

Baseline: `46789aad6c64cb9c9615f0cb4126b565187ce5eb`. This is an authoring-tool slice, not a runtime theme chooser/renderer or approval of converted visuals.

## Implemented and exercised

- `echosight-theme validate <file-or-theme-dir>` dispatches absent/1 versus 2 through reviewed format detection and validators; invalid references, selected union-branch failures, malformed JSON, missing input and unsupported formats fail. V2 never reads CSS.
- `convert <v1-file-or-theme-dir> <new-output-dir>` calls `convert_v1(ConversionInput)`, preserves exact core output/loss reports, re-parses serialized output, copies needed supported assets and publishes the manifest last without replacing any existing output. Errors/budget exhaustion produce no publishable theme. No source/bundled rewrites or schema/core changes.
- `list <explicit-themes-directory>` enumerates ordinary immediate files/folders in deterministic order, lists variants before bases, preserves duplicate-ID insertion slots, and returns diagnostics without seeding/sync/writes.
- Human and `--json` output, explicit `--` and `--help`, exit statuses 0/1/2. `doctor` explicitly fails as unimplemented, without a fabricated scan.
- Stored canonical-root checks plus Unix directory identity checks protect static/sequential source/destination retargets; stable authorized aliases work. Private staging, exclusive output creation and create-only hard links never replace existing paths. Cleanup uses an explicit ownership ledger, not recursive deletion of caller paths.

## Verification

Fresh target directory created specifically for this work; no workspace artifacts copied. CLI executable tests first failed against the four-line stub (8 RED tests), then passed against production commands. Subsequent standalone-file and variant-override regressions also had observed RED runs. Final commands:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo check --locked -p echosight-theme-cli --all-targets --target x86_64-pc-windows-msvc
cargo clippy --locked -p echosight-theme-cli --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo build --locked -p echosight-theme-cli
python crates/echosight-core/tests/theme_v2_contract_check.py
```

Results: **255 Rust tests passed**, zero failures; one pre-existing resource-limited core budget probe remains ignored by ordinary Cargo tests. CLI contribution: **12 executable integration tests + 3 filesystem unit tests**. These counts are not theme counts. Independent schema/provenance check: **70 corpus cases, 11 converted structures, 23 original source hashes** verified.

The executable integration suite copies **all 11** frozen source JSON/CSS fixtures and real assets into disposable directories, invokes `CARGO_BIN_EXE_echosight-theme` from unrelated working directories, and compares full output structures, exact core diagnostics and exact Markdown reports. Every output parses/resolves; source byte trees remain identical. Winamp SVG and WAV copies are checked. The eleven sources also validate and appear without truncation in mixed v1/v2 read-only listing. Tests cover root/asset/CSS symlink escape, dangling existing output, late-created output, staged-write cleanup, preservation of foreign children, root alias retarget/replacement, nested declared CSS, exact-case sound filenames and variant inheritance, input/directory/diagnostic budgets and explicit unsupported variant asset overrides.

A separately built real executable was run from a new scratch working directory with non-bundled ID `author-neon-987`, altered text token `#7abc31` and button text `#d47a19`. Folder and explicit `theme.json` inputs produced equal v2 output; source bytes were unchanged; validate/list succeeded. No runtime source-tree lookup is used.

The ordinary commands also passed an independent Codex CLI uncommitted-diff review (no actionable defects; it independently ran all 15 CLI tests). That review is separate from the fresh-target workspace gates above.

## Allocation-abort regression correction

On baseline `f612a22b63de5755f2ac7fc610280637135552e4`, `convert` skipped the CLI's v1 preflight and reached unbounded legacy diagnostic path construction. Extracted that existing check into one helper used by validation/listing and conversion, before CSS/assets, legacy validation or variant materialization; the limits and validation diagnostic are unchanged. The core converter/schema and bundled files are unchanged. Direct callers of the public core converter still need separate pre-validation hardening; this checkpoint protects the CLI route only.

Two new executable tests failed before the fix and passed afterward. The Linux test runs normally under Cargo (not ignored) and requires `python3` with only its standard library. It invokes the real executable in **10 resource-limited children**: validate/convert for the reproduced amplification, a 257-byte key, excess nodes, excess depth, and an overlapping nested variant containing an overlong key. Each child has **512 MiB address space, 15 seconds CPU, core dumps disabled**, plus a 25-second wall timeout. Assertions require exit **1**, one exact structured `budget` diagnostic, empty stderr and identical source/working directory trees (including no output or staging directories). A portable executable test also verifies the shared rejection precedes missing CSS and invalid asset discovery, including UTF-8 key byte counting and read-only listing.

The original **281,741-byte** fixture is reproduced byte-for-byte, SHA-256 `d5cce2adb45fe21a4a8eb7e431a2ab38fdae8f8e839e96b37a57d6229acc705c`. Before the fix, convert exited **SIGABRT (-6)** with **296,820 KiB** peak RSS and no JSON; afterward, the same input exited **1** with **12,400 KiB** peak RSS and the budget diagnostic. All ten bounded children passed after the fix. Peak RSS is measured per child with Linux `wait4`, not inferred from the input size.

Reproduce with a fresh, unseeded target directory and retain exact fixtures/results:

```sh
export CARGO_TARGET_DIR="$(mktemp -d)/target"
cargo test --locked -p echosight-theme-cli --test cli v1_preflight -- --nocapture
cargo build --locked -p echosight-theme-cli
python3 crates/echosight-theme-cli/tests/budget_check.py \
  "$CARGO_TARGET_DIR/debug/echosight-theme" "$(mktemp -d)/evidence"
```

The full locked workspace fmt/clippy/tests, Windows CLI check/clippy and independent schema/provenance commands above passed again: **257 Rust tests passed**, zero failures, one pre-existing ignored core probe. CLI coverage is now **14 integration tests + 3 filesystem unit tests**, preserving all previous 15 tests, all 11 exact conversions/reports/assets, ordinary custom themes and base-asset inheritance. Schema/provenance remains **70 corpus cases, 11 converted structures, 23 original hashes**. Windows cross-checks do not execute the Linux resource probe or establish native Windows acceptance.

## Deliberate boundaries / remaining acceptance

- Conversion is structural and lossy. Native Electron/GPUI visual comparison remains unclaimed. This work does not change the chooser, renderer, sound player, bundled versions or data paths.
- Validation checks documents, not availability of external assets. Conversion checks every required file before writing.
- Base assets inherit into variants. Per-variant filesystem asset overrides cannot be represented by the reviewed pure API's single asset set; the adapter explicitly fails `variant-assets` rather than silently inheriting incorrect base bytes. These need manual v2 authoring or a separately reviewed core extension.
- Standalone-file variants are included in the authoring inventory, beyond the legacy runtime library's folder-only variant discovery. Missing `theme.json` candidates are diagnosed errors. Neither behavior changes the runtime library.
- JSON is strict UTF-8 JSON in this CLI; legacy storage's permissive destr loading is not invoked. The CLI's additional documented input/read/diagnostic limits fail explicitly.
- Publication is portable manifest-last, not whole-directory atomic rename. Hard-link support is required. Crashes can leave staging or an output without a manifest; no power-loss durability claim. Concurrent hostile filesystem swaps remain outside the existing ThemeRoot threat model.
- Windows target check/clippy passed; native Windows filesystem/junction/ACL behavior and linking/runtime execution have **not** been accepted. No native acceptance is inferred from cross-compilation.
- `doctor` remains out of scope and nonzero. No inferred legacy data directory.
