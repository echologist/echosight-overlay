# Echosight Theme System — Inventory & GPUI Port Feasibility

Repo: `/home/agent/code/echosight-overlay`, branch `feat/exiles-corruption-sounds`, HEAD `5a63e5c`. All paths below are relative to the repo root. Every claim is tagged **[V]** (Verified: read in source / computed from the files) or **[I]** (Inferred: reasoned, not directly observed).

---

## 1. Theme file format as implemented

### 1.1 Discovery & file layout
- Themes live in the user themes dir (`THEMES_DIR`, under Electron `userData`) and are seeded from the bundled `data/themes` (`DEFAULT_THEMES_DIR`, `src/main/index.ts:51-54`). **[V]**
- Two shapes are loaded from the themes dir (`themeLibrary.ts:217-234`): **[V]**
  - **Single-file theme**: any `*.json` directly in the themes dir → `assets = {}`, `path` set (`themeLibrary.ts:236-253`).
  - **Folder theme**: any sub-directory containing `theme.json` (`themeLibrary.ts:255-291`); folder without `theme.json` is warned and skipped (l.260-265).
- Folder themes additionally get: auto-discovered `assets`, `folderPath`, `path`, `preview`, `documentation`, `metadata`, and expanded `variants` (`themeLibrary.ts:274-284`). **[V]**
- Loaded themes are keyed by `theme.id` in a `Map`; a later theme with the same id silently overwrites an earlier one (`themeLibrary.ts:248, 284`). **[V]**

