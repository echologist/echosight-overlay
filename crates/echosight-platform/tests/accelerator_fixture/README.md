# Accelerator adapter evidence

## Boundaries

`hotkeys::accelerator::electron_accelerator(main_normalized, target)` takes the
unchanged core main-normalizer output, an explicit Windows/Linux/Mac target,
and returns `HotKey::new` identity or a typed error. It does not read or write
settings, create a manager, register keys, dispatch actions, or implement a
registry. Persisted text, renderer comparison, and native ownership stay separate.

`AcceleratorError::is_conversion_failure()` is true only for non-ASCII input or
an unknown/missing final key. Those correspond to Electron's conversion failure
and must eventually abort its outer registration loop. Valid-but-unresolved
platform diagnostics are **not** conversion failures or observed OS failures;
continuing after them is a temporary adapter policy, not Electron behavior.

## Independent evidence lanes

- `observations.rs` includes 406 actual Electron raw/main-normalized pairs,
  nine upstream C++ test-case observations, and 2,709 upstream helper-string
  acceptance observations. They were executed by Electron **41.3.0**, Chromium
  **146.0.7680.188**, on Linux using `globalShortcut.isRegistered` without native
  registration. The adapter tests consume raw text through the unchanged core
  normalizer and also test the normalized strings directly. Windows/Mac parser
  acceptance is inferred from the platform-independent source, not executed on
  those operating systems.
- The same fixture separately contains **154 source-table rows**: 80 named
  lookup entries and 74 character-switch entries. Their chord expectations come
  from `keyboard_util.cc` joined to the pinned Rust Windows backend by exact
  VKEY/VK identity, **not** from menu display or parser acceptance. Aliases in
  the backend sharing a VK use the first canonical arm (e.g. Enter, not
  NumpadEnter; Backslash, not IntlYen).
- **Known research defect:** the original `token-mapping.csv` marks every named
  token as implicitly shifted. It is not read by the extractor. Pinned source
  gives implicit Shift only to named `plus`. Both source-table checks and named
  Shift-negative controls would reject that erroneous column. Original evidence
  and manifests were preserved unchanged.
- `provenance.json` retains only relevant source hashes, public URLs, the
  repository-relative main-normalizer hash, and the original observation hash.
  `supplemental-provenance.json` pins the exact Chromium ASCII split definition
  and the sources used for target-mapping diagnostics. No local machine paths,
  credentials, or user settings are included.

The supplementary primary sources were retrieved directly over unauthenticated
HTTPS; `web.run` was not exposed to this worker (`tool_describe` returned
`not_found`). This is pinned-source evidence, not a claim about newer releases.

## Replay / read-only provenance

Ordinary Rust tests need neither Electron nor the original research directory:

```sh
cargo test --locked -p echosight-platform --test accelerator_contract
cargo test --locked -p echosight-platform --test accelerator_corpus
```

With the original V-ACC evidence directory available, verify without rewriting
its sources, observations or manifests:

```sh
python crates/echosight-platform/tests/accelerator_fixture/generate.py "$V_ACC_EVIDENCE_ROOT" --check
```

Omit `--check` only when intentionally regenerating the two local fixture files.
The extractor never writes the input evidence. Supplemental sources can be
retrieved from their pinned public URLs and checked against the recorded hashes.
An independent replay during adapter development reproduced the entire original
Electron observation JSON byte-for-byte (SHA-256 in `provenance.json`).

## Target mapping limitations

| Target/key | Inactive diagnostic | Source reason |
|---|---|---|
| All: AltGr | `UnresolvedPlatformToken` | Electron syntax accepts it; no faithful Windows representation established. Linux/Mac policy remains unproven too. Never drop it or replace with Ctrl+Alt. |
| Linux: MediaPlayPause | `BackendUnsupported` | Pinned X11 backend has no matching Code arm; MediaPlay/MediaPause are not replacements. |
| Mac: F21–F24, ScrollLock, MediaStop | `BackendUnsupported` | Pinned Rust backend lacks corresponding mappings. |
| Mac: PrintScreen, NumLock | `DifferentNativeKey` | Chromium conversion gives -1; Rust selects 0x46 / 0x47. Do not silently change native key. |
| Mac: volume keys, MediaPlayPause, MediaNextTrack, MediaPreviousTrack | `UnprovenPlatformPolicy` | Electron's patched media listener and Rust's Carbon/media split are not proven interchangeable. |

Windows supported keys are joined to their exact backend VK values. Residual
Cmd/Command/Meta/Super uses META: Electron's **applied**
`command-ismediakey.patch` maps COMMAND to MOD_WIN, unlike unpatched Chromium.
Ordinary raw Cmd becomes the primary modifier in the unchanged main normalizer;
its exact-case `CommandOrControl` fast path can preserve residual Cmd.

No native acceptance is claimed, including for `Ok` values: OS conflicts,
reserved chords, repeats, layout/OEM behavior, numpad/NumLock, media handling,
Linux display protocols, Mac policy, dispatch, recorder consumption and B5 remain
separate gates. In particular the pinned Windows backend adds **MOD_NOREPEAT**;
Electron's traced path does not. This adapter deliberately makes no backend
change and cannot resolve that discrepancy.
