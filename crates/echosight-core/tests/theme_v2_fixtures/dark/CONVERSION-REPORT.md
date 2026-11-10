# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- css-source-order at "/customCSS": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS"

- unsupported-property at "/customCSS/0/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"blur(4px) !important"}

- unsupported-property at "/customCSS/0/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"blur(4px) !important"}

- unsupported-selector at "/customCSS/9/.modal-content-dialog.app-dialog-tone-danger/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"#dc143c !important"}

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "Segoe UI, Tahoma, Geneva, Verdana, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    false

- unsupported-property at "/styles/clickThrough/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"brightness(0.8)"}

- unsupported-property at "/styles/clickThrough/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"brightness(0.8)"}

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"transparent"}

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-primary)"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-secondary)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"var(--user-transparency, 0.25)","interactive":"var(--user-transparency, 0.25)"}
