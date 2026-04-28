# Theme Authoring

Echosight themes are JSON files, or folders that contain a `theme.json` plus optional assets and CSS. The app validates themes before loading them, so broken themes are skipped with a clear warning instead of producing half-applied UI.

## Where Themes Live

Use the app's **Open Folder** button in Settings to open the runtime themes directory. Built-in themes are copied there on first run and updated by version when the bundled copy is newer.

For development, bundled themes live in `data/themes`. Runtime user data lives under Electron `userData`, so editing tasks or settings should not dirty the repository.

## Minimal Theme

```json
{
  "id": "my-theme",
  "name": "My Theme",
  "version": "1.0.0",
  "author": "You",
  "colors": {
    "primary": "#d4a857",
    "secondary": "#5fd8c8",
    "danger": "#c54a3a",
    "text": {
      "primary": "#ffffff",
      "secondary": "#cccccc",
      "muted": "#888888"
    },
    "background": {
      "primary": "rgba(0, 0, 0, var(--user-transparency, 0.7))",
      "secondary": "rgba(20, 20, 20, var(--user-transparency, 0.7))",
      "transparent": "transparent"
    },
    "border": {
      "primary": "#d4a857",
      "secondary": "#333333",
      "interactive": "#5fd8c8"
    }
  }
}
```

Required fields:
- `id`: non-empty string, unique across loaded themes.
- `name`: non-empty string shown in Settings.

Common optional fields:
- `version`: string used when bundled themes update user copies.
- `author`: string.
- `description`: string shown as hover text in the theme selector.
- `isDefault`: boolean.
- `supportsTransparency`: boolean. Set `false` for opaque themes.
- `cssFile`: relative path to a `.css` file inside the theme folder.
- `customCSS`: CSS string or an object of named CSS snippets.

## Folder Themes And Assets

A folder theme looks like this:

```text
my-theme/
  theme.json
  styles.css
  main_bg.svg
  button_normal.png
```

Supported image asset extensions:
- `.png`
- `.jpg`
- `.jpeg`
- `.gif`
- `.svg`
- `.webp`

Assets are loaded automatically and exposed to generated CSS as custom properties. For example, `main_bg.svg` becomes:

```css
background-image: var(--asset-main-bg);
```

Asset names are inferred from filenames:
- names containing `background` or `bg` become background assets
- names containing `button` become button assets
- names containing `progress` become progress assets
- names containing `icon`, `texture`, or `border` get matching asset types

## CSS Files

Use `cssFile` when a theme needs stylesheet-level control:

```json
{
  "id": "retro",
  "name": "Retro",
  "cssFile": "styles.css"
}
```

Rules:
- The path must be relative.
- The path must stay inside the theme folder.
- The file must use the `.css` extension.
- Only the stylesheet named by `cssFile` is loaded for that theme.

Relative `url("./asset.svg")` references inside CSS are rewritten to theme asset variables when possible.

## Custom CSS

`customCSS` can be one CSS string:

```json
{
  "customCSS": ".modal-content { border-radius: 8px !important; }"
}
```

It can also be an object of snippets:

```json
{
  "customCSS": {
    "modal": ".modal-content { border-radius: 8px !important; }",
    "hover": ".task-item:hover { background: rgba(255,255,255,0.08) !important; }"
  }
}
```

Object values can also be property maps. A key named `compact` becomes a `.theme-compact` rule:

```json
{
  "customCSS": {
    "compact": {
      "fontSize": "12px",
      "padding": "6px"
    }
  }
}
```

## Generated Sections

The renderer understands these object sections:
- `colors`
- `fonts`
- `effects`
- `transparency`
- `styles`
- `components`
- `backgrounds`
- `layout`
- `animations`
- `compatibility`
- `metadata`

CSS values inside `colors` must be strings or numbers. Nested color groups are allowed, for example `colors.text.primary` or `colors.background.primary`.

`styles` usually contains mode-specific layout:

```json
{
  "styles": {
    "interactive": {
      "container": {
        "background": "var(--bg-primary)",
        "border": "1px solid var(--border-primary)"
      }
    },
    "clickThrough": {
      "container": {
        "background": "rgba(0, 0, 0, var(--user-transparency, 0.7))",
        "border": "none"
      }
    }
  }
}
```

Property names are converted from camelCase to kebab-case when CSS is generated.

## Variants

Variants let one theme expose alternate palettes or styling without copying the whole file:

```json
{
  "id": "my-theme",
  "name": "My Theme",
  "variants": {
    "blue": {
      "name": "My Theme Blue",
      "colors": {
        "primary": "#5fd8c8"
      }
    }
  }
}
```

The loaded variant id becomes `my-theme-blue`. Invalid variants are skipped independently, so a bad variant does not block the base theme.

Variant override fields:
- `name`
- `description`
- `colors`
- `effects`
- `fonts`
- `styles`
- `components`
- `backgrounds`
- `layout`
- `animations`
- `customCSS`

## Validation Errors

Common validation failures:
- `id must be a non-empty string`
- `name must be a non-empty string`
- `supportsTransparency must be a boolean`
- `colors must be an object`
- `colors.primary must be a CSS string or number`
- `cssFile must be a relative path inside the theme folder`
- `cssFile must point to a .css file`
- `customCSS must be a string or object`
- `variants.someVariant must be an object; variant will be ignored`

If a theme has validation errors, it is skipped. If only a variant has validation errors, only that variant is skipped.

## Testing Theme Changes

Run:

```bash
npm run verify
```

This runs type-checking, unit tests, and the production build. The bundled theme validation tests load every theme in `data/themes`, so invalid built-in themes fail fast.
