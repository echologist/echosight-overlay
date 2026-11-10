# Theme Format v2 (GPUI authoring contract)

Status: implemented **core data contract**, not a renderer or a visual-parity claim. The Electron format remains documented in `themes.md`; neither its sources nor bundled theme data are replaced by this work.

The normative structural contract is `crates/echosight-core/schema/theme-v2.schema.json` (JSON Schema 2020-12). The Rust validator interprets that same embedded schema, then performs reference, inheritance, asset-path, color, gradient-order and merged-variant checks. Unknown fields are retained with warnings. General schema validators must also run the core semantic validator: custom formats (`theme-color`, `theme-asset`, `theme-sound`), reference graphs and defu merges cannot be checked by ordinary structural schema validation alone.

## Small complete example

```json
{
  "format": 2,
  "id": "my-theme",
  "name": "My Theme",
  "version": "1.0.0",
  "author": "Theme author",
  "description": "An authored palette, not a theme-ID-specific renderer preset",
  "supportsTransparency": true,
  "tokens": {
    "colors": {
      "surface": { "main": "rgba(20,24,30,0.9)" },
      "text": { "primary": "#eeeeee", "muted": "#888888" },
      "accent": "#4488ff"
    },
    "opacity": { "appliesTo": ["surface.*", "window.*"] },
    "fonts": {
      "ui": { "family": ["Arial", "sans-serif"] },
      "sizes": { "ui": 12 },
      "weights": { "normal": 400, "bold": 700 }
    },
    "radius": { "default": 4 },
    "bevels": { "raised": { "light": "#ffffff", "dark": "#333333", "width": 1 } }
  },
  "slots": {
    "window.interactive": { "fill": "@surface.main", "radius": "@radius.default" },
    "window.clickThrough": { "$extends": "window.interactive" },
    "button": {
      "fill": "@colors.accent",
      "text": { "color": "@text.primary", "family": "@fonts.ui.family", "size": "@fonts.sizes.ui" },
      "padding": [4, 8],
      "states": {
        "hover": { "bevel": "@bevels.raised" },
        "disabled": { "text": { "color": "@text.muted" } }
      }
    },
    "button.primary": { "$extends": "button", "text": { "weight": "@fonts.weights.bold" } }
  }
}
```

Only `format`, nonblank `id`, and nonblank `name` are required. Metadata strings and `supportsTransparency` are optional. No implicit palette, layout size, font, border width or effect is injected. An omitted slot returns `None`; a present empty slot remains empty. The app chooses its own explicit fallback styles.

`detect_format(&Value)` returns v1 for absent `format` or numeric `1`, v2 for numeric `2`, and an error otherwise. JSON numbers compare by value: `2`, `2.0` and `2e0` all select v2 and pass its schema; `1`, `1.0` and `1e0` select v1. Strings, booleans, null and other numeric values do not select v2. **This module does not wire dispatch into the v1 filesystem library.** `Theme::parse` specifically requires format 2. Top-level/variant `cssFile` and `customCSS` are errors even when null or empty.

## Tokens and references

- `colors`: arbitrary nested groups with color leaves; conventional groups are `surface`, `text`, `border`, plus flat `primary`, `secondary`, `accent`, `danger`.
- `fonts`: `ui`, `header`, `mono`, each `{family: [nonempty family names], file?: relative asset path}`; arbitrary numeric `sizes` and `weights` maps. Sizes are nonnegative logical pixels; weights are 1–1000.
- `radius`: named nonnegative numbers or four-corner arrays.
- `shadows`: named outer-shadow arrays.
- `bevels`: named `{light, dark, innerLight?, innerDark?, width?}` records. Colors are not inferred from a theme ID.
- `opacity`: see below.

Where allowed by the schema, a value can be a whole-value `@token.path` reference, e.g. `@colors.accent`, `@fonts.ui.family`, `@shadows.window`, `@bevels.raised`. `@tokens.colors.accent` is equivalent. `@surface.x`, `@text.x`, `@border.x` abbreviate `@colors.surface.x`, etc. No string interpolation or CSS variables. Token names containing dots cannot be addressed by dot-path references; use letters, numbers, `_` or `-` for addressable names.

References resolve recursively and must have the destination property's type after resolution. Missing references and cycles are errors, including unused token definitions. Literal metadata, font-family array entries and file/asset names beginning with `@` remain literal. Unknown properties are not interpreted as references. Reference expansion, semantic checks and opacity matching all follow the selected schema branch and its declared child positions, including schema-valued token maps. Opaque extension subtrees are copied without interpreting their strings or field names; they remain subject to input and expansion budgets.

