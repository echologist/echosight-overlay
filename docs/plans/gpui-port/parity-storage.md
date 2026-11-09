# Core storage: bounded compatibility slice

Status: implemented and tested on Linux; Windows all-targets type-check passes.
**Not a full Phase 1 parity verdict. No live user-data directory was read or written.**
Baseline: topic branch `gpui-port-core-storage`, starting at `634d4eb`.

## API and ownership

- `DataPaths::new(explicit_data_directory)` resolves an explicitly authorized path;
  construction neither discovers app identity nor creates directories. Fixed children:
  `current_tasks.json`, `templates.json`, `settings.json`, `themes/`.
- `Storage::new(paths)` owns the load-failure block map. Clones share that map;
  separate instances model separate IPC registrations. Use one runtime instance.
- `load(FileKind)` returns a `serde_json::Value`, including TS's tasks-only
  `corruptBackupPath` property after successful quarantine/replacement.
- `save(FileKind, &Value)` returns `Result<(), StorageError>`; the shell can map this
  to `{success:true}` / `{success:false,error}`. No IPC transport is implemented here.
- `bootstrap(default_data_directory, &settings_value)` seeds only missing runtime
  files; it does not define or normalize Task/Settings models. Bundled settings are
  never read. The domain/settings owner must inject its normalized defaults.
- `AtomicWriter::write(path, &Value)` is the lower-level writer. It intentionally
  has no load-failure policy: normal runtime saves must go through `Storage`.
- `StorageHooks` provides clock and pre-IO fault/ordering seams. Default hooks use
  real filesystem operations and UTC time. Hooks are not a virtual filesystem.
  Do not reenter the same path from a hook.

The formatter remains `storage::json_format`, independent of the parallel
`jscompat` workstream. Reconcile the shared formatter only during integration.

## Verified behavior and evidence

Authoritative TS: `src/main/utils/jsonStorage.ts`, `src/main/ipc/storageIpc.ts`,
`src/main/storage/runtimeDataBootstrap.ts` and their three existing unit suites.
The generator imports and executes those actual modules, not copies of their logic.

`tools/golden/storage.gen.test.ts` writes
`crates/echosight-core/tests/golden/storage/storage.json`. It records SHA-256 hashes
of those TS sources and the installed `destr/dist/index.mjs`, plus destr **2.0.5**.
Fixtures contain 51 parser cases, 9 real writer byte outputs, 3 missing-file results,
9 quarantine cases (including invalid UTF-8 bytes), and 4 bootstrap scenarios.
Undefined/nonfinite/unpaired-surrogate results are classified from actual returned
JS values, rather than converted to invented JSON oracle values.

- Parser: case-insensitive top-level booleans/null; JS whitespace rules; raw quoted
  fast-path quirks (including a lone quote, embedded quotes and raw newline);
  strict invalid/truncated input; numeric signature bounds; nested/escaped
  `__proto__` and `constructor` rejection; duplicate properties; large JS integers.
  The plan's simple “trim and JSON.parse” description is not the actual destr API.
- Formatter: two spaces, LF, no final newline/BOM, insertion order after numeric
  array-index key order, Unicode/control escaping, fractional ID
  `1749938954072.5596`, negative zero and common exponent boundaries. `ryu-js`
  renders the JS f64 numeric domain; serde_json's ordinary pretty printer does not.
- Atomic save: exclusive same-directory `.<name>.<pid>.<uuid>.tmp`, full write,
  `sync_all`, close, replacement; cleanup does not hide the original error.
  Failed write/flush/close-hook/replacement leaves destination bytes untouched.
- Process-wide FIFO tickets serialize resolved lexical paths, including across
  writer instances. A failed write does not poison the following write. Unrelated
  paths proceed independently. Channel gates, not sleeps, synchronize tests.
- Missing reads do not create files or parents. Non-ENOENT read errors do not
  quarantine. Corrupt bytes are copied before publishing the domain fallback.
  Tasks expose backup metadata; templates/settings return only their values.
- Failed read, backup or fallback publication blocks that domain's saves, without
  further IO, until a successful reload. A missing-file reload also clears the
  block. External repair alone does not. Save failure alone does not set a block.
- Bootstrap preserves existing bytes even if empty/corrupt; bundled task/template
  read/parse failures use empty fallbacks; supplied settings are used unchanged.
  Create-only publication cannot overwrite a competing creator after the check.

## Deliberate bounds / integration decisions still open

1. **App directory identity is unresolved.** The plan's
   `%APPDATA%\\echosight\\data` is an inferred path, not proof of the packaged
   application's runtime identity. Absence of `%APPDATA%\\echosight` does not prove
   an alternate productName. No probing, migration or first-of-multiple-root
   selection exists here. Obtain packaged `app.getPath('userData')` evidence or an
   explicit user-selected directory; if several legacy roots exist, ask rather
   than select silently. `ECHOSIGHT_DATA_DIR` is only a possible future test seam,
   not an override present in the current TS app.
