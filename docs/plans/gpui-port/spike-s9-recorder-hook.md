# S9: app-owned Windows recorder interception experiment

**Verdict: NO-GO for recorder integration.** This is an isolated Windows-only GPUI
component probe, not the settings recorder, a native-accepted adapter, or a
replacement for the mandatory recorder. Linux policy tests and Windows target
compilation do not establish native consumption, DOM decoding or state repair.
Independent safety review is required before delivering any Windows artifact.

## Frozen scope and evidence

- Original spike base: `02671b05fcd9fb11b64d79979c872690f7f92fe7`.
- R1/R2 correction base: `a25f2aee5f7c5c84abd2eae825b9a424512d3e9c`.
  This correction changes only the S9 example/support files and this document;
  no manifests, lockfile, GPUI sources/pin, production core/UI or integration.
- GPUI and gpui_platform remain on
  `4ed72e0135aaab74a43914491c915dd6cbd10c67`. No fork, cache edits, new library or
  version change. The only manifest additions are existing windows 0.62 features
  `Win32_UI_Input_KeyboardAndMouse` and `Win32_UI_Shell` (snapshot/preheld mapping
  and the destruction observer respectively). Cargo.lock is unchanged.
- Normative inputs: continuation `contract-amendment.md` §3, original
  `contract-final.md` §3 retained recorder/decode rules, and
  `b5-hook-feasibility.md` with its `b5-hook-evidence/` primary-source bundle.
  The historical no-implementation restriction was superseded by this isolated
  spike authorization, not by editing either contract.
- Pinned source ordering: `gpui_windows/src/platform.rs:583–587` retrieves with
  GetMessage before `translate_accelerator`; `1183–1190` accepts only down/system
  down; the foreground-task timeout path at `1308–1335` uses PeekMessage with
  PM_REMOVE then the same translation/dispatch sequence. WH_GETMESSAGE is
  before both **by API/source composition**, not by an observed Windows trace.
- Microsoft primary API contracts in the evidence bundle: GetMsgProc,
  SetWindowsHookExW, WM_NULL, GetKeyboardState, GetKeyboardLayout,
  UnhookWindowsHookEx, SetWindowSubclass/RemoveWindowSubclass/DefSubclassProc.
  The final selected dependency checkout was read-only verified at the pin; its
  pre-existing untracked `.cargo-ok` remains the only checkout status entry.

The original spike included the two feature additions above; this correction
does not change them. Its allowlist is the example,
`examples/recorder_hook_support/`, and this document. No production library,
application binary, UI/core module, Electron/TypeScript, golden, workflow or
configuration changes are part of S9. Nothing is pushed/published/merged.

## What runs

On native Windows, from this reviewed source revision:

```powershell
cargo run --locked -p echosight-platform --example s9_recorder_hook
# Optional explicit activation request (still requires exact-window focus):
cargo run --locked -p echosight-platform --example s9_recorder_hook -- --start-recording
```

Release build, only after review:

```powershell
cargo build --locked --release -p echosight-platform --example s9_recorder_hook
```

No data-directory argument exists. No settings, files, clipboard, sound, global
hotkey registration or user text are saved. The probe opens its own ordinary GPUI
window, not the production overlay. Installation starts **inactive**; only Start,
Switch, Begin with live Ctrl+K timeout, Prequeue chars + Begin, or the explicit
CLI flag requests activation.
Begin fails visibly if the exact window does not have focus or preheld snapshot
validation fails. Installation failure has no ordinary-GPUI-event fallback.

The UI shows static counters, typed diagnostic counts and only the latest
explicit-own-window numeric raw snapshot (VK, scan, flags, timestamp, HKL,
left/right modifier bytes). It never decodes text, retains a key history, or
writes raw snapshots to a log/file. Stop's matching-release/character guard does
not take keyboard snapshots. GPUI event/action counters are ordinary local
canaries, not an inactive raw recorder.

Non-Windows execution prints an unsupported message and exits **2**, without
starting a fake Windows window or claiming a native result.

### Source layout