### 1.2 Top-level keys recognised by validation (`themeValidation.ts`)
| Key | Rule | Line |
|---|---|---|
| `id`, `name` | required non-empty string | 70-71, 120-128 |
| `author`, `description`, `version` | optional string | 15-19, 72 |
| `isDefault`, `supportsTransparency` | optional boolean | 21-24, 73 |
| `cssFile` | optional non-empty string; no NUL, not absolute (posix or win32), no `..` segment; extension must be `.css` | 157-175, 379-386 |
| `customCSS` | string, or object whose values are each a CSS string or a (nested) property map of string/number leaves | 177-201, 357-373 |
| `sounds` | object: `enabled?: boolean`, `volume?: 0..1`, `events?: object`; unknown event names → **warning** (ignored), known events must be a filename string or `{file, volume?}`; file must be a bare file name (no `/`,`\`, no `..`, not absolute) with ext in `.mp3 .ogg .wav .m4a` | 50-56, 203-289 |
| `colors` | object; nested objects allowed; leaves must be string or number | 291-302, 357-373 |
| `fonts`, `effects`, `transparency`, `styles`, `components`, `backgrounds`, `layout`, `animations`, `compatibility`, `metadata` | must be an object if present — **no deeper validation** | 26-37, 304-315 |
| `variants` | must be object; non-object → warning, ignored; each entry must be an object (else warning) | 340-355 |
| anything else (`backgroundImage`, `loadedAssets`, …) | not validated, passed through (Theme has an index signature `[key: string]: unknown`, `src/shared/types.ts:145`) | — |

Validation errors → the whole theme is skipped with a logged warning (`themeLibrary.ts:426-436`). Warnings never block loading. **[V]** `isDefault`, `compatibility`, `preview`, `documentation` are loaded but consumed by nothing in the renderer (grep shows no readers outside themeLibrary/validation). **[V]**

### 1.3 Variants (`themeHelpers.ts:60-98`, `themeValidation.ts:88-114`, `themeLibrary.ts:372-424`) **[V]**
- Only expanded for **folder themes** (`themeLibrary.ts:280-282`). Single-file themes' `variants` key is validated but never expanded. **[V]**
- Per variant: `name`/`description` optional strings; `customCSS` and `sounds` validated as above; `colors, effects, fonts, styles, components, backgrounds, layout, animations` must be objects (only `colors` leaf-checked).
- Variant theme = `structuredClone(base)`; `id = "<baseId>-<variantId>"`; name defaults to `"<base name> (<variantId>)"`; sections deep-merged with `defu(overrides, base)` (variant wins); `variants` key deleted from the clone.
- Variant-specific assets from `<theme>/variants/<variantId>/` are merged over base assets (`themeLibrary.ts:398-405`).
- `metadata.isVariant/baseTheme/variantId` set. Invalid variants are skipped individually.
- **No bundled theme uses variants.** [V — all 11 `theme.json`/`*.json` key lists]

### 1.4 Asset auto-discovery (`themeLibrary.ts:293-321`, `themeHelpers.ts:5-44`) **[V]**
- Only the theme folder root is scanned (non-recursive); `theme.json` skipped; extension must be one of `.png .jpg .jpeg .gif .svg .webp .mp3 .ogg .wav .m4a`.
- Asset name = file basename without extension (case preserved; e.g. `main_bg`).
- Asset `type` from the name: audio ext → `sound`; name contains `background` or `bg` → `background`; `button` → `button`; `progress` → `progress`; `icon`, `texture`, `border` → that word; else `misc`. (Note: `.css` is NOT a discoverable asset extension, so the `asset.type === 'css'` branch in `themeAssetLoaders.ts:76` and `themeRuntime.ts:45` is dead code for the bundled flow. **[I]**)
- Preview: first existing of `preview.png|preview.jpg|preview.jpeg|screenshot.png`; docs: first of `README.md|readme.txt|info.txt` (content read into memory) (`themeLibrary.ts:325-353`).
- `metadata.isAdvanced = !!(animations || layout || backgrounds)` (l.361).

### 1.5 Asset / CSS delivery over IPC (`themeIpc.ts`, `themeAssetLoaders.ts`) **[V]**
- Channels: `load-themes`, `get-theme`, `reload-themes`, `open-themes-folder`, `get-themes-path`, `load-theme-css`, `get-theme-asset` (`themeIpc.ts:23-87`).
- `get-theme-asset` → reads file, returns `{data: base64, type, extension, mimeType, isText:false}`; refuses if asset path isn't inside the theme dir (`themeAssetLoaders.ts:62-95`).
- `load-theme-css` → only serves the file **declared** in `theme.cssFile`; requested name must normalise to the same path; `.css` only; path normalised via `path.posix.normalize`, rejecting `.`, `..`, `../`, absolute (`themeAssetLoaders.ts:10-60`). Then rewrites `url('./x.ext')` → `var(--asset-x)` (non-alphanumerics in the name → `-`) (l.35-38). No bundled CSS file contains a `url(` so this rewrite is never exercised by bundled themes. **[V]**
- Path safety primitive: `resolveChildPath(base, ...segments)` throws if the resolved path escapes `base`; `isPathInside` uses `path.relative` (`pathSafety.ts:3-14`).

### 1.6 Bundled-theme sync into the user dir (`themeLibrary.ts:64-215`) **[V]**
- On every `initialize()`, each bundled entry is synced:
  - File (`*.json`): copy if missing; if present, overwrite only when `id` and `author` match **and** bundled `version` > user `version` (numeric dotted compare, `compareThemeVersion` l.455-480; unparsable versions compare equal → no update).
  - Folder: same rule using `theme.json`, then recursive copy of the whole folder (overwrites all files, does not delete extras). Also: migration from legacy single-file `<name>.json` → folder: the legacy file is kept if same/newer version (`shouldKeepSingleFileTheme`), else deleted after the folder copy (`removeReplacedSingleFileTheme`).
- Consequence: a user who edits a bundled theme in place loses the edit when the bundled version is bumped (unless they change `id`/`author`). **[I]**

### 1.7 Shared types (`src/shared/types.ts:101-146`) **[V]**
`ThemeAsset {path?, relativePath?, type, extension}`, `ThemeAssetData {data, type, extension, mimeType, isText}`, `ThemeSoundEvent = 'taskCompleted'|'backgroundActivated'|'undo'|'redo'`, `ThemeSoundConfig {enabled?, volume?, events?}`, `Theme {id, name, description?, assets?, loadedAssets?, cssFile?, sounds?, folderPath?, path?, [key]: unknown}`. Everything visual (`colors`, `styles`, …) is untyped.

---

## 2. CSS generation pipeline (renderer)

### 2.1 Apply sequence (`themeApplicator.ts:19-40`) **[V]**
1. `api.getTheme(id)` → null ⇒ log error, return null (nothing removed/applied).
2. `removeThemeStyleElements()` removes `#theme-style, #background-style, #transparency-style, #font-style, #theme-assets` (`themeRuntime.ts:7-19`; only `theme-style` and `font-style` are ever created today — the other ids are legacy). **[V/I]**
3. `loadThemeAssets`: every non-sound asset is fetched via IPC and stored as `data:<mime>;base64,…` in `theme.loadedAssets[name]` (`themeRuntime.ts:21-56`). Sounds are lazy (loaded by the sound controller).
4. `loadThemeCss`: fetches `cssFile` text (or `''`).
5. `generateThemeCSS(theme, transparency/100, cssFileStyles)` → one string → appended as `<style id="theme-style">` to `<head>`.
6. `<style id="font-style">` appended (if `fonts.primary` present).
7. `document.documentElement.style.setProperty('--user-transparency', t/100)`.

Ordering/precedence **[V]**: `index.html:7` links `styles.css` (base) first; the generated `<style>` elements are appended after, so with equal specificity + `!important` on both sides the theme wins (e.g. base `.overlay-container.click-through{background:transparent !important}` at `styles.css:26-28` is overridden by the theme's generated click-through block). Inside `theme-style` the order is (`themeCss.ts:27-43`):

```
:root { vars }  → background → layout → animations → common → components
→ interactive → clickThrough → customCSS → /* CSS File Styles */ cssFile
```
so `cssFile` is last and beats everything generated; `customCSS` beats all generated blocks; `font-style` (separate element, appended after) beats `theme-style` at equal specificity. Nearly all generated declarations carry `!important`.

### 2.2 `:root` variables (`themeCssVariables.ts`) **[V]**
- `--user-transparency: <t>` (l.10).
- `colors.*`: nested group `background` → `--bg-<key>`; `text` → `--text-<key>`; `border` → `--border-<key>` (+ aliases `--border-light` from `border.primary`, `--border-dark` from `border.secondary`, l.30-33); any other group → `--<group>-<key>`; scalar → `--<key>` (e.g. `--primary`, `--secondary`, `--accent`, `--danger`).
- **Transparency injection** (l.24-26, 61-81): for `background.*` values ≠ `transparent` that do not already contain `var(--user-transparency`: `rgba(r,g,b,a)` → `rgba(r,g,b,calc(a * var(--user-transparency,1)))`; `rgb(r,g,b)` → `rgba(r,g,b,var(--user-transparency,1))`; hex/named colours untouched. Applied even when `supportsTransparency:false` (only the slider is disabled, `settingsController.ts:222`, `settingsUi.ts:97-116`). **[V]**
- `effects.<camelKey>` → `--<kebab-key>` (e.g. `--border-radius`, `--text-shadow`, `--box-shadow`, `--button-shadow`, `--bevel-inset`, `--bevel-outset`, `--drop-shadow`, `--glow`, `--blur`).
- `transparency.<key>` → `--transparency-<kebab>`; **nothing consumes these** (grep). **[V]**
- `loadedAssets` → `--asset-<name with non-alnum→'-'>: url('data:…')`.

### 2.3 Background block (`themeCssBackground.ts`) **[V]**
- If `loadedAssets.main_bg` or `theme.backgroundImage` (name w/o ext) resolves, or `colors.background.primary` is a non-transparent colour: emits `body { background-color; background-image url(data) 100% 100% center no-repeat fixed }` and makes `.overlay-container.interactive/.click-through` background transparent when an image exists.
- `backgrounds` section: each key is an **arbitrary selector** → `background-image(asset)/size/position/repeat/attachment/color` (no `!important`).
- Every asset with `type==='background'` gets `.bg-<rawAssetName>{…cover…}`; no markup uses these classes. **[V]**
- Auto asset→selector mapping by **file name** (l.71-91): `titlebar_bg`→interactive `.header`; `button_normal/hover/pressed`→interactive buttons `/:hover/:active`; `progress_bg`→`.progress-bar`; `progress_fill`→`.progress-fill` (as `background-image … cover no-repeat !important`).

### 2.4 Layout block (`themeCssLayout.ts`) **[V]**
`layout.window{width,height,min*,max*}`→`.overlay-container`; `layout.components.<name>` through `layoutSelectorMap` (header, templateSection, tasksSection, taskList, taskItem, progressBar, buttons, inputs; `themeCssSelectors.ts:1-10`) with a whitelist of 24 box/position/flex/grid properties (truthy values only, so `0` is dropped); `layout.positions.<arbitrary selector>`; `layout.grid`→`.layout-grid`; `layout.flex`→`.layout-flex` (classes unused in markup). **No bundled theme has `layout`.**

### 2.5 Animation block (`themeCssAnimation.ts`) **[V]**
`animations.keyframes.<name>` → `@keyframes`; `animations.components.<name>` via `animationSelectorMap` (container, header, tasks, buttons, progress, modal, inputs); `animations.selectors.<arbitrary selector>`; `animations.transitions.global` → `* { transition }`, `.components`, `.hover.<arbitrary selector>:hover`. Property keys mapped via `animationPropertyMap`. **No bundled theme has `animations`.**

### 2.6 Common block (`themeCssBaseStyles.ts:48-91,120-170`) — always emitted **[V]**
- Utility classes `.bevel-inset/.bevel-outset/.text-shadow/.drop-shadow/.glow/.blur` when the matching `effects` key exists (unused by markup).
- Unconditional: `.task-text{color:var(--text-primary);font-weight:var(--font-weight-normal,normal)}`, `.task-text.completed{color:var(--text-muted)}`, `.progress-text{color:var(--text-secondary);font-weight:var(--font-weight-bold,bold)}`; conditional `text-shadow`, `.progress-bar` box-shadow/radius, `.task-checkbox` filter, button radius/shadow/text-shadow, input radius/bevel.

### 2.7 Components block (`themeCssComponents.ts`, `componentSelectorMap` in `themeCssSelectors.ts:22-54`) **[V]**
Keys: `button, input, select, progressBar, progressFill, taskItem, checkbox` → fixed selector lists. Values: camelCase→kebab property map, every declaration `!important`. `asset:<name>` values become `url(data)` (`backgroundImage`/`background`→`background-image`, `borderImage`→`border-image`). If a component has `states{hover|active|focus|disabled}`, **only** the states are emitted (base props at the same level are ignored, l.28-52). Unknown component keys are dropped. Used only by **cs16**.

### 2.8 Mode blocks (`themeCssBaseStyles.ts:8-46`) **[V]**
`styles.interactive.{container,header,templateSection,tasksSection}` → `.overlay-container.interactive[ .header| .template-section| .tasks-section]`; `styles.clickThrough.{container,tasksSection}` → `.overlay-container.click-through[ .tasks-section]`. Arbitrary CSS properties, all `!important`. Mode class toggled on `#overlayContainer` by `renderInteractiveMode` (`overlayUi.ts:23-41`), plus a hard-coded `glass-mode` class when `themeId==='glass'` (l.26) — a theme-specific special case in app code. **[V]**

### 2.9 customCSS (`themeCssBaseStyles.ts:93-118`) **[V]**
string → emitted verbatim; object → string values emitted verbatim (in key order), object values → `.theme-<key>{ props }` (class unused by markup).

### 2.10 Fonts (`themeRuntime.ts:87-137`) **[V]**
`fonts.primary.{family,sizes{header,task,progress,ui},weights{normal,bold}}` → `body,.overlay-container{font-family}`, `.header h2`, `.task-text`, `.progress-text`, `button,select,input` font-size/family; weights become `--font-weight-normal/bold`. Only `primary` is read.

### 2.11 Base `styles.css` coupling **[V]**
Base stylesheet (1498 lines, 54 `!important`, 5 `@keyframes`) reads only these theme vars: `--border-interactive, --border-secondary, --danger, --secondary, --text-muted, --text-primary, --text-secondary` (+ app-internal `--click-through-shortcut-hint`, drag vars). Base click-through mode also hides the header, template section, delete buttons, and checkboxes and injects a `::before` hint (`styles.css:26-120`).

---

## 3. Bundled theme inventory

| Theme (id, ver) | Shape | colors/fonts/effects | `transparency` | `styles` modes | `components` | `customCSS` | `cssFile` | Image assets | Sounds | Variants/layout/anim | `supportsTransparency` |
|---|---|---|---|---|---|---|---|---|---|---|---|
| echosight 1.0.3 | file | ✓/✓/4 keys | ✓ | ✓ | – | string, 29 rules, 1 `@import` Google Fonts, 1 `::before`, 7 hover, 7 gradients, 2 backdrop-filter | – | – | – | – | true |
| dark 1.0.1 | file | ✓/✓/– | ✓ | ✓ | – | string, 11 rules (modal only) | – | – | – | – | true |
| light 1.0.1 | file | ✓/✓/– | ✓ | ✓ | – | string, 11 rules (modal only) | – | – | – | – | true |
| minimal 1.0.1 | file | ✓/✓/3 | ✓ | ✓ | – | string, 11 rules (modal only) | – | – | – | – | true |
| glass 1.0.1 | file | ✓/✓/4 | ✓ | ✓ (backdrop blur) | – | string, 11 rules (modal only) | – | – | – | – | true |
| cs16 1.0.1 | file | ✓/✓/8 (bevels, glow, drop-shadow) | ✓ | ✓ | ✓ 6 comps | object of 4 snippet strings, 13 rules | – | – | – | – | false |
| exiles_corruption 1.1.0 | folder | ✓/✓/4 | ✓ | ✓ (gradients + backdrop) | – | string, 11 rules (modal) | – | – | ✓ 4 wav (all events), vol 0.8 | – | false |
| winamp-classic 1.0.2 | folder | ✓/✓/6 | – | ✓ | – | – | `styles.css` | 7 SVG (main_bg, titlebar_bg, button_normal/hover/pressed, progress_bg, progress_fill) → auto-mapped | ✓ `complete.wav` → taskCompleted | – | unset (treated true) |
| windows-98 1.0.1 | folder | ✓/✓/6 | – | ✓ | – | – | `styles.css` | – | – | – | false |
| windows-xp 1.0.1 | folder | ✓/✓/6 | – | ✓ | – | – | `styles.css` | – | – | – | false |
| windows-7 1.0.1 | folder | ✓/✓/6 | – | ✓ | – | – | `styles.css` | – | – | – | false |

All **[V]** (keys enumerated per file; rule counts computed by regex over the CSS text). No bundled theme uses `variants`, `layout`, `animations`, `backgrounds`, `backgroundImage`, `asset:` component values, or `states`. **[V]** A unit test asserts every bundled theme's generated CSS contains a `Theme Modal` block with `.modal-content` and `.modal-btn` (`tests/unit/themes/themeModalStyles.test.ts:8-17`) — i.e. hand-written modal CSS is a de-facto requirement today. **[V]**

### 3.1 cssFile themes — what the CSS does (computed over each `styles.css`, comments stripped) **[V]**

| Metric | winamp-classic | windows-98 | windows-xp | windows-7 |
|---|---|---|---|---|
| rules / selectors / declarations | 26 / 43 / 84 | 34 / 56 / 152 | 30 / 52 / 139 | 45 / 76 / 241 |
| `!important` | 84 (100%) | 152 (100%) | 139 (100%) | 241 (100%) |
| selectors under `.overlay-container.interactive` | 25 | 37 | 33 | 56 |
| selectors under `.click-through` | 0 | 2 | 2 | 2 |
| modal/dialog selectors | 18 | 17 | 17 | 18 |
| `linear-gradient` / `radial-gradient` | 2 / 0 | 4 / 0 | 4 / 0 | 20 / 4 |
| `box-shadow` decls (inset tokens) | 2 (4) | 18 (49) | 14 (44) | 19 (15) |
| `text-shadow` decls | 7 | 7 | 7 | 8 |
| `:hover` / `:active` / `:focus` / `:checked` | 5/3/0/0 | 6/4/6/0 | 5/4/6/0 | 8/3/6/2 |
| `::before`/`::after` | 0 | 0 | 0 | 5 (container aero overlay, footer separator ×3, checkbox `✓` glyph) |
| `@keyframes` / `@font-face` / `@import` / `@media` | 0 | 0 | 0 | 0 |
| `filter` / `backdrop-filter` / `transform` | 1 (drop-shadow on checkbox) / 1 / 0 | 1 (`none`) / 1 (`none`) / 0 | 1 / 1 / 0 | 0 / 2 (blur on click-through + modal) / 0 |
| `transition` | 0 | 0 | 0 | 6 |
| `url(` / `var(--asset-…)` / `border-image` | 0 / 0 / 0 | 0 | 0 | 0 |
| `appearance:none` / `outline` / `accent-color` / `text-decoration` | 0/0/1/1 | 0/2/1/1 | 0/2/1/1 | 1/2/0/1 |
| selectors targeting classes that do not exist in the markup | — | `.status-bar`, `.menu`, `.menu-item` | — | `.window-body`, `.window-footer`, `.progress-section`, `.status-section`, `.status-bar-field`, `.window-controls`, `.window-control-btn`, `.has-space`, `button.default` |

Qualitative summary **[V]**:
- **winamp-classic**: flat colours + `outset/inset` borders, green-on-black text with 1px hard text-shadow, uppercase header, hover inverts colours. Imagery comes entirely from the *auto-mapped SVG assets* (body background `main_bg.svg` stretched 100%×100%, title bar, three button states, progress bg/fill), not from the CSS. The SVGs are simple rect/circle/gradient drawings (e.g. `main_bg.svg`: 36 rects, 8 circles, 4 gradients, 1 text). Note `colors.background.primary` is `"pink"` (hidden under the image). 
- **windows-98 / windows-xp**: near-identical structure. The whole look is **multi-layer inset `box-shadow` bevels** (3-4 layers, 44-49 inset tokens), a 2-stop title gradient, pixel font, zero radius, `:active` swaps the bevel, `:focus` dotted outline, hover row = solid navy/blue with white text. XP adds asymmetric top radii on the header.
- **windows-7**: the heaviest: 24 gradients (multi-stop with hard stops at 45%), radial gradients to fake rounded progress caps, 5 pseudo-elements (`::before` glass overlay with `z-index:-1`, footer hairline, `:checked::after` check glyph with `appearance:none`), transitions, focus rings via `box-shadow` spread, backdrop blur in click-through.

---

## 4. Sounds (`src/renderer/features/themes/themeSounds.ts`) **[V]**
- Playback: `new Audio(dataUrl)` (HTMLAudioElement), not Web Audio (l.38). One cached element per event (`audioCache`), rewound with `currentTime = 0` then `play()` (l.71-79). Cache cleared on `applyTheme`.
- Delivery: lazily on first play via IPC `getThemeAsset(themeId, assetName)` → `data:<audio mime>;base64,…` (l.85-97); asset must have an `audio/*` mime.
- Resolution: `sounds.events[event]` string → asset name = file name without extension/dirs, volume 1; object → `{file, volume}` (l.107-131).
- Gate: `settings.sounds.enabled && theme.sounds.enabled !== false` (l.133-137). Default user setting: disabled, volume 60 (`settingsDefaults.ts:9-14`).
- Mixing: `clamp(settings.volume/100, fallback .6) × clamp(theme.sounds.volume, 1) × eventVolume` (l.139-147).
- Events and triggers: `taskCompleted` on `completeNextTask` and on checkbox toggle completing (`taskWorkflowController.ts:109, 216`); `undo` / `redo` (l.298, 312); `backgroundActivated` from the background-task controller (`renderer.ts:75-78`). Failures only log a warning.

---

## 5. Theme selection UI & persistence **[V]**
- Settings modal shows current theme name/id (`settingsUi.ts:42-47`) and a "choose" button that opens `themeSelectionModal` (`settingsController.ts:231-239`).
- Dialog (`themeSelectionUi.ts`): list of buttons (name + "Current – author – version" meta); clicking **previews** (does not apply): details panel (name, description or "No description provided.", ID, Author, Version, Transparency "N% visible"/"Not supported", asset count, CSS = `cssFile` or "Generated") and 6 colour swatches (`primary, secondary, danger, text.primary, background.primary, border.primary`).
- Live preview: the full `generateThemeCSS` output (with assets + cssFile) is run through `scopeThemeCss`, a hand-rolled brace parser that prefixes every selector with `#themeSelectionPreviewRoot` (`:root/html/body` → the root; `@keyframes/@font-face` untouched; other `@` blocks recursed), injected as `<style id="theme-selection-preview-style">` (l.139-174, 306-384). Preview root contains sample interactive **and** click-through containers (`index.html:296-297, 367`). Stale previews are discarded via a request counter.
- Apply: disabled when previewed == current; `onApply` → `settingsController.updateTheme` → `applyRendererTheme` → re-render interactive visuals (`settingsController.ts:185-197`). Theme id persisted in `settings.theme` on **Save Settings** (`saveSettings`, l.241+) — `updateTheme` alone updates in-memory settings. **[V]** (persist-on-save) / whether Apply also saves immediately: not observed → **[I]** it does not.
- Reload: `reload-themes` IPC → library cleared and reloaded → `afterReload` re-renders selector and re-applies `#themeSelect` value → alert "Themes reloaded! Found N themes." (`themeController.ts:52-61`, `renderer.ts:163-166`). Open folder: `shell.openPath(themesDir)` via `openThemesFolder`.
- Fallback: `applyTheme` uses `settings.theme || 'echosight'` (`settingsController.ts:215`); if that id is missing from the library, `getTheme` returns null → error logged, **previous style elements remain** (nothing removed, `themeApplicator.ts:21-25`). Default settings theme = `echosight`, transparency = 70 (`settingsDefaults.ts:19-20`). The selection dialog pre-selects the first theme if current is missing (`themeSelectionUi.ts:386-390`).

---

## 6. Feasibility: Rust/GPUI port with no CSS engine

GPUI facts used here **[I — from general knowledge of Zed's GPUI, not verified in this repo]**: Tailwind-like builder API (`div().flex().bg(..).border_1().rounded(..).shadow(..)`), per-element `hover()/active()/focus()` closures, `BoxShadow` lists (offset, blur, spread, colour; inset is not supported as of the versions I know), linear gradients via `linear_gradient(angle, stops)` on `Fill`, `img()` with image/SVG assets, no `backdrop-filter`, no `filter: drop-shadow`, no pseudo-elements, no `@keyframes` (use `.with_animation(...)` on elements), no selector matching, no `!important`, no `text-shadow` (none known), no `border-image`, no `outline` (emulate with an extra ring/shadow), `accent-color`/`appearance` N/A (checkbox is your own element).

### 6.1 Classification of every current mechanism

| Mechanism | Class | Notes |
|---|---|---|
| `colors.*` flat + `text/background/border` groups | **(a) tokens** | Colour strings need a parser (hex, `rgb[a]()`, named, and the `calc(var(--user-transparency)…)` idiom). Replace with numeric alpha + an explicit `transparencyAffects` flag. |
| `--user-transparency` injection into `background.*` | **(a)** | Becomes `alpha × user_opacity` at build time; semantics are easy to preserve exactly (multiply alpha for rgba, set alpha for rgb, leave hex). |
| `fonts.primary` family/sizes/weights | **(a)** | Family fallback lists map to GPUI font fallbacks; sizes in px → `px()`. Web fonts via `@import` (echosight) ⇒ **bundle font files** instead. |
| `effects.borderRadius`, `blur`(as number) | **(a)** | radius trivial; `blur` is a backdrop blur ⇒ **(c)** (see below). |
| `effects.textShadow` | **(c)** unless GPUI gains text shadow; could be faked by double-rendering text with offset **(b)**. |
| `effects.boxShadow/buttonShadow` (outer) | **(a)** | multi-shadow list parse → `Vec<BoxShadow>`. |
| `bevelInset/bevelOutset` and all **inset** shadows (98/XP/7 bevels) | **(b)** | Not an inset shadow primitive, but a 4-colour **bevel border** (top/left light, bottom/right dark, 1-2px each) can be a first-class component style (`bevel: {light, dark, width}`) rendered as nested borders. Preserves the 98/XP look faithfully. |
| `effects.dropShadow` (filter) on checkbox, `glow` | **(c)** / glow ≈ outer shadow **(a)** |
| `styles.interactive/clickThrough.{container,header,templateSection,tasksSection}` | **(b)** | A fixed set of 6 *slots* with a bounded property set: fill (colour or linear-gradient), border, radius, shadow, padding, text colour/shadow, font. Gradients in `background` need a gradient parser → `linear_gradient`. `backdropFilter` ⇒ **(c)**; `position/overflow` are irrelevant in GPUI. |
| `components.{button,input,select,progressBar,progressFill,taskItem,checkbox}` + `states` | **(b)** | Maps 1:1 to per-component style records with `hover/active/focus/disabled` variants. |
| Asset auto-mapping by filename (`titlebar_bg`, `button_*`, `progress_*`, `main_bg`) | **(b)** | Direct: component style `image: <asset>` with stretch/cover mode; `img()`/SVG rendering in GPUI. Keep the naming convention as sugar. |
| `backgrounds.<selector>`, `layout.positions.<selector>`, `animations.selectors.<selector>`, `transitions.hover.<selector>` | **(c)** | Arbitrary selectors. Unused by bundled themes ⇒ drop. |
| `layout.window/components/grid/flex` | **(b)** partial | window min/max size and per-slot padding/margin/size are fine; grid/flex redefinition of app layout is **(c)**-ish (don't let themes restructure layout). Unused ⇒ drop. |
| `animations.keyframes/components/transitions` | **(c)** as CSS; **(b)** if reduced to a few named effects (`hover_transition_ms`, `progress_fill_ease`, `pulse`). Unused ⇒ drop or define a tiny subset. |
| `customCSS` (string / snippet object / `.theme-<key>` maps) | **(c)** | 100% of bundled themes use it, but **in 9/10 cases only for the modal/dialog family** (`.modal, .modal-content, .modal-title, .modal-btn.primary/.secondary, hint/help text, .community-template, danger tone`) — a bounded component set ⇒ convertible to **(b)** `dialog` slots. echosight additionally restyles buttons/inputs/task rows/context menu and adds a `::before` hairline. |
| `cssFile` | **(c)** | See §6.3 for per-theme survival. |
| Pseudo-elements (`::before` overlays, `:checked::after` glyph) | **(c)** | Replace with built-in component features: checkbox glyph is app-drawn; header/footer hairlines become `divider` tokens. |
| `backdrop-filter` blur/saturate/brightness (glass, echosight, exiles, win7 click-through, modals) | **(c)** | GPUI has no backdrop blur; OS-level window blur (macOS vibrancy / Windows acrylic) is the only substitute and is per-window, not per-element. Glass theme loses its defining effect. |
| `@import` Google Fonts (echosight) | **(c)** → bundle fonts **(a)** |
| `text-transform: uppercase`, `letter-spacing` | **(b)** | uppercase = transform string in app; letter-spacing: GPUI text runs — **[I]** limited support. |
| `text-decoration: line-through` for completed tasks | **(a)** | GPUI supports strikethrough on text runs. |
| `outline` focus rings | **(b)** | render as extra border/ring. |
| `transition` | **(b)** limited (GPUI animations are explicit). |
| Sounds | **(a)** | WAV/OGG/MP3 via `rodio`; same gate/mixing math; no DOM involved. |
| Variants (defu merge) | **(a)** | pure data merge; trivially portable. |
| Version-based bundled sync, path safety, validation | **(a)** | pure filesystem logic; port as-is (serde + `Path::components()` checks). |
| Selection dialog preview (scoped CSS) | n/a | Replace by rendering the sample widgets with the candidate theme struct — simpler than today. |

### 6.2 Proposed **Theme Format v2** (token + component DSL, no CSS)

```jsonc
{
  "format": 2,
  "id": "windows-98", "name": "...", "version": "1.1.0", "author": "...", "description": "...",
  "supportsTransparency": false,

  "tokens": {
    "colors": { "primary": "#1084d0", "secondary": "#000080", "accent": "#f00", "danger": "#f00",
                 "text": { "primary": "#222", "secondary": "#000", "muted": "#666", "inverse": "#fff", "disabled": "#808080" },
                 "surface": { "primary": "#c0c7c8", "secondary": "#c0c7c8", "tertiary": "#c0c7c8" },
                 "border": { "primary": "#000080", "secondary": "#808080", "interactive": "#1084d0", "light": "#fff", "dark": "#808080" } },
    "opacity": { "appliesTo": ["surface.*"], "interactive": 0.95, "clickThrough": 0.3 },   // replaces the calc(var(--user-transparency)) idiom
    "fonts": { "ui": { "family": ["Pixelated MS Sans Serif", "Arial", "sans-serif"], "file": "ms-sans.ttf" },
                "sizes": { "header": 11, "task": 11, "progress": 11, "ui": 11 }, "weights": { "normal": 400, "bold": 700 } },
    "radius": { "default": 0 },
    "shadows": { "window": [ { "x":0,"y":2,"blur":8,"color":"#0000001a" } ] },
    "bevels": { "raised": { "light": "#fff", "dark": "#0a0a0a", "innerLight": "#dfdfdf", "innerDark": "grey", "width": 1 },
                 "sunken": { "light": "#0a0a0a", "dark": "#fff", "innerLight": "grey", "innerDark": "#dfdfdf", "width": 1 } }
  },

  // Bounded style sheet: fixed slot names × fixed property vocabulary × fixed state names.
  "slots": {
    "window.interactive":  { "fill": "@surface.primary", "bevel": "@bevels.raised", "radius": 0, "padding": 2 },
    "window.clickThrough": { "fill": { "color": "@surface.primary", "alpha": 0.9 }, "bevel": "@bevels.raised" },
    "header":              { "fill": { "gradient": { "angle": 90, "stops": [["#000080",0],["#1084d0",1]] } },
                             "text": { "color": "#fff", "weight": "bold", "transform": "none" }, "padding": [2,4] },
    "templateSection":     { "fill": "@surface.primary", "bevel": "@bevels.sunken" },
    "tasksSection":        { "fill": "@surface.primary", "bevel": "@bevels.sunken" },
    "taskRow":             { "padding": [1,2], "states": { "hover": { "fill": "#000080", "text": { "color": "#fff" } } } },
    "taskText":            { "color": "@text.primary", "completed": { "color": "#808080", "strikethrough": true } },
    "button":              { "fill": "@surface.primary", "bevel": "@bevels.raised", "minSize": [75,23], "padding": [0,12],
                             "states": { "active": { "bevel": "@bevels.sunken" }, "focus": { "ring": { "style": "dotted", "color": "#222", "inset": 4 } } } },
    "input":               { "fill": "#fff", "bevel": "@bevels.sunken", "padding": [1,2] },
    "select":              { "$extends": "input" },
    "checkbox":            { "size": 13, "fill": "#fff", "border": { "color": "#808080" }, "accent": "#1084d0" },
    "progressBar":         { "fill": "#fff", "bevel": "@bevels.sunken", "height": 16 },
    "progressFill":        { "fill": { "gradient": {...} } },
    "dialog.scrim":        { "fill": { "color": "#000", "alpha": 0.25 } },
    "dialog.window":       { "fill": "@surface.primary", "bevel": "@bevels.raised", "shadow": [ {"x":3,"y":3,"blur":0,"color":"#00000059"} ] },
    "dialog.title":        { "$extends": "header" },
    "dialog.text":         { "color": "@text.primary" }, "dialog.hint": { "color": "@text.muted" },
    "dialog.button.primary": {...}, "dialog.button.secondary": {...}, "dialog.danger.title": { "fill": { "gradient": {...} } },
    "contextMenu":         {...}, "divider": { "color": "@border.secondary" }
  },

  "images": {                      // explicit, replaces filename auto-mapping (keep the old names as defaults)
    "window.background": { "asset": "main_bg.svg", "mode": "stretch" },
    "header":            { "asset": "titlebar_bg.svg", "mode": "stretch" },
    "button":            { "normal": "button_normal.svg", "hover": "button_hover.svg", "active": "button_pressed.svg", "mode": "nineSlice", "insets": [3,3,3,3] },
    "progressBar": "progress_bg.svg", "progressFill": "progress_fill.svg"
  },

  "effects": { "windowBlur": "none|system" },   // only knob for glass-like looks: OS vibrancy, per window
  "motion":  { "hoverMs": 100, "progressFillMs": 300 },
  "sounds":  { ... unchanged ... },
  "variants": { "<id>": { "tokens": {...}, "slots": {...}, "images": {...}, "sounds": {...} } }   // same defu merge
}
```

Design rules:
1. **Property vocabulary is closed**: `fill` (color | {color,alpha} | {gradient}), `border` {color,width,style}, `bevel`, `radius` (number | [tl,tr,br,bl]), `shadow` (outer list only), `padding/margin` (n | [v,h] | [t,r,b,l]), `minSize/size/height`, `text` {color,size,weight,family,transform,letterSpacing,strikethrough,shadow?}, `ring` (focus), `image`, `opacityAffects`. Unknown keys → validation warning (not error) for forward compatibility.
2. **Closed slot list and state list** (`hover, active, focus, disabled, checked, completed`). This is the "constrained style sheet DSL" — the renderer looks up `theme.slot("button").state(Hover)` and translates to GPUI builder calls; no selector matching ever happens.
3. `@token.path` references replace `var(--…)`; `$extends` replaces selector sharing.
4. **User opacity** is a single float multiplied into `fill.alpha` for slots listed in `tokens.opacity.appliesTo` (default: all `surface.*` colours and window/section fills) — exactly today's `applyUserTransparency` behaviour, but explicit.
5. **v1 → v2 migration** ships as a one-shot converter (TS or Rust) that handles the *generated* parts losslessly: `colors`→`tokens.colors` (background→surface), `fonts`, `effects.borderRadius/boxShadow/buttonShadow/glow`→tokens, `styles.*`→`window.*/header/templateSection/tasksSection` slots, `components.*`→slots, asset filenames→`images`, `sounds` verbatim, `variants` verbatim. For `customCSS`/`cssFile` it emits **a diagnostic listing each unsupported rule** and converts the recognisable modal family (`.modal`, `.modal-content`, `.modal-title`, `.modal-btn.primary/.secondary`, hint/help selectors, `.community-template`, danger tone) into `dialog.*` slots by simple selector→slot table + declaration→property table (colour, background colour/gradient, border, radius, box-shadow (outer only), font-family/size/weight, text-transform, letter-spacing, padding). Everything else is dropped with a warning. `cssFile` and `customCSS` are **rejected by the v2 validator** (error, with the converter's hint), i.e. explicit deprecation rather than silent ignore.
6. Keep: folder layout, `theme.json`, asset discovery rules, path-safety rules, version-based bundled sync, sound rules — all unchanged so users' mental model survives.

### 6.3 Survival estimate per bundled theme

Percentages are my judgement of "visual intent preserved", computed roughly as (declarations mappable ÷ total declarations that have visible effect) and then adjusted for how central the lost effect is. **[I]**

| Theme | Token-only (tokens + the 6 `styles` slots as plain fills) | Token + component DSL (v2 above) | What is lost under v2 | Hard blockers |
|---|---|---|---|---|
| dark | ~85% | **~98%** | nothing material (modal backdrop blur 4px) | – |
| light | ~85% | **~98%** | modal backdrop blur 3px | – |
| minimal | ~90% | **~100%** | – | – |
| echosight | ~55% | **~80%** | `@import` web fonts (bundle Cinzel/Inter/JetBrains Mono instead → recoverable), backdrop `blur+saturate` on container (defining but secondary), `::before` hairline (→ `divider`), `text-shadow` glows on titles (c), transitions | text-shadow, backdrop-filter |
| glass | ~40% | **~55%** | **backdrop blur is the theme** (15px container, 10/8/5px nested, 18px modal); only OS-level window blur can approximate it, and not per-section | backdrop-filter |
| cs16 | ~60% | **~92%** | `filter: drop-shadow` on checkbox, `filter: glow` on header title, 1px hard `text-shadow` (→ fake with offset text or drop) | text-shadow, filter |
| exiles_corruption | ~60% | **~85%** | backdrop blur/saturate/brightness (3 places), red `text-shadow` glow, radial-gradient modal (radial can be added to DSL) ; sounds 100% | backdrop-filter, text-shadow |
| winamp-classic | ~45% (loses all SVG imagery) | **~90%** | hard 1px `text-shadow` (7 decls), `filter: drop-shadow` on checkbox; SVG skin images, bevels, uppercase header, hover inversion all map | text-shadow |
| windows-98 | ~35% (bevels gone) | **~95%** | dotted focus outline → solid/dashed ring; pixel font must be bundled | – |
| windows-xp | ~35% | **~93%** | asymmetric header radii fine; dotted outline; font | – |
| windows-7 | ~30% | **~75%** | 5 pseudo-elements (glass overlay → `window` second fill layer; footer hairline → divider; checkbox glyph → app-drawn ✓ — all recoverable *in app code*, not theme code), multi-stop hard-stop gradients (supported if DSL stops carry positions), radial progress caps (need radial gradient support in DSL), focus ring via shadow spread, backdrop blur click-through + modal, 6 transitions | backdrop-filter; dead selectors (9 classes) were never visible anyway |

Aggregate: **token-only** keeps the 4 flat themes (dark/light/minimal/echosight-core) usable and degrades the 7 others to "palette only". **Token + component DSL** recovers ≥85% of 9/11 themes; the two structural losses are *backdrop blur* (glass, and the glassy accents of echosight/exiles/win7) and *text-shadow/filter glows* (cs16, echosight, exiles, winamp). If GPUI (or a shader pass in the port) adds text shadow and a window-level blur, every bundled theme except glass's nested per-section blur reaches ≥90%.

### 6.4 Recommendations
1. Port loader/validator/sync/sounds/variants as-is (all (a)); they are pure data logic with tests (`tests/unit/main/themeLibrary.test.ts`, `themeValidation.test.ts`, `tests/unit/themes/themeSounds.test.ts`).
2. Freeze v1 and ship the converter + per-rule diagnostics; add `format: 2` and reject `cssFile`/`customCSS` with a message pointing at the diagnostics.
3. Move the three things themes currently fake with CSS into **app-owned widgets** with themeable knobs: custom checkbox (glyph, accent), bevel border primitive, dialog family slots. That alone converts the modal `customCSS` present in 10/10 bundled themes.
4. Remove the `glass-mode` special case in app code (`overlayUi.ts:26`) in favour of `effects.windowBlur`.
5. Drop never-used v1 surface area in v2: `transparency` section (dead vars), utility classes, `layout.grid/flex`, `backgrounds/positions/selectors` arbitrary-selector maps, `isDefault`, `compatibility`, `documentation`, `preview` (or make preview a real feature).

---

## 7. Noteworthy quirks found (all **[V]**)
- `generateComponentCSS`: when `states` exists, sibling base properties are silently ignored (`themeCssComponents.ts:28-52`).
- `generateLayoutProperties`/`generateAnimationProperties` drop falsy values (`0`, `"0"` is truthy but numeric 0 isn't).
- `applyUserTransparency` applies to `supportsTransparency:false` themes too; the flag only disables the slider.
- `transparency.*` tokens are emitted but never read; `.bg-*`, `.theme-*`, `.layout-*`, `.bevel-*`, `.glow`, `.blur` classes have no consumers in markup.
- `customCSS` and `cssFile` can inject arbitrary CSS including `@import` of remote URLs (echosight does) — a network fetch controlled by theme files.
- On a missing theme id the previous theme's `<style>` stays in place (no reset to default).
- Bundled CSS files reference 12 class names that don't exist in the DOM (listed in §3.1), i.e. dead rules carried over from 98.css/7.css.
- `README`/preview files are read into the theme object and shipped over IPC on every `load-themes` call but never displayed.