2. **Value cannot represent all JS.** `undefined`, NaN/infinities (also numeric
   overflow), lone UTF-16 surrogates and serde's excessive nesting are unsupported.
   Loading them fails closed and blocks saves **without quarantine or fallback**;
   it does not pretend they are corrupt JSON. Hex-escape parser errors are also
   conservatively fail-closed because serde uses that error for unpaired high
   surrogates. Error text is Rust-specific, not a V8 message parity promise.
   Unsupported bundled values use the bootstrap error fallback. Full JS value
   semantics remain outside this slice.
3. Serialization coerces Rust JSON numbers to f64, matching JS rather than
   preserving arbitrary-precision Rust integers. Common/fractional values and
   the checked-in samples are byte-verified, not an exhaustive binary64 proof.
   The crate requires serde_json's existing `preserve_order` + `float_roundtrip`;
   do not enable `arbitrary_precision` without revisiting the formatter.
4. Locks are in-process only, lexical rather than symlink/inode or case-folded
   identity. Callers must not concurrently edit the same file from another
   process/alias. Loads and recovery share the writer queue here, a stronger
   ordering boundary than TS's write-only queue. Blocking APIs belong off the UI
   thread. Bootstrap's atomic hard-link publication is a stricter implementation
   of create-only intent than TS's exists-check + replace race. It fails closed on
   filesystems without hard-link support; no destructive fallback is attempted.
   Access-check errors fail rather than being treated as absence. These safety
   boundaries need shell/integration acceptance, not an implied release delta.
5. As in TS, backup names use millisecond UTC timestamps and copy can overwrite a
   same-name backup on a repeated same-millisecond recovery. Backup collision
   policy is not redesigned here. Backup copies and parent directories are not
   fsynced; atomic visibility does not promise full power-loss durability.
   Rust `File` drop cannot report an OS close error; the mandatory file fsync is
   real, while the close-error path is exercised through the injected hook.
   Failed cleanup may leave a temp file and does not change the decisive result.
6. No real-user-data compatibility claim, Windows runtime pass, migration,
   settings normalization, UI/IPC wiring or platform discovery is included.
   The parent integrator regenerates root `Cargo.lock` after combining core
   dependency changes; that shared file is intentionally outside this lane's
   commits. The added numeric dependency resolved to `ryu-js 1.0.3` in verification.

## Windows replacement: explicit check and runtime gate

Rust 1.99.1's `std::fs::rename` implementation was checked directly in
`rust-lang/rust` tag `1.99.1`, `library/std/src/sys/fs/windows.rs`, function `rename`:
first `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)`, then on access denied a
`SetFileInformationByHandle(FileRenameInfoEx)` fallback. This is not an assumption
that Unix rename semantics apply everywhere. No destination-delete workaround or
custom core FFI is used.

`creates_and_overwrites_existing_destination_with_ts_bytes` is cross-platform and
replaces an existing target repeatedly. The Windows-only
`windows_open_destination_without_delete_sharing_blocks_replace_without_deleting_it`
test holds a target without `FILE_SHARE_DELETE`, expects replacement failure and
unchanged old bytes/no temp leak, then drops the handle and expects success.
Both tests compile for Windows; **neither was executed on Windows in this lane**.
Run the following on a Windows NTFS checkout before claiming Windows runtime parity:

```powershell
cargo test -p echosight-core --test storage_atomic
cargo test -p echosight-core --test storage_bootstrap
cargo test -p echosight-core --test storage_recovery
```

## Reproduce

Run from the repository root. Tests create isolated temporary directories under
`TMPDIR`; for an agent run, point it at the agent scratch directory, not live data.
Use the existing package-lock installation; the generator needs no Electron launch.

```sh
node_modules/.bin/vitest run --config tools/golden/vitest.storage.config.ts
cargo test -p echosight-core
cargo fmt --all --check
cargo clippy -p echosight-core --all-targets -- -D warnings
cargo check -p echosight-core --all-targets --target x86_64-pc-windows-msvc
```

Observed Linux results: TS **4 files / 20 tests passed** (generator plus unchanged
original storage suites); Rust **23 tests passed** (20 storage + 3 placement).
Windows all-targets check succeeded without the GPUI shim. Formatting and core
all-targets clippy passed. Fixture regeneration is byte-stable (SHA-256
`d0edd897859713115cdffeab650441a52331fd43f790076bf7348adcaf1db1f7`).
A separate TypeScript `tsc --noEmit` check was attempted but the execution guard
refused to scan TypeScript's >1 MiB `_tsc.js`; no standalone TS type-check pass is
claimed. The generator and original suites did execute successfully in Vitest.
Initial Rust
feature tests were run red against explicit unimplemented API scaffolding before
implementation; the lone-quote oracle addition also reproduced a concrete red
parser result before its fix.
