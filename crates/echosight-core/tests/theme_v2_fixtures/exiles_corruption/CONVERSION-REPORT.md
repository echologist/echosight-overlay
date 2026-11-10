# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- css-source-order at "/customCSS": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS"

- malformed-declaration at "/customCSS/0": declaration has no colon

    ""

- unsupported-property at "/customCSS/0/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"blur(7px) saturate(130%) !important"}

- unsupported-property at "/customCSS/0/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"blur(7px) saturate(130%) !important"}

- malformed-declaration at "/customCSS/1": declaration has no colon

    ""

- unsupported-property at "/customCSS/1/.modal-content/background": CSS declaration not representable by this bounded converter

    {"background":"radial-gradient(circle at 20% 0%, rgba(75,0,130,0.82), rgba(25,0,25,0.98) 48%, rgba(12,0,16,0.98)) !important"}

- unsupported-property at "/customCSS/1/.modal-content/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"0 0 28px rgba(220,20,60,0.32), 0 24px 52px rgba(0,0,0,0.72), inset 0 1px 0 rgba(220,20,60,0.22) !important"}

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

    {"border-color":"#ff1744 !important"}

- unsupported-effect at "/effects/blur": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "8px"

- unsupported-effect at "/effects/boxShadow": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "0 0 20px rgba(220, 20, 60, 0.4), 0 0 40px rgba(139, 0, 0, 0.2), inset 0 1px 0 rgba(220, 20, 60, 0.3)"

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "Segoe UI, Tahoma, Geneva, Verdana, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    false

- unsupported-property at "/styles/clickThrough/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(3px) saturate(1.1) brightness(0.7)"}

- unsupported-property at "/styles/clickThrough/container/background": CSS declaration not representable by this bounded converter

    {"background":"linear-gradient(135deg, rgba(25, 0, 25, var(--user-transparency, 0.15)) 0%, rgba(35, 0, 35, calc(var(--user-transparency, 0.15) * 1.2)) 50%, rgba(25, 0, 25, var(--user-transparency, 0.15)) 100%)"}

- unsupported-property at "/styles/clickThrough/container/borderRadius": CSS declaration not representable by this bounded converter

    {"borderRadius":"var(--border-radius)"}

- unsupported-property at "/styles/clickThrough/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(3px) saturate(1.1) brightness(0.7)"}

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"linear-gradient(180deg, rgba(25, 0, 25, calc(var(--user-transparency, 0.15) * 0.8)) 0%, rgba(35, 0, 35, calc(var(--user-transparency, 0.15) * 1.1)) 100%)","position":"relative"}

- unsupported-property at "/styles/interactive/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(var(--blur)) saturate(1.2)"}

- unsupported-property at "/styles/interactive/container/borderRadius": CSS declaration not representable by this bounded converter

    {"borderRadius":"var(--border-radius)"}

- unsupported-property at "/styles/interactive/container/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--box-shadow)"}

- unsupported-property at "/styles/interactive/container/overflow": CSS declaration not representable by this bounded converter

    {"overflow":"hidden"}

- unsupported-property at "/styles/interactive/container/position": CSS declaration not representable by this bounded converter

    {"position":"relative"}

- unsupported-property at "/styles/interactive/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(var(--blur)) saturate(1.2)"}

- unsupported-property at "/styles/interactive/header/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(5px)"}

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-primary)"}

- unsupported-property at "/styles/interactive/header/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(5px)"}

- unsupported-property at "/styles/interactive/tasksSection/position": CSS declaration not representable by this bounded converter

    {"position":"relative"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-secondary)"}

- unsupported-property at "/styles/interactive/templateSection/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"inset 0 1px 3px rgba(220, 20, 60, 0.2)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"0.15","interactive":"0.9"}