### Colors

The bounded sRGB parser accepts `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`; comma-form `rgb(r,g,b)` and `rgba(r,g,b,a)` with numeric channels 0–255 and alpha 0–1; `transparent`; the sixteen basic CSS names, `grey`, and `pink`. Hex alpha is divided by 255, without intermediate rounding. Percent channels, HSL, modern space/slash RGB, `currentColor`, other named colors, CSS `var`, and arbitrary `calc` are not v2 colors. Prefer hex for portable authoring.

## Slots

The complete fixed vocabulary is:

```
window.interactive window.clickThrough
header headerTitle templateSection tasksSection sectionTitle
taskRow taskText checkbox subtaskRow backgroundRow
badge.high badge.active badge.dormant
button button.primary button.danger button.icon
input textarea select slider
progressBar progressFill progressText toast contextMenu contextMenuItem
dialog.scrim dialog.window dialog.title dialog.text dialog.hint
dialog.button.primary dialog.button.secondary dialog.danger.title
communityTemplateCard divider dragPlaceholder clickThroughHint
```

Unknown slot names warn, and their values must still be style objects. A slot does not change application structure or define selectors. In particular, `tasksSection` has no click-through variant in this vocabulary; the app must decide how to expose such mode differences in a later contract rather than silently conflating the two.

### Style properties

| Property | Shape / units |
|---|---|
| `fill` | color string; `{color, alpha?}`; `{gradient:{angle,stops},alpha?}`; `{radial:{center,radius,stops},alpha?}` |
| `border` | `{color?,width?,style?:solid\|dashed\|dotted\|none}` |
| `bevel` | `{light,dark,innerLight?,innerDark?,width?}` |
| `radius` | nonnegative pixels or `[topLeft,topRight,bottomRight,bottomLeft]` |
| `shadow` | array of `{x,y,color,blur?,spread?}`; no inset shadows |
| `padding` | nonnegative pixels, `[vertical,horizontal]`, or `[top,right,bottom,left]` |
| `margin` | same shapes as padding; negative values allowed |
| `size`, `minSize` | nonnegative scalar (square), or `[width,height]` |
| `height` | nonnegative pixels |
| `text` | `{color?,size?,weight?,family?,transform?,letterSpacing?,strikethrough?,shadow?}` |
| `ring` | `{color?,width?,style?,inset?}`; style vocabulary same as border; signed inset |
| `image` | `{asset,mode:stretch\|cover\|nineSlice,insets?}`; nineSlice requires four nonnegative pixel insets |
| `opacityAffects` | explicit boolean overriding opacity inclusion/exclusion |

Gradient `angle` is degrees in CSS orientation (0 up, 90 right); consumers must convert to their renderer's angle convention. Stops are `[[color,position], ...]`, at least two, positions 0–1 in nondecreasing order (equal positions permit hard stops). Radial center is normalized `[x,y]` in 0–1; radius is nonnegative in units of the shorter side. Rendering geometry is a consumer responsibility. Fill alpha multiplies every stop's own alpha. `text.transform` is `none`, `uppercase`, `lowercase` or `capitalize`; letter spacing is signed logical pixels. Text shadows use the same shadow records; representation does not promise native GPUI soft-glow fidelity.

Object fills use **first valid branch in schema order**: solid `color`, then linear `gradient`, then `radial`. Presence of a key is not enough: `{color:null,gradient:{angle:0,stops:[["red",0],["blue",1]]}}` is a linear fill. If several branches validate, solid wins over linear, and linear wins over radial, independently of JSON key order. Keys outside the selected branch are retained with warnings and are not interpreted as references, gradient geometry or opacity inputs. A selected branch's missing/cyclic references or semantic errors remain fatal; the resolver does not retry another branch to hide those errors. Authors and converters should emit only their intended branch; renderer consumers should use `ResolvedStyle::fill()` rather than dispatching on key presence. The structural schema remains open (`anyOf`), not a blanket unknown-property rejection.

Unknown properties warn and survive in `properties()`, but consumers must ignore properties they do not implement. An unknown object containing `mode: "nineSlice"` or descending `stops` does not trigger image/gradient validation. Those checks still apply at supported destinations, including reference-expanded shapes. Partial border/text/ring records intentionally do not acquire defaults.

## Inheritance and state precedence

