# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- css-source-order at "/customCSS": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS"

- malformed-declaration at "/customCSS/0": declaration has no colon

    ""

- unsupported-property at "/customCSS/0/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"none !important"}

- unsupported-property at "/customCSS/0/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"none !important"}

- malformed-declaration at "/customCSS/1": declaration has no colon

    ""

- malformed-declaration at "/customCSS/10": declaration has no colon

    ""

- malformed-declaration at "/customCSS/2": declaration has no colon

    ""

- malformed-declaration at "/customCSS/3": declaration has no colon

    ""

- malformed-declaration at "/customCSS/4": declaration has no colon

    ""

- malformed-declaration at "/customCSS/5": declaration has no colon

    ""

- malformed-declaration at "/customCSS/6": declaration has no colon

    ""

- malformed-declaration at "/customCSS/7": declaration has no colon

    ""

- malformed-declaration at "/customCSS/8": declaration has no colon

    ""

- malformed-declaration at "/customCSS/9": declaration has no colon

    ""

- unsupported-selector at "/customCSS/9/.modal-content-dialog.app-dialog-tone-danger/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"#ff4a4a !important"}

- unsupported-effect at "/effects/dropShadow": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "none"

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "Segoe UI, Tahoma, Geneva, Verdana, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    true

- unsupported-property at "/styles/clickThrough/container/background": CSS declaration not representable by this bounded converter

    {"background":"none"}

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"none"}

- unsupported-property at "/styles/interactive/container/background": CSS declaration not representable by this bounded converter

    {"background":"none"}

- unsupported-property at "/styles/interactive/header/background": CSS declaration not representable by this bounded converter

    {"background":"none"}

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"2px solid var(--border-primary)"}

- unsupported-property at "/styles/interactive/tasksSection/background": CSS declaration not representable by this bounded converter

    {"background":"none"}

- unsupported-property at "/styles/interactive/templateSection/background": CSS declaration not representable by this bounded converter

    {"background":"none"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"2px solid var(--border-primary)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"1.0","interactive":"1.0"}
