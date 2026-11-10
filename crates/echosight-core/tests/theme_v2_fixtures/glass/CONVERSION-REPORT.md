# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- css-source-order at "/customCSS": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS"

- unsupported-property at "/customCSS/0/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"blur(12px) saturate(140%) !important"}

- unsupported-property at "/customCSS/0/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"blur(12px) saturate(140%) !important"}

- unsupported-property at "/customCSS/1/.modal-content/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"blur(18px) saturate(150%) !important"}

- unsupported-property at "/customCSS/1/.modal-content/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"blur(18px) saturate(150%) !important"}

- unsupported-property at "/customCSS/1/.modal-content/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"0 24px 64px rgba(0,0,0,0.38), inset 0 1px 0 rgba(255,255,255,0.42) !important"}

- unsupported-selector at "/customCSS/9/.modal-content-dialog.app-dialog-tone-danger/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(255,90,120,0.82) !important"}

- unsupported-effect at "/effects/blur": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "15px"

- unsupported-effect at "/effects/boxShadow": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "0 8px 32px rgba(0, 0, 0, 0.2), inset 0 1px 0 rgba(255, 255, 255, 0.4)"

- effect-utility-selector at "/effects/textShadow": effect token and known control usages retained; arbitrary v1 utility-class targets have no fixed v2 slot and require explicit token references

    {"selector":".text-shadow","effect":"0 1px 2px rgba(0, 0, 0, 0.5)"}

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "Segoe UI, Tahoma, Geneva, Verdana, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    true

- unsupported-property at "/styles/clickThrough/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(var(--blur))"}

- unsupported-property at "/styles/clickThrough/container/borderRadius": CSS declaration not representable by this bounded converter

    {"borderRadius":"var(--border-radius)"}

- unsupported-property at "/styles/clickThrough/container/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--box-shadow)"}

- unsupported-property at "/styles/clickThrough/container/overflow": CSS declaration not representable by this bounded converter

    {"overflow":"hidden"}

- unsupported-property at "/styles/clickThrough/container/position": CSS declaration not representable by this bounded converter

    {"position":"relative"}

- unsupported-property at "/styles/clickThrough/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(var(--blur))"}

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"transparent","backdropFilter":"blur(5px)","webkitBackdropFilter":"blur(5px)","borderRadius":"8px","border":"1px solid var(--border-secondary)"}

- unsupported-property at "/styles/interactive/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(var(--blur))"}

- unsupported-property at "/styles/interactive/container/borderRadius": CSS declaration not representable by this bounded converter

    {"borderRadius":"var(--border-radius)"}

- unsupported-property at "/styles/interactive/container/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"0 0 20px rgba(50, 205, 50, 0.3), var(--box-shadow)"}

- unsupported-property at "/styles/interactive/container/overflow": CSS declaration not representable by this bounded converter

    {"overflow":"hidden"}

- unsupported-property at "/styles/interactive/container/position": CSS declaration not representable by this bounded converter

    {"position":"relative"}

- unsupported-property at "/styles/interactive/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(var(--blur))"}

- unsupported-property at "/styles/interactive/header/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(10px)"}

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-secondary)"}

- unsupported-property at "/styles/interactive/header/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(10px)"}

- unsupported-property at "/styles/interactive/tasksSection/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(5px)"}

- unsupported-property at "/styles/interactive/tasksSection/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(5px)"}

- unsupported-property at "/styles/interactive/templateSection/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(8px)"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-secondary)"}

- unsupported-property at "/styles/interactive/templateSection/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(8px)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"var(--user-transparency, 1)","interactive":"var(--user-transparency, 1)"}
