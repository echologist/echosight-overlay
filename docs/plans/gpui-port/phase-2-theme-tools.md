# Phase 2.2 — explicit-root theme tools

Baseline: `46789aad6c64cb9c9615f0cb4126b565187ce5eb`. Scope is the CLI crate and migration/parity documentation, not the runtime chooser, renderer, bundled assets or core behavior.

## API anchors and execution

1. Add executable RED tests against `CARGO_BIN_EXE_echosight-theme`, running from unrelated disposable directories. Cover format dispatch, real source-driven conversion and frozen eleven-theme expectations, readonly listing, diagnostics, filesystem containment and publication failure.
2. Implement minimal std-args `validate`, `convert`, `list`, `--json`, `--help`, and `--` paths. Dispatch with `theme::v2::detect_format`; validate v1 with `validate_theme` / `validate_theme_variant_config`, v2 with `Theme::parse`. Never interpret CSS during v2 validation. Doctor explicitly fails as unimplemented.
3. Authorize explicit input roots with existing `ThemeRoot::new/existing`; bound regular-file reads, directory enumeration and diagnostic work. Resolve folder inputs to exactly `theme.json`. List ordinary lowercase `.json` files and folders in filename order, variants before base, replacing duplicate IDs in their original slot as the existing library does. No initialize/sync or inferred data paths.
4. Feed real JSON, declared CSS text and supported immediate asset filenames to `convert_v1(ConversionInput)`. Preserve its output and exact Markdown loss report; gather required image/audio assets including inherited variant references. Reject unsafe/missing paths, unsupported formats and budget errors before creating output.
5. Stage complete payload in an exclusively-created private sibling directory, fsync files, and publish into an exclusively-created output directory with `theme.json` last. The existing `AtomicWriter` replaces destinations, so cannot publish a no-replace directory; use its exclusive-create/fsync and hard-link create-only publication pattern, never its replacing `write` API. Revalidate stored parent/root identities before writes; refuse existing paths including dangling symlinks and destinations within the source root. Cleanup only explicitly owned files/directories. Whole-directory atomic visibility is not promised by portable std filesystem APIs; no theme manifest is visible before all assets/report are complete.
6. Run locked CLI/workspace tests, fmt, clippy and Windows CLI cross-check in a new target directory. Document native Windows/junction and concurrent hostile filesystem race limitations separately from verified static/sequential containment. Commit exact owned paths only.

## Acceptance boundaries

Successful v1 validation is not conversion/visual approval. Conversion warnings remain visible in human/JSON output and the report. Unsupported selectors/effects remain reviewed core losses. All eleven source fixtures and real assets must retain their bytes; output must match reviewed structures/reports. Conversion errors, including the shared 512-diagnostic budget, must not leave a publishable partial theme. No arbitrary CSS execution, remote downloads, autoseeding, runtime machine paths, version bumps, doctor scan or changes to the reviewed core APIs.
