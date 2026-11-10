# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- css-source-order at "/customCSS": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS"

- malformed-declaration at "/customCSS/0": declaration has no colon

    ""

- unsupported-property at "/customCSS/0/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"blur(3px) !important"}

- unsupported-property at "/customCSS/0/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"blur(3px) !important"}

- malformed-declaration at "/customCSS/1": declaration has no colon

    ""

- unsupported-property at "/customCSS/1/.modal-content/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"0 18px 38px rgba(0,0,0,0.22), inset 0 1px 0 rgba(255,255,255,0.9) !important"}

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

    {"border-color":"#dc143c !important"}

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "Segoe UI, Tahoma, Geneva, Verdana, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    true

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"transparent"}

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-secondary)"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-secondary)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"var(--user-transparency, 1)","interactive":"var(--user-transparency, 1)"}