| File | Responsibility |
|---|---|
| `s9_recorder_hook.rs` | cfg boundary and honest unsupported entry point |
| `recorder_hook_support/policy.rs` | fixed-capacity capture/release/tail/lease policy, no OS or GPUI |
| `recorder_hook_support/tests.rs` | executable pure policy adversarial tests |
| `recorder_hook_support/decode.rs` | closed typed diagnostic interface; deliberately no DOM decoder |
| `recorder_hook_support/native.rs` | own-thread hook, narrow observer, native diagnostics/probes |
| `recorder_hook_support/probe.rs` | GPUI window, foreground drain, existing core recorder lifecycle, controls |
| `recorder_hook_support/canaries.rs` | shared distinct key bindings, live pending-timeout guard, keymap/removal regression tests |
| `recorder_hook_support/gpui_timeout_tests.rs` | opt-in headless GPUI dispatch/timer tests, no native hook |
| `recorder_hook_support/run_gpui_timeout_tests.py` | builds the opt-in harness and asserts exact named tests execute |

## Implemented safety policy (experimental, not accepted source parity)

1. **Ownership:** verify current PID, current nonzero TID and HWND owner PID/TID
   before installation, and revalidate immediately before SetWindowsHookExW.
   Register WH_GETMESSAGE with no module and that nonzero current thread. No
   low-level/global hooks, foreign process/window selection or SendInput.
   Native functions remain private to the example's module tree.
2. **Filter:** negative/non-action codes and PM_NOREMOVE chain directly without
   pointer dereference, decoding, depth updates or records. Other HWNDs, thread
   WM_QUIT, WM_HOTKEY and unrelated message classes are never rewritten. An
   inactive installation produces no raw records. Lifetime bookkeeping is not
   input capture.
3. **Consume before decode:** for selected exact-window downs, copy raw MSG fields,
   write WM_NULL/zero wParam/lParam before snapshotting, preserve queue-associated
   GetKeyboardState and current-thread HKL, then enqueue copied data. Snapshot
   failure/overflow does not release the original key into GPUI. The raw shape
   preserves scan, extended, repeat count, previous-down, transition and Alt
   context; location is a **native hint**, not claimed DOM location parity.
   Scan-zero identity uses VK; E1/multi-device/layout mapping equivalence is open.
4. **Release candidate:** a fixed physical-key set includes preheld keyboard keys
   at Begin plus consumed downs. Matching releases are neutralized and retire the
   obligation. After Stop, matching autorepeats and releases remain guarded but
   produce no snapshots. This includes preheld Alt with **no captured down** after
   an old 200 ms Stop. A fresh non-repeat down while stopped ends that key's old
   obligation, so its later unrelated up is not swallowed. Focus loss and
   destruction clear all obligations. Unknown/unmatched ups are not blanket
   swallowed. This guard deliberately does **not** claim GPUI state repair.
5. **Characters:** exact-HWND WM_CHAR, WM_SYSCHAR, WM_DEADCHAR, WM_SYSDEADCHAR are
   neutralized while armed, including prequeued characters. After a real active→
   stopped transition, quarantine lasts **at most 50 ms and 32 removed character
   messages**, whichever ends first. A fresh stopped keydown, focus loss or
   destruction terminates it earlier. Repeated Stop does not extend it. Other
   messages/windows are not drained. Delayed characters beyond the bound can
   pass; the bound is a tested probe policy, not a proof of complete source parity.
6. **FIFO and ordering:** 64 copied records, monotonic sequence and arm ID; overflow
   drops newest records but retains a fixed typed loss report (first sequence,
   arm and count). Selected keys remain consumed, recorder remains active. The
   outer record is queued before chaining, so recursive retrieval appends after
   it. A non-pumping 16 ms foreground poll drains independently of key traffic;
   every Begin/switch/Save/Cancel/Stop/close/200 ms/global-boundary handler first
   drains and applies the old arm, then changes native/core state, without pumping
   in between. Normal close invalidates the HWND before window removal. An
   unexpected destruction invalidates synchronously, retains copied records until
   the final foreground cleanup, and reports abandoned diagnostics rather than
   applying them to a new recorder. That abnormal path is not business parity.
