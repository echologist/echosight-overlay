# Echosight — Main-Process Platform Inventory (for 1:1 Rust port)

Repository: `/home/agent/code/echosight-overlay`, branch `feat/exiles-corruption-sounds`, HEAD `5a63e5c`, package version 2.1.0 (`package.json:3`). Read-only analysis; nothing in the repo was modified.

Evidence labels: **Verified** = read in source/tests/node_modules; **Inferred** = follows from Electron/electron-builder semantics but not exercised at runtime.

---

## 1. Data paths

### 1.1 Mutable user data (Verified: `src/main/utils/dataPaths.ts:12-22`, test `tests/unit/main/dataPaths.test.ts:21-31`)

```
<userData>/
└── data/                       DATA_DIR
    ├── current_tasks.json      TASKS_FILE
    ├── templates.json          TEMPLATES_FILE
    ├── settings.json           SETTINGS_FILE
    └── themes/                 THEMES_DIR  (user + synced bundled themes; single-file *.json or <folder>/theme.json)
```

Also possible siblings created at runtime (Verified `src/main/ipc/storageIpc.ts:143-145`):
`current_tasks.json.corrupt-<ISO-timestamp-with-:.→->`, `templates.json.corrupt-…`, `settings.json.corrupt-…`, and transient `.<basename>.<pid>.<uuid>.tmp` files (`jsonStorage.ts:36-39`).

### 1.2 What `<userData>` resolves to (Inferred — Electron semantics)

Electron `app.getPath('userData')` = `app.getPath('appData')/<app.name>`. `app.name` is package.json `productName` if present, else `name`. Top-level package.json has `name: "echosight"` and **no top-level `productName`** (`package.json:2`; `build.productName` at `:24` is electron-builder-only). electron-builder's package.json transformer only merges `config.extraMetadata` (none configured) into the packaged package.json — it does not inject `build.productName` (Verified: `node_modules/app-builder-lib/out/platformPackager.js:357-363`, `fileTransformer.js:88-91`). Therefore app.name = `echosight` in both dev and packaged builds:

| Platform | userData | Concrete data dir |
|---|---|---|
| Windows | `%APPDATA%\echosight` | `C:\Users\<u>\AppData\Roaming\echosight\data\` |
| macOS | `~/Library/Application Support/echosight` | `~/Library/Application Support/echosight/data/` |
| Linux | `$XDG_CONFIG_HOME/echosight` (default `~/.config/echosight`) | `~/.config/echosight/data/` |

Note: the Windows build is a `portable` target (`package.json:45`) but the app never reads `PORTABLE_EXECUTABLE_DIR`; data still lives in `%APPDATA%` (Verified by absence: no reference in `src/`).

### 1.3 Bundled read-only defaults (Verified `src/main/index.ts:51-54`)

```ts
DEFAULT_DATA_DIR = app.isPackaged ? path.join(process.resourcesPath, 'data')
                                  : getDevelopmentBundledDataDir(__dirname)   // path.resolve(appDir,'..','..','data')
