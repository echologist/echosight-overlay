# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- unsupported-property at "/components/button/cursor": CSS declaration not representable by this bounded converter

    {"cursor":"pointer"}

- unsupported-property at "/components/button/transition": CSS declaration not representable by this bounded converter

    {"transition":"none"}

- unsupported-property at "/components/checkbox/accentColor": CSS declaration not representable by this bounded converter

    {"accentColor":"var(--primary)"}

- unsupported-property at "/components/checkbox/filter": CSS declaration not representable by this bounded converter

    {"filter":"var(--drop-shadow)"}

- unsupported-property at "/components/input/border": CSS declaration not representable by this bounded converter

    {"border":"2px inset var(--border-primary)"}

- unsupported-property at "/components/input/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--bevel-inset)"}

- font-availability at "/components/input/fontFamily": family names retained; no font file is bundled or downloaded

    "system-ui, sans-serif"

- unsupported-property at "/components/progressBar/border": CSS declaration not representable by this bounded converter

    {"border":"1px inset var(--border-dark)"}

- unsupported-property at "/components/progressBar/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--bevel-inset)"}

- unsupported-property at "/components/progressFill/height": CSS declaration not representable by this bounded converter

    {"height":"100%"}

- unsupported-property at "/components/select/border": CSS declaration not representable by this bounded converter

    {"border":"2px outset var(--border-primary)"}

- font-availability at "/components/select/fontFamily": family names retained; no font file is bundled or downloaded

    "system-ui, sans-serif"

- css-source-order at "/customCSS/authentic": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS/authentic"

- css-source-order at "/customCSS/glow": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS/glow"

- unsupported-property at "/customCSS/glow/0/.header h2/filter": CSS declaration not representable by this bounded converter

    {"filter":"var(--glow) !important"}

- css-source-order at "/customCSS/modal": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS/modal"

- unsupported-property at "/customCSS/modal/0/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"none !important"}

- unsupported-property at "/customCSS/modal/0/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"none !important"}

- unsupported-property at "/customCSS/modal/1/.modal-content/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"inset 2px 2px 0 #8c9284, inset -2px -2px 0 #292c21, 4px 4px 0 rgba(41,44,33,0.55) !important"}

- font-availability at "/customCSS/modal/1/.modal-content/font-family": family names retained; no font file is bundled or downloaded

    "system-ui, sans-serif !important"

- unsupported-property at "/customCSS/modal/5/.community-template/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"inset 1px 1px 0 #8c9284, inset -1px -1px 0 #292c21 !important"}

- unsupported-selector at "/customCSS/modal/8/.modal-content-dialog.app-dialog-tone-danger/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"#a0aa95 !important"}

- css-source-order at "/customCSS/retro": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS/retro"

- unsupported-effect at "/effects/bevelInset": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "inset 1px 1px 0px var(--border-light), inset -1px -1px 0px var(--border-dark)"

- effect-utility-selector at "/effects/bevelOutset": effect token and known control usages retained; arbitrary v1 utility-class targets have no fixed v2 slot and require explicit token references

    {"selector":".bevel-outset","effect":"1px 1px 0px var(--border-light), -1px -1px 0px var(--border-dark)"}

- unsupported-effect at "/effects/boxShadow": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "inset 2px 2px 0px var(--border-light), inset -2px -2px 0px var(--border-dark)"

- unsupported-effect at "/effects/dropShadow": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "drop-shadow(2px 2px 2px rgba(41, 44, 33, 0.6))"

- effect-utility-selector at "/effects/glow": effect token and known control usages retained; arbitrary v1 utility-class targets have no fixed v2 slot and require explicit token references

    {"selector":".glow","effect":"0 0 8px var(--primary)"}

- effect-utility-selector at "/effects/textShadow": effect token and known control usages retained; arbitrary v1 utility-class targets have no fixed v2 slot and require explicit token references

    {"selector":".text-shadow","effect":"1px 1px 0px var(--border-dark)"}

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "system-ui, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    false

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"rgba(62, 70, 55, calc(var(--user-transparency, 0.9) * 0.8))","border":"1px solid var(--border-dark)"}

- font-availability at "/styles/interactive/container/fontFamily": family names retained; no font file is bundled or downloaded

    "system-ui, sans-serif"

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"2px solid var(--border-dark)"}

- unsupported-property at "/styles/interactive/header/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--bevel-inset)"}

- unsupported-property at "/styles/interactive/tasksSection/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--bevel-inset)"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid var(--border-dark)"}

- unsupported-property at "/styles/interactive/templateSection/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"var(--bevel-inset)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"var(--user-transparency, 0.3)","interactive":"0.95"}