7. **Core boundary:** the existing `HotkeyRecorder` handles Begin/switch/Stop and
   `finish_deferred_stop`; no recorder normalization/duplicate/business reducer is
   cloned. Start/Switch controls alternate two probe action rows, not a production
   settings UI. Effects are observed, **not** executed as OS registration/storage.
   The 200 ms button injects an independent detached callback; it captures no arm
   token and stops whichever recorder is current. No successful DOM key means no
   source success-generated timer is claimed. Save/global buttons are ordering
   controls only, not integration with persistence/global registrations.
8. **FFI/chain:** stable extern-system trampolines; no MSG pointer escapes, heap
   allocation, GPUI/App callback, dialog, blocking lock, storage, hotkey
   registration or pump in the recorder callback. Short TLS RefCell borrows end
   before CallNextHookEx/DefSubclassProc. Neutralize→chain→reassert preserves the
   downstream return value. Internal panic/borrow-invariant failure aborts rather
   than unwinding through FFI or failing open. This is not protection against a
   hostile hook earlier/later in the chain.
9. **Lifetime:** a narrowly scoped SetWindowSubclass observer clears target and
   recording state on nonqueued WM_NCDESTROY before forwarding, and clears the
   guard on WM_KILLFOCUS. It counts armed system-command/IME/sent-key bypass
   observations but is **not** a late keydown interceptor. On destruction it
   removes its own subclass; no input sink/App pointer is registered. TLS storage
   has no destructor and lives through thread exit; executable code is never
   unloaded. HHOOK ownership is retained on failed unhook while target is zero;
   cleanup is retryable. Success at nonzero callback depth defers handle
   reclamation, and a second cleanup does not re-unhook a successfully released
   handle. Normal close refuses while cleanup is outstanding. Unexpected process
   exit with debt reports it, never calls it successful cleanup.

## Precise blockers / fallback comparison

**Reconciliation remains blocked.** The release policy prevents selected ups from
reaching defaults, but swallowing a preheld release can strand these separate
pinned states:

- `gpui_windows/src/window.rs:61` and `events.rs:1534–1563`:
  `last_reported_modifiers` / `last_reported_capslock` and deduplication;
- `gpui/src/window.rs:6305–6334`: `pending_modifier` can synthesize lone-modifier
  actions when a later update finally arrives;
- `gpui/src/window.rs:6381–6465,6606–6715`: pending keymap input/replay/timeouts;
- `gpui_windows/src/window.rs:58–62`, `events.rs:1406–1435`: pending UTF-16
  high surrogate, separate from IME; later input can complete a pair across a
  capture boundary;
- `gpui_windows/src/events.rs:694–767`: native IME/composition and the text input
  handler; disabling IME may COMPLETE composition, not silently cancel it.

`Window::modifiers()` / pending-key counters are observations, not a complete
repair API. The low-level pending-key clear is crate-private, but public `blur`
clears pending input, and public `focus` clears it **when focus changes**; focusing
the already-focused handle does nothing (`gpui/src/window.rs:2620–2645`). Public
`set_pending_input_timeout_paused` pauses/resumes rather than discards input;
releasing the owner resumes it. Timeout replay can invoke actions/text without
rerunning keystroke interceptors (`6476–6498,6640–6752`).

Public modifier-event dispatch can enter GPUI lone-modifier synthesis. An exact-
window interceptor can suppress a synthesized keystroke, but neither API repairs
the backend modifier/caps-lock dedup caches or pending surrogate. Test-only
`set_modifiers` sets only the public field. Live platform-window/backend storage
is private; raw-HWND access to that storage is not a public repair API. Focus/
input-handler changes do not prove silent native composition cancellation.
Thus public pending cancellation exists, but no complete coordinated backend/
GPUI state-reconciliation seam has been demonstrated.
This spike does not call synthetic modifier events, ToUnicodeEx, dead-key
flushing, or blanket-keyup swallowing and pretend they solve this problem.
There is no text input handler in S9, so zero text cannot establish editor/IME
safety. The IME counter only exposes a known bypass surface.