DEFAULT_THEMES_DIR = DEFAULT_DATA_DIR + '/themes'
```
- Dev: compiled main lives at `dist-electron/main/index.js` (`electron.vite.config.ts:7`) → `<repo>/data` (test `dataPaths.test.ts:9-19`).
- Packaged: electron-builder `extraResources: data → data` (`package.json:33-41`) → `<install>/resources/data` (win/linux) or `Echosight PoE Task Overlay.app/Contents/Resources/data` (mac) (Inferred from electron-builder layout).
- Bundled contents (Verified `ls data`): `current_tasks.json`, `settings.json`, `templates.json`, `themes/{cs16,dark,echosight,glass,light,minimal}.json`, `themes/{exiles_corruption,winamp-classic,windows-7,windows-98,windows-xp}/`.
- **`data/settings.json` is shipped but never read** — bootstrap's settings entry has no `source` (`runtimeDataBootstrap.ts:40-43`); defaults come from code. It is legacy (has `backgroundColor`, 3 hotkeys, no `settingsVersion`; `data/settings.json:1-10`).

### 1.4 Path safety (Verified `src/main/utils/pathSafety.ts:3-14`)
`isPathInside(base,target)`: `path.relative(resolve(base), resolve(target))` must be `''` or not start with `..` and not be absolute. `resolveChildPath(base,...segments)` throws `Refusing to access path outside <base>` otherwise. Used by theme loading (`themeLibrary.ts:238,257-258,306,327,342,398`) — not by storage IPC (storage paths are fixed, not user-supplied).

---

## 2. JSON storage (`src/main/utils/jsonStorage.ts`)

| Aspect | Behaviour | Evidence |
|---|---|---|
| Parse | `destr(data, { strict: true })` | `jsonStorage.ts:8-10` (destr 2.0.5) |
| Read | `fs.readFile(path,'utf8')` then parse | `:12-15` |
| Serialize | `JSON.stringify(value, null, 2)` — 2-space indent, LF, **no trailing newline**, UTF-8 | `:46` |
| Atomic write | mkdir -p dir → open temp `.<basename>.<pid>.<uuidv4>.tmp` with `'wx'` (exclusive create) → writeFile → `fsync` → close → `rename(temp, target)` | `:34-50` |
| Failure | close handle, `rm -f` temp, rethrow original error (cleanup errors swallowed) | `:51-65`; tests `jsonStorage.test.ts:54-61, 90-96, 98-111` |
| Locking | In-process per-resolved-path promise queue (`pendingWrites` Map); writes to the same `path.resolve(filePath)` serialize in caller order; a rejected predecessor does not block successors (`.catch(() => undefined)`) | `:6, 17-32`; tests `:20-52, 63-88` |
| Backups | none in jsonStorage itself (only the quarantine copy in storageIpc, §4) | — |
| Cross-process locking | none | — |

destr strict-mode quirks a Rust port must mirror if byte/semantic compatibility of *reads* matters (Verified `node_modules/destr/dist/index.mjs`):
- Input first trimmed; `"true"/"false"/"undefined"/"null"/"nan"/"infinity"/"-infinity"` (case-insensitive, ≤9 chars) return JS primitives **without throwing** (a file containing `undefined` loads as `undefined`, not corruption).
- A string that is `"…"` with no backslash returns the inner string without JSON validation.
- Anything not matching `/^\s*["[{]|^\s*-?\d{1,16}(\.\d{1,17})?([Ee][+-]?\d+)?\s*$/` throws `SyntaxError('[destr] Invalid JSON')` → empty file counts as corrupt (test `storageIpc.test.ts:57-83`).
- Keys `__proto__` / `"constructor":` present anywhere → strict mode throws `Possible prototype pollution` → file is treated as **corrupt and quarantined**.

---

## 3. Runtime data bootstrap & migrations

### 3.1 First run (Verified `src/main/storage/runtimeDataBootstrap.ts:25-67`, called from `index.ts:69-76`)
Order on every start: `app.whenReady()` → `initializeRuntimeData` → `themeLibrary.initialize()` → `loadSettingsOnStartup` → create window → register IPC (`index.ts:78-84`).

For each of the three files, **only if the target does not exist** (`fs.access`):
| File | Seed source | Fallback if seed missing/unparseable |
|---|---|---|
| `current_tasks.json` | `<DEFAULT_DATA_DIR>/current_tasks.json` | `{ tasks: [], currentTemplate: null, snapshots: [] }` |
| `templates.json` | `<DEFAULT_DATA_DIR>/templates.json` | `[]` |
| `settings.json` | *none* (code) | `createDefaultSettingsState(DEFAULT_HOTKEYS)` |

Seeds are copied via `readJsonFile` → `writeJsonFile` (re-serialized, not byte-copied). Existing files are never overwritten (test `runtimeDataBootstrap.test.ts:49-71`). No `defu` merge anywhere in storage — `defu` is only used for theme variants (`themeHelpers.ts:105`).

Default settings written on first run (Verified `src/shared/settingsDefaults.ts:7-24` + `src/main/utils/hotkeys.ts:3-9`):
```json
{
  "settingsVersion": 2,
  "transparency": 70,
  "theme": "echosight",
  "hotkeys": {
    "toggleVisibility": "CommandOrControl+Shift+T",
    "toggleInteractive": "CommandOrControl+Shift+I",
    "completeNextTask": "CommandOrControl+Shift+N",
    "undoLastAction": "CommandOrControl+Shift+Z",
    "redoLastAction": "CommandOrControl+Shift+Y"
  },
  "sounds": { "enabled": false, "volume": 60 }
}
```
Bundled seeds today (Verified): `data/current_tasks.json` = one task `{id: 1749938954072.5596, text: "Hello there! :)", completed: false, createdAt}` + `currentTemplate: "hf & gl"` (no `snapshots`, no new Task fields); `data/templates.json` = one template `{id: 1749925298270, name: "hf & gl", tasks:[{text}], createdAt}`.

### 3.2 Settings migrations (Verified — performed in the **renderer**, `src/renderer/features/settings/settingsDomain.ts:94-121`, driven by `settingsPersistence.ts:7-36`)
- `CURRENT_SETTINGS_VERSION = 2` (`settingsDefaults.ts:7`). There is no explicit "v1" constant; pre-versioned files simply lack `settingsVersion`.
- `normalizeSettings(input, fallback)` always produces `settingsVersion: 2`; `theme = input.theme (trimmed) || input.backgroundColor (trimmed, legacy) || fallback.theme`; `transparency`/`sounds.volume` → `Number()`, non-finite→fallback, else `round` and clamp 0..100; `sounds.enabled` must be boolean else fallback; hotkeys: each key trimmed string or fallback (no format conversion here).
- `shouldPersistSettingsMigration` → rewrite file when input isn't an object, `settingsVersion !== 2`, `'backgroundColor' in input`, or any normalized field differs. The rewrite drops `backgroundColor` (output shape has no such key, `:99-105`).
- Main process additionally tolerates a missing/corrupt settings file at startup: `readJsonFile` failure → log + keep defaults (`index.ts:86-100`); only `hotkeys` is consumed by main at startup.

### 3.3 Task-structure migration (cross-reference, renderer): `taskStateController.migrateTaskStructure` (`taskStateController.ts:116-123`) upgrades legacy tasks lacking `mode/triggers/activated/...`. Owned by the other agent; mention because the bundled seed relies on it.

### 3.4 Version-based theme refresh (Verified `src/main/themes/themeLibrary.ts:64-195`, cross-reference)
On every start `copyDefaultThemes` walks `DEFAULT_THEMES_DIR`: a bundled theme is copied if absent; if present it is overwritten only when `id` and `author` match and bundled `version` (dotted ints) is strictly greater (`:179-195`). Folder themes replace a legacy single-file `<name>.json` only when the bundled folder version is newer; the legacy file is then removed (`:102-177`). Tests `themeLibrary.test.ts:20-118`.

---

## 4. Storage IPC contract (`src/main/ipc/storageIpc.ts`; preload `src/preload/index.ts:5-10`)

All six are `ipcMain.handle` (invoke/response). No argument validation is performed on save payloads — whatever object the renderer passes is serialized verbatim (`:65, :84, :102`).

| Channel | Request | Response | Notes |
|---|---|---|---|
| `load-tasks` | — | `TaskLoadData` = `{tasks, currentTemplate, snapshots?, corruptBackupPath?}` | Missing file → `{tasks:[],currentTemplate:null,snapshots:[]}` with **no file created** (test `:22-31`). Corrupt → quarantine, write fallback, return `{...EMPTY, corruptBackupPath}` (`:46-56`). Non-ENOENT read error → **rejects** (throws to renderer) (test `:33-55`). |
| `save-tasks` | `TaskSaveData` | `SaveResult {success, error?}` | Blocked after failed load (see below). |
| `load-templates` | — | parsed JSON (any) or `[]` | corruptBackupPath is logged, not returned (`:73-75`). |
| `save-templates` | `TaskTemplate[]` | `SaveResult` | |
| `load-settings` | — | parsed JSON or `null` | fallback is `null`; on corrupt, `null` is **written to the file** (test `:85-112`). Non-ENOENT errors rethrown (`:112-122`). |
| `save-settings` | `Settings` | `SaveResult` | logs full settings object. |

Quarantine algorithm `loadJsonOrQuarantine` (`:125-149`): read bytes → ENOENT ⇒ `{value: fallback}` → parse (destr strict) → on failure: `backup = <file>.corrupt-<new Date().toISOString().replace(/[:.]/g,'-')>` (e.g. `tasks.json.corrupt-2026-07-24T12-34-56-789Z`), `fs.copyFile(file, backup)` (byte-exact), then `writeJsonFile(file, fallback)`, return `{value: fallback, corruptBackupPath}`.

Save-block state machine (`:20-44`): per-path `blockedPaths: Map<path, errorMessage>`. Any load that throws (EACCES, backup copy failure, fallback write failure) records the error; subsequent saves to that path return `{success:false, error:'Save blocked after load failure: <msg>'}` without touching disk (tests `:114-182`). A later successful load clears the block (`:32`; test `:184-197`). Block state is in-memory only.

Error shape: `SaveResult.error = error instanceof Error ? error.message : String(error)` (`src/shared/errors.ts:1-3`).

Snapshots (Verified): the cap is renderer-side — `MAX_AUTO_SAVE_SNAPSHOTS = 5` (`taskStateController.ts:44`), appended on every `saveTasks()` as `{createdAt: ISO, state:{tasks,currentTemplate}}` and sliced to the last 5 (`:224-232`); on load, `slice(-5)` (`taskPersistence.ts:47`). Main stores whatever it receives.

Persisted shapes (Verified `src/shared/types.ts:9-99`):
```ts
Task { id:number; text:string; completed:boolean; createdAt:string; children:Task[]; mode:'main'|'background';
       triggers:number[]; activated:boolean; activatedAt:string|null;
       backgroundOptions:{expiresAfterMinutes:number|null; priority:'normal'|'high'}|null }
TaskSaveData { tasks:Task[]; currentTemplate:string|null; snapshots?:{createdAt:string; state:{tasks,currentTemplate}}[] }
TaskTemplate { id:number; name:string; tasks:TemplateTask[]; createdAt:string; imported?:boolean; community?:boolean }
TemplateTask { text; children?; mode?; backgroundOptions?; triggerIndices?:number[] }
Settings { settingsVersion:number; transparency:number; theme:string; hotkeys:HotkeySettings; sounds:{enabled,volume}; backgroundColor?:string }
```
Task `id` values are `Date.now()`-style numbers, sometimes fractional (`data/current_tasks.json:4`) — a Rust port must use f64/JSON number, not integer.

Other main-side channels (for completeness; Verified `windowIpc.ts`, `themeIpc.ts`, preload):
`ipcMain.on`: `update-hotkeys(HotkeySettings)`, `focus-window`, `set-hotkey-recording(boolean)`, `reset-window-position`, `toggle-overlay`, `minimize-overlay`, `show-overlay`, `toggle-interactive-mode`, `quit-application`.
`ipcMain.handle` (themes): `load-themes`, `get-theme(id)`, `reload-themes`, `open-themes-folder`, `get-themes-path`, `load-theme-css(id,file)`, `get-theme-asset(id,name)`.
Main→renderer `webContents.send`: `interactive-mode-changed(bool)`, `complete-next-task`, `undo-last-task-action`, `redo-last-task-action` (`windowActions.ts:58-76`).

---

## 5. Hotkey normalization

### 5.1 Main process (Verified `src/main/utils/hotkeys.ts`)
- Canonical runtime format: Electron accelerator, e.g. `CommandOrControl+Shift+T` (`:3-9` defaults; `+`-joined, no spaces).
- `normalizeHotkeys(input, fallback=DEFAULT_HOTKEYS)` → object with exactly the 5 keys; each via `convertHotkeyFormat` (`:13-47`).
- `convertHotkeyFormat(v, fallback)` (`:49-74`): non-string/blank → fallback; if contains `CommandOrControl` → only dedupe; else regex-replace whole words `Ctrl|Control|Cmd|Command` (case-insensitive) → `CommandOrControl`; then `dedupePrimaryModifier`: split on `+`, trim, drop empties, keep only the first `CommandOrControl` (`:80-99`). `Alt`, `Shift`, `Super`, key names pass through untouched. Tests `hotkeys.test.ts:9-29` (`Cmd+Control+Shift+T` → `CommandOrControl+Shift+T`).
- Applied at: startup from `settings.hotkeys` (`index.ts:92-96`) and on `update-hotkeys` IPC with fallback = current hotkeys; only re-registers if changed (`windowIpc.ts:27-43`).
- Registration (`globalHotkeys.ts:20-89`): `globalShortcut.unregisterAll()` then register all 5; each failure is logged; throws (caught, logged) only if 0/5 succeed. Toggle-visibility works even when hidden; the other four only fire when `window.isVisible()`. Registered 500 ms (dev) / 2000 ms (packaged) after `did-finish-load` (`createOverlayWindow.ts:73-80`). Suspended during recording via `set-hotkey-recording` (`index.ts:216-225`).

### 5.2 Renderer recorder (Verified `settingsDomain.ts:60-92, 174-198`; `hotkeyRecorder.ts`; `hotkeyDisplay.ts`)
- Uses `KeyboardEvent.key` (not `.code`). Modifier-only keys (`Control/Alt/Shift/Meta`) ignored; `' '`→`Space`, `ArrowUp/Down/Left/Right`→`Up/Down/Left/Right`; single-char keys upper-cased; others (e.g. `F5`, `Enter`, `Escape`) kept verbatim.
- Assembled order: `[Ctrl|Cmd]`, `Alt`, `Shift`, main key, joined with `+`. Primary label is `Cmd` when `navigator.platform` matches `/mac|iphone|ipad|ipod/i`, else `Ctrl` (`hotkeyDisplay.ts:1-3,15-17`). `ctrlKey || metaKey` both map to the primary label.
- Accepted only if it includes the primary modifier, or `Alt`, or the main key is `F1`–`F12`; `Shift+X` alone → warning “Please use Ctrl+X, Alt+X, or a function key…”; bare key → ignored.
- Duplicate check: `formatHotkeyForDisplay(h).trim().toLowerCase()` equality against the other 4 actions; blocks with alert (`hotkeyRecorder.ts:119-139`).
- Display: `CommandOrControl`→primary label; `Command`→`Cmd`; `Cmd`/`Ctrl`→primary label (`hotkeyDisplay.ts:5-13`).
- **Consequence for on-disk format:** `settings.json.hotkeys` may contain either `CommandOrControl+…` (written by main's first-run defaults) or `Ctrl+…`/`Cmd+…` (written by the renderer after recording — renderer `normalizeHotkeys` does not convert). Renderer defaults are `Ctrl+Shift+T` etc. (`settingsDomain.ts:14-20`). A port must accept both and convert at use-time exactly like §5.1.
- Labels (`hotkeyRecorder.ts:111-117`): Toggle Overlay Visibility / Toggle Interactive Mode / Complete Next Task / Undo Last Task Action / Forward Last Task Action.

---

## 6. Single instance & lifecycle (Verified `src/main/lifecycle/singleInstance.ts`, `src/main/index.ts:252-298`)

- `ensureSingleInstance(app, cb)`: if `requestSingleInstanceLock()` is false → `app.quit()`, return false, no listeners registered; else `app.on('second-instance', cb)`, return true (test `singleInstance.test.ts`). Nothing else runs when denied (`index.ts:255-274` guards all setup).
- Second-instance callback: `manuallyHidden = false`; await `initialization` promise; if a live window exists → `showOverlayWindow` (focusable=true, show, alwaysOnTop 'screen-saver', focus, webContents.focus — `windowActions.ts:24-34`); else `overlay.openOverlayWindow()` (create window + start game monitor + install menu) and show. Errors logged as `Failed to reveal Echosight for second instance:` and never thrown (test `indexSecondInstance.test.ts:101-125`). The callback returns `void` (promise chain detached).
- Init failure → log `Failed to initialize Echosight:` + `app.quit()` (`:277-280`).
- `window-all-closed`: `overlay.cleanup()` (stop PoE monitor, `globalShortcut.unregisterAll`) then `app.quit()` unless `darwin` (`:282-287`).
- `activate` (macOS dock): reopen overlay if no windows (`:289-293`).
- `will-quit`: `unregisterOverlayHotkeys()` (`:295-297`).
- Renderer-initiated quit: `quit-application` → `cleanup()` + `app.quit()` (`:230-233`).
- Game monitor (cross-ref `poe2Monitor.ts`): polls process list (`tasklist /fo csv` on Windows, `ps` elsewhere) for a name containing `pathofexile`; show on start unless `manuallyHidden`, hide on stop (`index.ts:157-192`).

---

## 7. Application menu (Verified `src/main/menu/overlayMenu.ts:18-83`)

Single top-level `File` menu, installed via `Menu.setApplicationMenu` and rebuilt whenever hotkeys change (`index.ts:122-131, 214`). Accelerators are the live `currentHotkeys` strings (already in `CommandOrControl` form).

| Label | Accelerator (default) | Action |
|---|---|---|
| Show/Hide Overlay | `toggleVisibility` (Ctrl/Cmd+Shift+T) | `toggleOverlayWindow`; sets `manuallyHidden = !visible` |
| Toggle Interactive Mode | `toggleInteractive` (…+Shift+I) | only if window visible |
| Complete Next Task | `completeNextTask` (…+Shift+N) | send `complete-next-task` if visible |
| Undo Last Task Action | `undoLastAction` (…+Shift+Z) | send `undo-last-task-action` if visible |
| Forward Last Task Action | `redoLastAction` (…+Shift+Y) | send `redo-last-task-action` if visible |
| — separator — | | |
| Quit | `role: 'quit'` (platform default) | |

No context menu is defined in main; the window is frameless so this menu is only reachable via macOS menu bar / accelerators (Inferred).

---

## 8. Window placement (Verified `src/main/overlay/windowPlacement.ts`, test `windowPlacement.test.ts`)

```
DEFAULT_OVERLAY_WIDTH  = 500
DEFAULT_OVERLAY_HEIGHT = 700
DEFAULT_MARGIN         = 20

getOverlayAnchorPosition(workArea, {width=500, margin=20}):
    x = workArea.x + max(0, workArea.width - width - margin)
    y = workArea.y + margin
getOverlayWindowBounds(workArea, {width,height,margin}) = { width, height, x, y }   // top-right corner, inset by margin
```
Examples asserted: 1920×1040 @0,0 → `{500,700,1400,20}`; workArea x=-1920,y=40 → `[-520,60]`; narrow 420-wide area @100,30 → `[100,50]` (clamped, never negative offset).
Used with `screen.getPrimaryDisplay().workArea` at creation (`createOverlayWindow.ts:20-21`) and for `reset-window-position` using the *current* window width (`windowIpc.ts:57-66`). Window position/size are **not persisted**.
BrowserWindow flags (`createOverlayWindow.ts:23-47`): minWidth 450, minHeight 600, transparent, `backgroundColor '#00000000'`, frameless, alwaysOnTop, skipTaskbar, focusable, resizable, not minimizable/maximizable, `show:false` then shown after 1000 ms; sandbox+contextIsolation on, nodeIntegration off; visible on all workspaces; alwaysOnTop level `screen-saver` re-asserted on focus and on each monitor tick.

---

## 9. CI / release / packaging

### 9.1 Workflow `.github/workflows/build.yml` (Verified)
- Triggers: `pull_request`, `push` to `main`, `push` tags `v*`, `workflow_dispatch` (`:3-10`). Top-level `permissions: contents: read` (`:12-13`). Node `22.12.0` (`:16`). No `concurrency:` block exists.
- Jobs: `lint-workflows` (actionlint 1.7.12 downloaded with SHA-256 check, `:19-41`), `verify` (`npm ci`, `npm run verify` = typecheck+test+build, then `electron-builder --linux --dir --publish=never` smoke pack, `:43-63`), `audit-build-chain` (only dispatch/v-tags; `npm audit --json --audit-level=high` assessed by an inline Node script that accepts **only** GHSA-mh99-v99m-4gvg in `brace-expansion` (issue #9) and fails on any other high/critical or on an empty report, `:65-142`), `build` (needs all three; dispatch/v-tag only; matrix macos-latest→`build-mac`→`dist/*.dmg`, windows-latest→`build-win`→`dist/*.exe`, ubuntu-latest→`build-linux`→`dist/*.AppImage`+`dist/*.deb`; `libarchive-tools` on Linux; `if-no-files-found: error`, retention 7 d, `:144-199`), `release` (needs build; v-tag push only; `permissions: contents: write`; checkout `persist-credentials: false`; asserts tag == `v<package.json version>` and that no GitHub release exists yet (curl, fails closed on non-404); requires exactly 1 exe, 2 dmg (one `*-arm64.dmg`, one other), 1 AppImage, 1 deb; `softprops/action-gh-release` with `draft:false, prerelease:false`, `:201-339`).
- All actions pinned to 40-char SHAs. No code signing, notarization, or auto-update configuration anywhere (Verified by absence in workflow and `package.json` build block).

### 9.2 electron-builder (`package.json:22-90`, Verified)
appId `com.poe2tools.task-overlay`; productName `Echosight PoE Task Overlay`; output `dist/`; files `dist-electron/**`, `dist/**`, `package.json`; extraResources `data/`→`data/`. Targets: win `portable` x64 (icon `assets/icon.png`); mac `dmg` x64+arm64 (icon `assets/icon.icns`, category games); linux `AppImage` + `deb` x64 (category Game). An `nsis` block exists but no NSIS target is declared (dead config). Runtime deps: `defu`, `destr` only (`:122-125`); `engines.node ^20.19.0 || >=22.12.0`; overrides pin brace-expansion 1.1.16/2.1.2/5.0.8 (`:101-105`).

### 9.3 What `tests/unit/ci/workflow.test.ts` asserts (Verified, 18 tests)
1. Triggers include pull_request, workflow_dispatch, push branches `main`, tags `v*`; `lint-workflows` and `verify` have no `if` (`:140-147`).
2. Top-level `contents: read`; only `release` has `contents: write` (`:149-156`).
3. `verify` runs `npm ci`, `npm run verify`, `electron-builder --linux --dir --publish=never` (`:158-163`).
4. `build.needs` ⊇ {lint-workflows, verify, audit-build-chain}; exact `if` expression for dispatch/v-tag (`:165-172`).
5. `audit-build-chain` has same `if`, step "Audit build dependencies" with `shell: bash` and `npm audit --json` (`:174-184`).
6. Audit step mentions `GHSA-mh99-v99m-4gvg` (`:186-190`).
7. Executes the extracted `assess()` heredoc against 4 fixture reports: accepted-only→0, accepted+unknown→1, empty→1, finding despite zero metadata counts→1 (`:192-258`).
8. No job/step has `continue-on-error: true` (`:260-267`).
9. `release.needs == ['build']`, `if` is v-tag push only, no `always()`, reads `require('./package.json').version`, `TAG_NAME` env from `github.ref_name`, no `${{` in the run script, uses `$GITHUB_API_URL/repos/$GITHUB_REPOSITORY/releases/tags/$TAG` (`:269-283`).
10. Tag validation script (run in bash with a fake curl): 404→exit 0; 200, 500, curl exit 7, tag mismatch → non-zero (`:285-296`).
11. No `${{` inside any `run:`; build step is bash, `BUILD_SCRIPT` env = `${{ matrix.build_script }}`, run = `npm run "$BUILD_SCRIPT"`; matrix contains windows-latest (`:298-312`).
12. Release checkout `persist-credentials: false` (`:314-317`).
13. Every `uses:` pinned to a 40-hex SHA (`:319-325`).
14. actionlint: no `curl|sh` patterns anywhere; archive name uses `linux_amd64` not `x86_64`; expected SHA present; `sha256sum --check` precedes `tar`; check step runs after download and uses `/tmp/actionlint` (`:327-344`).
15. Upload uses `if-no-files-found: error`; matrix artifact globs exactly `dist/*.exe`, `dist/*.dmg`, and linux containing `dist/*.AppImage` and `dist/*.deb` (`:346-364`).
16. "Prepare release assets" script run in bash: valid set {exe, dmg, -arm64.dmg, AppImage, deb} succeeds and copies all; missing exe / missing dmg / extra dmg / two x64 dmg / extra AppImage / missing deb each fail with `::error::` (`:366-392`).
17. `package.json.overrides` equals the three brace-expansion pins; per-owner (`@electron/asar`, `@electron/universal`, `dir-compare`, `filelist`, `glob`) installed and locked brace-expansion versions match; minimatch still expands `a/{b,c}` and `a/{01..03}`; root brace-expansion is 5.0.8 (`:394-444`).
18. Lockfile contains no `registry.npmmirror.com`; workflow never mentions placeholder/missing-icon generation (`:446-453`).

### 9.4 Landing page `docs/index.html` (Verified via grep)
Title "Echosight, open-source task overlay"; Download buttons all link to `https://github.com/echologist/echoesight-overlay/releases/latest` (`:954, :1169-1179`); FAQ promises defaults Ctrl+Shift+T/I/N/Z/Y, "All bindings are user-configurable" (`:1128`), platforms "Windows (portable .exe), macOS (.dmg, Intel + Apple Silicon), Linux (AppImage + .deb)" (`:1136`), "Hotkeys complete checklist actions only. They do not automate gameplay" (`:1100`).

### 9.5 Misc
- `.env.example`: `ELECTRON_RENDERER_URL=http://localhost:5173/` (dev renderer URL, consumed at `index.ts:107`).
- `.gitignore` ignores `dist/`, `dist-electron/`, `out/`, `.env`, `*.exe`, `*.log`, plus agent dirs `docs/superpowers/`, `instructions/`, `.codex`, `.agents`, `PRODUCT.md`.
- `docs/plans/2026-07-25-edit-task-actions.md` (486 lines): edit-task feature intentionally adds **no IPC, schema, or main-process change**; persists whole `TaskSaveData` through the existing funnel (`:20, :445`).
- Recent intent from `git log`: PR #7 (`b91c0bd`) added atomic writes, quarantine, save-blocking, snapshots; PRs #8/#10/#11 hardened CI (SHA pins, no `${{` in scripts, fail-closed release checks, audit gate). Theme version-sync and sounds landed in `d0b00d4`, `7257bc0`, `5a63e5c`.

---

## 10. Rust port compatibility checklist

Must stay byte/semantics-compatible (or be migrated explicitly):

1. **Directory**: `<appData>/echosight/data/` on all platforms (app name `echosight`, lowercase). If the Rust binary uses a different identifier (e.g. `com.poe2tools.task-overlay` or `Echosight`), add a one-time migration that copies from the legacy path. (Inferred path; Verified sub-layout.)
2. **File names**: `current_tasks.json`, `templates.json`, `settings.json`, `themes/` (single-file `*.json` and `<dir>/theme.json`).
3. **Serialization**: `JSON.stringify(v, null, 2)` — 2-space indent, `\n`, no trailing newline, JS key insertion order, UTF-8 without BOM. Numbers like `1749938954072.5596` must round-trip (f64, shortest-repr printing).
4. **Atomic write**: temp `.<name>.<pid>.<uuid>.tmp` in same dir, exclusive create, fsync, rename; per-path serialization; leave destination untouched on failure.
5. **Parse leniency**: accept whatever destr strict accepts (incl. trailing whitespace, bare `null`/`true`/numbers at top level); treat empty file, truncated JSON, and `__proto__`/`constructor` keys as corrupt.
6. **Quarantine**: copy bytes to `<file>.corrupt-<ISO ts with : and . → ->`, then write fallback (`{tasks:[],currentTemplate:null,snapshots:[]}` / `[]` / `null`); report `corruptBackupPath` only for tasks; block saves after a failed load until a load succeeds.
7. **Bootstrap**: create-if-missing only; seed tasks/templates from bundled `data/`, settings from code defaults (settingsVersion 2, `CommandOrControl+…` hotkeys, transparency 70, theme `echosight`, sounds off/60).
8. **Settings tolerance**: accept missing `settingsVersion`, legacy `backgroundColor` (→ `theme`), missing hotkey keys (fill defaults), missing `sounds`, out-of-range numbers (clamp), and hotkeys in any of `CommandOrControl|Ctrl|Control|Cmd|Command` spellings; persist normalized v2 on change.
9. **Task shape**: tolerate legacy tasks lacking `children/mode/triggers/activated/activatedAt/backgroundOptions`; keep `snapshots` ≤ 5 newest; `currentTemplate` nullable string.
10. **Templates**: top-level JSON array; `tasks[]` entries may carry `children`, `mode`, `backgroundOptions`, `triggerIndices`; optional `imported`/`community` flags.
11. **Themes dir sync**: never overwrite a user-modified theme unless bundled `id`+`author` match and bundled `version` is strictly greater; legacy single-file→folder replacement rule.
12. **Hotkey accelerators**: emit Electron-style strings when registering OS-level shortcuts; exactly five actions with the names above.
13. **Single instance**: second launch must reveal the running overlay (and clear "manually hidden"), not start a second process.
14. **Window**: default 500×700, top-right of primary work area with 20 px margin, min 450×600; no persisted geometry.
15. **No signing/notarization/auto-update** exists today — nothing to preserve, but macOS users currently face Gatekeeper prompts (Inferred).