1. Top-level `images` mappings provide default images to their named slots and image states. Explicit slot/state `image` values win over these defaults.
2. `$extends: "slotName"` recursively inherits the parent's base **and state definitions**. Missing parents and cycles are errors.
3. Child properties merge over inherited properties. Objects merge recursively; scalar values and arrays replace, never concatenate. `fill` replaces as a whole tagged union, avoiding a stale gradient when a state supplies a solid fill. An image with an explicit mode replaces the whole image record, avoiding stale nine-slice insets.
4. For `slot(name, active_states, user_opacity)`, start with the inherited base. Apply the present states in this fixed order, lowest to highest: **completed, checked, hover, focus, active, disabled**. Caller ordering and duplicate states do not matter. Thus an inherited hover can override a child base, and disabled wins a conflict with hover. Omitted state fields retain lower-priority values. States cannot themselves extend slots or nest other states.
5. Resolve token references in the effective style, then apply fill opacity once. `states` and `$extends` do not appear in the returned properties.

### Opacity

`tokens.opacity.appliesTo` is an explicit array of slot names and/or color-token paths. A trailing `*` is prefix matching. `surface.*` and `colors.surface.*` both match surface-token references, including aliases and gradient stops. Without the array, defaults are `surface.*`, `window.*`, `templateSection`, `tasksSection`; an empty array matches nothing. Only references in the selected fill branch count: `{color:"red",future:"@surface.foo"}` is not faded merely because its unknown field looks like a surface reference. Known fill references through aliases still count.

For a matched slot, the fill's alpha is multiplied by clamped `user_opacity` (0–1; nonfinite input uses 1). `opacityAffects:false` excludes it; `true` includes it regardless of the list. On the two window slots only, optional `tokens.opacity.interactive` / `clickThrough` is another 0–1 multiplier. References to these multipliers are supported. A matched gradient fades all stops uniformly. Text, borders, images, children and shadows are **not** faded. `supportsTransparency:false` is UI capability metadata, not a resolver bypass, preserving the v1 distinction.

## Images, effects, motion, sounds, variants

`images` uses the same slot names. Each entry is `{asset,mode,insets?,states?:{hover:{asset,mode,...},...}}`; it is a default mapping, not a second paint layer. Use a state-only slot image if there is no base image. Assets and font files are relative slash-separated paths, no empty/dot/dotdot segments, backslash, colon, NUL or absolute root. This is lexical validation only: the filesystem adapter must still use the existing canonical theme-root containment checks and inspect asset types before reading.

`effects.windowBlur` is `none` or `system`. `motion.hoverMs`, `progressFillMs`, `toastMs` are nonnegative milliseconds. These values are retained; core never starts an animation or enables OS blur.

Sounds keep the v1 shape: optional `enabled`, volume 0–1, and `events.taskCompleted/backgroundActivated/undo/redo` mapping to a bare filename or `{file,volume?}`. Extensions: wav/ogg/mp3/m4a, case-insensitive. Unknown events warn. Playback remains with the existing sound subsystem.

`variants` maps IDs to patches. `Theme::variant(id)` merges a patch over the base using the existing **defu** helper: null falls back, objects recurse, arrays concatenate override-first, and override-side prototype keys are ignored. The generated ID is `<base>-<variant>`, the name defaults to `<base name> (<variant>)`, and the variants map is removed. The effective merged document is validated. This is intentionally **different from slot inheritance's array replacement**. To replace a gradient or family array, author a distinct token and change the reference instead of trying to overwrite an inherited array. Variant patches are structurally open so nested null fallback is legal; semantic acceptance requires validating their merged documents.

## API for the GPUI style writer

```rust
use echosight_core::theme::v2::{Theme, State, Fill};
# fn apply_document(document: &serde_json::Value) -> Result<(), Vec<echosight_core::theme::v2::Diagnostic>> {
let theme = Theme::parse(document)?;
// Surface theme.diagnostics() to the chooser/editor even on success.
if let Some(style) = theme.slot("button.primary", &[State::Hover], 0.7) {
    if let Some(Fill::Solid(rgba)) = style.fill() {
        // rgba channels are f64 in 0..1; map to the renderer's color type.
        let _ = rgba;
    }
    let _text = style.text();
    let _border = style.border();
    let _bevel = style.bevel();
    let _image = style.image();
    let _shadows = style.shadows();
    let _padding = style.edges("padding");
    let _height = style.number("height");
    let _remaining_supported_fields = style.properties();
}
# Ok(()) }
```