**Decoder remains blocked.** `Result<KeyEvent, RecorderDecodeError>` uses the
closed SnapshotFailed / UnrepresentableKey / UnsupportedTranslation variants.
All otherwise-valid raw keydowns currently return UnsupportedTranslation,
including plain letters. No KeyEvent is fabricated or fed to the core on error;
capture continues. ToUnicodeEx flag 4 is not implemented or claimed as a DOM
implementation. Actual Electron/Chromium DOM comparison remains mandatory.

**Native and integrated sequencing remain unproven.** Native pump wake ordering,
benign hook reentrancy, actual unhook failure, window destruction/reuse, editor
state, global-command integration and real successful-recording timer effects
have no Windows run here. Pure reentrancy tests exercise the same chain helper and
policy, not Windows itself. PostMessage probes are not hardware-state evidence.

If native/state reconciliation cannot be made complete through narrow public
app-owned seams, compare a **reviewed pinned pretranslation callback shared by
both GPUI pumps**, plus explicit backend cache/GPUI pending-key/lone-modifier/
UTF-16 surrogate/composition repair and lifetime ownership. Such a patch would
still require the release/character policy, decoder and native acceptance. A
keydown-only patch is not sufficient.
Keeping WH_GETMESSAGE plus a separately reviewed backend-owned reconciliation
API is also a comparison option. Private fields do not prove every app-owned
mechanism impossible or mandate moving interception into GPUI. No fork,
dependency change, broad reconciliation implementation or relaxation of the
mandatory recorder is authorized or made by this correction.

## Original spike gates (Linux host, 2026-11-11; historical a25f2aee)

A fresh, **unseeded** scratch CARGO_TARGET_DIR was created for this task. Builds
used the existing host GPUI/llvm-rc shim and CARGO_BUILD_JOBS=2; no config or shared
cache source was modified. All commands below use the workspace toolchain and
locked dependency graph.

| Gate | Actual result |
|---|---|
| First compiling policy RED | 22 executed: 17 failed, 5 negative/control cases passed; failures were missing consumption/FIFO/ownership behavior, not compilation errors |
| Policy GREEN | 22 passed |
| Typed decoder RED | 3 executed, 3 failed against an intentionally permissive compiling adapter |
| Chain/reassert RED | 3 selected: new chain case failed, 2 existing/control cases passed |
| Raw-shape RED | 3 selected: 2 new shape/location cases failed, 1 existing snapshot control passed |
| Deferred-reclaim RED | 1 executed, failed because successful deferred unhook could be attempted again |
| `cargo test --locked -p echosight-platform --all-targets` | 23 library tests, 1 existing S8 test, **30 S9 tests** passed; six other example targets have zero tests |
| `cargo fmt --all --check` | passed |
| `cargo clippy --locked -p echosight-platform --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | passed |
| `cargo test --locked -p echosight-platform --examples` | 8 targets, **31 tests passed** (30 S9 + 1 S8); none ignored |
| `cargo clippy --locked -p echosight-platform --all-targets -- -D warnings` | passed on Linux |
| `cargo check --locked -p echosight-platform --all-targets --target x86_64-pc-windows-msvc` | passed, no warnings; compilation only, no native link/run |
| `cargo run --locked -p echosight-platform --example s9_recorder_hook` on Linux | linked and executed unsupported stub; printed Windows requirement, actual exit **2** |
| Native Windows linking/execution | NOT RUN; no Windows runtime available |

The S9 30 tests comprise 27 policy/chain/lease tests and 3 typed-decoder tests.
They cover PM_NOREMOVE/non-action/negative codes, wrong HWND/quit/hotkey classes,
explicit activation, failed snapshot consumption, raw copies, physical identity,
recursive FIFO order, overflow, every foreground boundary, arm switching,
uncancelled old stops, preheld Alt, repeats/releases/new cycles, focus/destruction,
prequeued-character/tail bounds, chain return/reassertion and unhook debt/retry.
Test names are in `recorder_hook_support/tests.rs` and `decode.rs`; no ignored
native tests are mislabeled as passes.

## Native acceptance script — every row PENDING

Record reviewed build SHA, Windows version, layout/HKL, keyboard/IME, run mode,
per-step counters and failures. Use test keys only; do not record personal text.
Do not distribute an artifact before independent safety review. These controls
are diagnostic probes, not self-certifying green checks.

1. **Inactive/isolation/install:** launch without arguments. Confirm armed=false
   and no raw counter growth while typing in this window or another app. F10/Alt
   canaries establish baseline behavior. Start with the mouse; verify only this
   HWND records. Test owner PID/TID rejection and installation failure in a native
   debugger/test harness; no fallback capture may activate. Same-thread second
   HWND and hidden global-hotkey HWND isolation require a reviewed native harness
   (S9 does not install a global hotkey or a foreign-window hook).
2. **Both pinned pumps:** break at `native::get_message` and the two pinned
   retrieval return sites (`platform.rs:583–587` and `1319–1335`). Type physical
   keys during ordinary idle and during **Nested-pump load**. The latter queues
   bounded foreground work exceeding GPUI's 10 ms task budget; only a captured
   stack attests which pump actually ran. For both, inspect returned WM_NULL and
   no keymap/key callback/TranslateMessage keyboard path for selected downs.
   The hook cannot infer the caller route. There is no manual retrieval button
   in this revision; only the ordinary GPUI pumps retrieve queued messages.
3. **Default paths:** while armed test bare Escape, Tab, Enter, Space, Alt, F10,
   Alt+Space, Alt+F4, and release/repeat variants. No canary action, system menu,
   focus navigation or close should arise from selected queued input. Repeat with
   the backend modifier cache already equal to the release state. OS-reserved
   Win keys, Alt+Tab/secure attention/media effects remain outside the claim.
4. **Preheld and old timer:** hold left Alt before mouse Start, without pressing
   any new key; queue **200 ms Stop**, wait for it to fire, then release Alt.
   Matching up must be consumed even with no captured down. Repeat with right
   Alt, Shift, Ctrl and F10, release during recording/after Stop, and while
   switching rows before an old timer fires. Old callback stops the current row;
   no generation cancellation. After release, a fresh unrelated down/up must
   pass normally. Record GPUI modifier/pending-key state; stale state is a known
   blocker, not a successful recorder.
5. **Quarantine:** click **Prequeue chars + Begin**, verify four character classes
   are consumed. Stop and test characters before/after the 50 ms boundary and
   around the 32-message cap with a native harness; fresh stopped keydown/focus
   loss must end the tail. Prequeue characters from an earlier TranslateMessage
   too, not only PostMessage. Do not drain another HWND or unrelated messages.
6. **PM_NOREMOVE/FIFO — native gate unavailable in S9:** the unsafe manual
   retrieval diagnostic and UI control were removed (R1). The former claim that
   it removed only exact-own-HWND key messages is retracted: PeekMessage can also
   retrieve child-HWND messages and WM_QUIT bypasses numeric message filters.
   The helper discarded successful removals, including those unexpected classes.
   No replacement retrieval/replay/dispatch loop was added. Pure policy tests
   still cover no-remove callbacks, FIFO/overflow and unrelated/quit policy; they
   do not establish native queue preservation. Native no-remove/overflow, child-
   window isolation and quit exit-code delivery remain pending a separately
   reviewed harness; do not recreate the removed control for this script.
7. **Decode failure:** toggle snapshot failure, type own-window keys, verify
   SnapshotFailed growth while consumption and recording continue. Disable it;
   valid raw downs still report UnsupportedTranslation, never successful chords.
   Check left/right bytes, scan/extended/repeat/transition and message time/HKL
   against a native trace. Snapshot self-transition timing is still pending.
8. **Ordering controls:** rapid Start/switch/Save/Cancel/global-boundary/close with
   a backlog and multiple queued 200 ms Stops. `stale` must stay zero. Inspect
   sequences/arm IDs in the debugger and validate old batch before new core row.
   Source successful-recording callbacks and actual global dispatch/registration
   remain pending a real decoder/integration harness; these buttons are not that.
9. **Real pending-timeout canary (R2), then focus/composition diagnostics:**
   - While inactive, press Ctrl+K and observe `pending_timeout=true`, then wait:
     SHORT should increment and LONG should not. Repeat Ctrl+K then Ctrl+C before
     expiry: LONG should increment without a later SHORT. F10/Alt retain their
     separate ordinary canary counter. A pending-key count alone is insufficient.
   - Stop, press Ctrl+K, then mouse-click **Begin with live Ctrl+K timeout** before
     expiry. The handler checks actual `pending_input().timeout()` is Some, for
     exactly Ctrl+K, running and with time remaining, synchronously before Begin.
     A refusal (already armed/no live timer) is **not** a timeout run: retry. Only
     successful native/core Begin increments `verified_timeout_begins`. No focus
     change, pause/resume or pending cancellation is performed by this control.
   - Observe the SHORT counter and pending state during recording, then after
     Stop/release/focus loss; existing pending actions can replay despite queued
     key interception. Record actual deltas, not expected success. The gate and
     counters are diagnostics, not a fix for replay or proof of reconciliation.
   - Separately start with a held modifier and compare backend dedup caches,
     GPUI pending_modifier/pending_input and public modifiers before/during/after.
     Switch focus away/back; inspect stale key cycles. Inspect pending UTF-16
     surrogate state separately from IME start/composition/end bypass counters.
     A real text-handler test is **not present**; editor/preedit/surrogate
     reconciliation is blocked, not silently accepted.
10. **DOM/layout corpus:** compare the locked Electron DOM key/modifier outputs for
    Ctrl letters, shifted digits/OEM, pre-focus modifiers, physical Ctrl+Alt vs
    AltGr/dead keys, pending/successive dead keys, layout switching, numpad/NumLock,
    locks/media, VK_PACKET/IME and non-BMP. S9 currently yields no successful DOM
    keys, so this row cannot pass as decoder acceptance.
11. **Sent negative control:** Start then **Sent negative control**. It sends only
    a static key to this HWND outside App/entity borrows; raw count must not claim
    a capture. A bypass-key/default observation is expected. Direct sent/dispatched
    input does not pass through WH_GETMESSAGE. The control is not coverage.
12. **Chaining/reentrancy:** with a separately reviewed benign same-thread hook
    installed in a native harness, verify observed WM_NULL, preservation of its
    nonzero return, reassertion after it changes the selected MSG, and recursive
    own-window retrieval order/depth. Do not add a pump to S9's recorder callback.
    Earlier hooks may already have altered input. Pure chain tests are not this
    native result; this harness is not bundled or run.
13. **Unhook debt:** click **Retain unhook debt**. This intentionally SKIPS the OS
    unhook, leaving a real inactive HHOOK; it does not fabricate an OS error.
    Target is cleared immediately, raw capture stops, hook debt=true. **Install
    inactive** must refuse until **Retry cleanup** actually unhooks. Test a real
    injected API failure and cleanup during recursive callback depth in the native
    harness; success must defer reclaim and must not be unhooked twice.
14. **Destruction/reuse:** close normally while armed/with held keys/backlog. Target
    must clear before native destruction. Then force nonqueued destruction in the
    native harness, including an installed debt case, inspect WM_NCDESTROY before
    forwarding and recreate windows/force numeric HWND reuse. Old callbacks must
    remain inactive; copied diagnostics must never enter a new recorder. Normal
    close with failed cleanup is refused; unexpected exit reports debt honestly.
15. **Privacy/unsupported controls:** verify no files/userdata/clipboard/audio or
    registrations are created. Only the latest own-window numeric snapshot is
    visible, no text history/log. Other-window typing remains untouched. Run the
    Linux executable separately: exit 2 is the expected unsupported result, not
    native acceptance.

## R1/R2 correction verification

R1 is resolved by removal, not message preservation/replay. An architectural
source guard rejects direct GetMessage/PeekMessage calls in the native adapter
and the removed UI route. It failed against the old helper. This guard is not a
Windows child-HWND/WM_QUIT runtime test. All existing policy/chain/decoder tests
remain, including ordinary interception isolation and no-remove controls.

R2 adds distinct shorter Ctrl+K and longer Ctrl+K Ctrl+C bindings, shared between
the native probe and headless GPUI tests. Keymap tests failed against the old
bindings. The optional real-GPUI tests establish the negative control (long-only
prefix is pending without a timer), assert an actual timeout exists for the
ambiguous prefix, advance GPUI's test clock and observe SHORT replay, then prove
LONG completion cancels SHORT replay. The native timeout-Begin handler uses the
same tested live-timeout guard. A compiling false guard failed both positive
GPUI tests before implementation; restoring the original binding set also made
both positive probes fail, including the actual `timeout().is_some()` assertion.
These tests do not install WH_GETMESSAGE,
exercise native Begin or establish native timer-versus-hook ordering.

Repeatable Linux commands (use a fresh dedicated scratch target and the host
build environment; no manifest/lockfile or dependency-source edits):

```bash
export CARGO_BUILD_JOBS=2
# Set CARGO_TARGET_DIR to a fresh dedicated scratch directory first.
cargo test --locked -p echosight-platform --example s9_recorder_hook
python3 crates/echosight-platform/examples/recorder_hook_support/run_gpui_timeout_tests.py
```

The Python runner enables the existing `gpui/test-support` feature only on its
Cargo invocation, and passes `--test --cfg s9_gpui_timeout_tests` only to the S9
example via `cargo rustc`. It reads the exact executable from Cargo JSON, asserts
the three expected test names exist, executes them and requires all three to
pass. This builds a **test harness in the example executable slot**, not a native
artifact for delivery. Normal Cargo example tests exclude this opt-in module
and do not need test-support. The test module uses GPUI's headless test platform
(no text handler); it is not Windows runtime evidence.

Correction gates used a new unseeded scratch target, the existing host build shim
and two build jobs. No dependency-source/cache edits or artifact copying seeded
the build. Final logs were checked for warnings/errors and expected test names
and nonzero counts, rather than relying on Cargo exit status alone.

| Correction gate | Actual result |
|---|---|
| Removal/binding RED | 3 named tests executed and failed against the old helper/bindings |
| Live-timeout guard RED | 3 real-GPUI tests executed: 2 positive cases failed, negative control passed |
| Original-binding mutation control | 2 positive real-GPUI tests failed, negative control passed; corrected bindings restored |
| `cargo test --locked -p echosight-platform --example s9_recorder_hook` | **33 passed**, none ignored/filtered: original 30 plus 2 keymap and 1 source-removal guard |
| `run_gpui_timeout_tests.py` | **3 named headless GPUI tests passed**, 33 ordinary tests filtered; actual pending timer/replay, not Win32 |
| `cargo test --locked -p echosight-platform --all-targets` | **57 passed**: 23 library + 1 S8 + 33 S9; six other example targets contain zero tests |
| `cargo fmt --all --check`; explicit `rustfmt --edition 2024 --check` on the opt-in included test file | passed |
| `cargo check --locked -p echosight-platform --all-targets` | passed on Linux |
| Same check with `--target x86_64-pc-windows-msvc` | passed; type-check/cross-compilation only |
| `cargo clippy --locked -p echosight-platform --all-targets -- -D warnings` | passed on Linux |
| Same Clippy with `--target x86_64-pc-windows-msvc` before `--` | passed |
| Opt-in harness lint through `RUSTC_WORKSPACE_WRAPPER` set to `clippy-driver`, using the runner's `cargo rustc` arguments plus `-D warnings` | passed; test-harness flags scoped only to the example |
| `cargo run --locked -p echosight-platform --example s9_recorder_hook` on Linux | unsupported stub linked/executed; expected exit **2** |
| Native Windows link/run, native timeout-Begin control, both-pump stacks | **NOT RUN / PENDING** |

The headless tests are additional to, not included in, the default 33 S9 tests.
Scratch evidence paths and the checkpoint SHA are returned in the handoff; no
logs, binaries or host-specific build configuration are committed.

No Windows runtime is available. **Every native row above remains pending;
integration remains NO-GO** due to unimplemented DOM decoding and unresolved
backend caches, GPUI pending/lone modifiers, UTF-16 surrogate and IME/composition
reconciliation. This isolated checkpoint requires safety re-review before any
exact-SHA native link/run or artifact delivery; no production wiring is authorized.