`Fill` is typed (`Solid`, `Linear`, `Radial`) and already includes opacity in its RGBA channels. Text/border/bevel/shadow color strings are validated and resolved; use `Rgba::parse`. Geometry fields not yet given dedicated accessors remain resolved supported JSON, not opaque unvalidated source. `document()` preserves the author document for metadata/effects/motion and round trips; it is not a pre-resolved style. Consumers must explicitly map slots, states, images, bevels and text shadows. Cache resolved styles per theme/state/opacity if appropriate; this core does not own a GPU or UI cache.

## Diagnostics and limits

`Theme::parse` returns `Result<Theme, Vec<Diagnostic>>`. Each diagnostic has severity, JSON-pointer location, code, message and offending value. Destination-type errors may contain the resolved value; reference errors contain the authored reference. Warnings are sorted by location/code/message/severity and deduplicated. CSS converter locations extend pointers with escaped selectors and declaration names.

These are **parser resource guards**, not schema vocabulary changes or accepted visual deltas. Unknown/future properties remain warning-only and opaque within the limits; they are charged for storage even when their semantics are ignored. The independent JSON Schema describes shapes, not allocation limits. All 11 converted fixture themes remain accepted without fixture changes.

| Resource | Limit / policy |
|---|---|
| Input | 64 levels, 100,000 JSON value nodes, 16 MiB of UTF-8 string values plus object keys; inspected without cloning or building a breadth-sized queue of diagnostic paths |
| Materialization work | One shared allowance of **1,000,000 nodes and 16 MiB of UTF-8 payload bytes** for a parse, including all variants. Both limits must fit. Checked multiplication/subtraction fails closed on overflow. This is conservative clone-work accounting, not serialized size or an exact RSS promise. |
| Before document/image copies | Reserve six copies of the input to cover the owned document, slot/image/state mapping and validation scaffolding. This means the effective source-byte acceptance limit can be lower than 16 MiB. |
| Before inheritance copies | Reserve three copies of each visited ancestor on **every** slot traversal, including unknown slots, overwritten values and opaque subtrees. This covers merge materialization and the expanded validation copy; reference output is charged separately. Work is not refunded when temporary values are dropped. |
| Before variant merge | Charge the initial document clone plus both sides of each overlapping object subtree that the existing recursive defu helper copies. Merged variant parsing spends the same allowance, not a fresh one. |
| Reference expansion | The existing 64-level / 100,000-visit limits remain; charge each output node and string/key bytes to the shared materialization allowance **before** allocating it. Stop on the first failed expansion rather than producing a diagnostic per remaining node. |
| Renderer access | A slot lookup reserves its base/state copies and charges reference output before materialization. It returns `None` if that bounded operation cannot complete. |
| Diagnostics | At most 512 entries per collection, including aggregated variant diagnostics. Exhaustion installs an error with code `budget`, never silently hides errors behind warnings. Values larger than 64 nodes or 4 KiB UTF-8 payload are represented as `null` with an omission message; the actual theme property is retained. Diagnostic paths/codes/text are truncated at UTF-8 boundaries to 1 KiB (messages may add the fixed omission suffix). |

The cumulative allowance intentionally overestimates some merges: overwritten inherited data and already-dropped variant intermediates still consume work. This bounds both allocation amplification and repeated traversal without introducing a general allocator or changing extension interpretation. It is not a guarantee that arbitrary caller-owned JSON can be created/deserialized within these limits; the loader must separately bound file reading/JSON decoding. Invalid or over-budget data returns diagnostics, not a partial runnable theme. No schema/runtime/Cargo dependency was added.

### Allocation regression probe

```sh
CARGO_TARGET_DIR="$PWD/target" cargo test --locked -p echosight-core
python crates/echosight-core/tests/theme_v2_budget_check.py
# Optionally append exact saved review JSON paths; each must return a budget error.
python crates/echosight-core/tests/theme_v2_budget_check.py /path/to/known-inherit-90000.json
```

The script builds the current production-API integration-test executable in this worktree's `target` directory and prints its path. Only Linux child probes receive 512 MiB address-space, 30-second CPU and zero-core-dump limits; non-Linux skips that resource-specific lane, not the portable Rust budget/diagnostic tests. Each result reports exit status, actual parser outcome and `wait4` peak RSS. Fatal payloads are never run in the host test process. Cases include all 41 declared slots inheriting an opaque 90,000-element array, the 50,000-element control, a 64 KiB string inherited by 1,000/3,000 future slots, known-slot string-byte amplification, reference node/byte expansion, cumulative variants and multi-ancestor inheritance. Large payloads are generated temporarily, not checked into golden fixtures.
