# Theme conversion report

Structural conversion only; not visual parity. Review every loss before publishing.

- css-source-order at "/customCSS": recognized selectors are applied in source order, not CSS specificity/importance; manual review required

    "/customCSS"

- unsupported-rule at "/customCSS/0": at-rule, nested rule or malformed CSS is not executed

    "@import url('https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap');"

- unsupported-selector at "/customCSS/1/body/color": selector is not in the fixed selector-to-slot table

    {"color":"#e8e2d2 !important"}

- unsupported-selector at "/customCSS/1/body/font-family": selector is not in the fixed selector-to-slot table

    {"font-family":"'Inter','Segoe UI',sans-serif !important"}

- font-availability at "/customCSS/11/.progress-text/font-family": family names retained; no font file is bundled or downloaded

    "'JetBrains Mono','Inter',monospace !important"

- unsupported-property at "/customCSS/11/.progress-text/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.06em !important"}

- unsupported-selector at "/customCSS/12/.task-section-label/border-bottom": selector is not in the fixed selector-to-slot table

    {"border-bottom":"1px dashed rgba(212,168,87,0.18) !important"}

- unsupported-selector at "/customCSS/12/.task-section-label/color": selector is not in the fixed selector-to-slot table

    {"color":"#d4a857 !important"}

- unsupported-selector at "/customCSS/12/.task-section-label/font-family": selector is not in the fixed selector-to-slot table

    {"font-family":"'Cinzel','Inter',serif !important"}

- unsupported-selector at "/customCSS/12/.task-section-label/font-size": selector is not in the fixed selector-to-slot table

    {"font-size":"10px !important"}

- unsupported-selector at "/customCSS/12/.task-section-label/letter-spacing": selector is not in the fixed selector-to-slot table

    {"letter-spacing":"0.18em !important"}

- unsupported-selector at "/customCSS/12/.task-section-label/text-transform": selector is not in the fixed selector-to-slot table

    {"text-transform":"uppercase !important"}

- unsupported-selector at "/customCSS/16/.task-delete/background": selector is not in the fixed selector-to-slot table

    {"background":"rgba(197,74,58,0.18) !important"}

- unsupported-selector at "/customCSS/16/.task-delete/border": selector is not in the fixed selector-to-slot table

    {"border":"1px solid rgba(197,74,58,0.35) !important"}

- unsupported-selector at "/customCSS/16/.task-delete/color": selector is not in the fixed selector-to-slot table

    {"color":"#f0a896 !important"}

- unsupported-selector at "/customCSS/17/.task-delete:hover/background": selector is not in the fixed selector-to-slot table

    {"background":"rgba(227,128,74,0.4) !important"}

- unsupported-selector at "/customCSS/17/.task-delete:hover/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(227,128,74,0.7) !important"}

- unsupported-selector at "/customCSS/17/.task-delete:hover/color": selector is not in the fixed selector-to-slot table

    {"color":"#ffe1d6 !important"}

- unsupported-property at "/customCSS/18/.modal/-webkit-backdrop-filter": CSS declaration not representable by this bounded converter

    {"-webkit-backdrop-filter":"blur(4px) !important"}

- unsupported-property at "/customCSS/18/.modal/backdrop-filter": CSS declaration not representable by this bounded converter

    {"backdrop-filter":"blur(4px) !important"}

- unsupported-property at "/customCSS/19/.modal-content/position": CSS declaration not representable by this bounded converter

    {"position":"relative !important"}

- font-availability at "/customCSS/2/.header h2/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',serif !important"

- unsupported-property at "/customCSS/2/.header h2/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.18em !important"}

- unsupported-selector at "/customCSS/20/.modal-content::before/background": selector is not in the fixed selector-to-slot table

    {"background":"linear-gradient(90deg, transparent, rgba(212,168,87,0.5), transparent)"}

- unsupported-selector at "/customCSS/20/.modal-content::before/content": selector is not in the fixed selector-to-slot table

    {"content":"''"}

- unsupported-selector at "/customCSS/20/.modal-content::before/height": selector is not in the fixed selector-to-slot table

    {"height":"1px"}

- unsupported-selector at "/customCSS/20/.modal-content::before/left": selector is not in the fixed selector-to-slot table

    {"left":"0"}

- unsupported-selector at "/customCSS/20/.modal-content::before/pointer-events": selector is not in the fixed selector-to-slot table

    {"pointer-events":"none"}

- unsupported-selector at "/customCSS/20/.modal-content::before/position": selector is not in the fixed selector-to-slot table

    {"position":"absolute"}

- unsupported-selector at "/customCSS/20/.modal-content::before/right": selector is not in the fixed selector-to-slot table

    {"right":"0"}

- unsupported-selector at "/customCSS/20/.modal-content::before/top": selector is not in the fixed selector-to-slot table

    {"top":"0"}

- font-availability at "/customCSS/21/.modal-content h4/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',serif !important"

- unsupported-property at "/customCSS/21/.modal-content h4/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.14em !important"}

- font-availability at "/customCSS/21/.modal-title/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',serif !important"

- unsupported-property at "/customCSS/21/.modal-title/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.14em !important"}

- font-availability at "/customCSS/21/.settings-title/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',serif !important"

- unsupported-property at "/customCSS/21/.settings-title/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.14em !important"}

- unsupported-selector at "/customCSS/25/.modal-content-dialog.app-dialog-tone-danger/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(197,74,58,0.75) !important"}

- unsupported-selector at "/customCSS/28/.context-menu div/color": selector is not in the fixed selector-to-slot table

    {"color":"#e8e2d2 !important"}

- unsupported-selector at "/customCSS/28/.context-menu div/font-family": selector is not in the fixed selector-to-slot table

    {"font-family":"'Inter',sans-serif !important"}

- unsupported-selector at "/customCSS/29/.context-menu div:hover/background": selector is not in the fixed selector-to-slot table

    {"background":"linear-gradient(90deg, rgba(212,168,87,0.18), rgba(212,168,87,0.06)) !important"}

- unsupported-selector at "/customCSS/29/.context-menu div:hover/color": selector is not in the fixed selector-to-slot table

    {"color":"#f1d990 !important"}

- unsupported-selector at "/customCSS/3/.header h2 #shortcut-hint/color": selector is not in the fixed selector-to-slot table

    {"color":"rgba(184,176,154,0.62) !important"}

- unsupported-selector at "/customCSS/3/.header h2 #shortcut-hint/font-family": selector is not in the fixed selector-to-slot table

    {"font-family":"'Inter',sans-serif !important"}

- unsupported-selector at "/customCSS/3/.header h2 #shortcut-hint/font-weight": selector is not in the fixed selector-to-slot table

    {"font-weight":"400 !important"}

- unsupported-selector at "/customCSS/3/.header h2 #shortcut-hint/letter-spacing": selector is not in the fixed selector-to-slot table

    {"letter-spacing":"0 !important"}

- unsupported-selector at "/customCSS/3/.header h2 #shortcut-hint/text-shadow": selector is not in the fixed selector-to-slot table

    {"text-shadow":"none !important"}

- unsupported-selector at "/customCSS/3/.header h2 #shortcut-hint/text-transform": selector is not in the fixed selector-to-slot table

    {"text-transform":"none !important"}

- font-availability at "/customCSS/4/.modal-btn.secondary/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',sans-serif !important"

- unsupported-property at "/customCSS/4/.modal-btn.secondary/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.06em !important"}

- unsupported-property at "/customCSS/4/.modal-btn.secondary/transition": CSS declaration not representable by this bounded converter

    {"transition":"all .2s ease !important"}

- font-availability at "/customCSS/4/.template-btn/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',sans-serif !important"

- unsupported-property at "/customCSS/4/.template-btn/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.06em !important"}

- unsupported-property at "/customCSS/4/.template-btn/transition": CSS declaration not representable by this bounded converter

    {"transition":"all .2s ease !important"}

- unsupported-property at "/customCSS/6/.add-btn/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"inset 0 1px 0 rgba(212,168,87,0.12) !important"}

- font-availability at "/customCSS/6/.add-btn/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',sans-serif !important"

- unsupported-property at "/customCSS/6/.add-btn/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.12em !important"}

- unsupported-property at "/customCSS/6/.add-btn/transition": CSS declaration not representable by this bounded converter

    {"transition":"all .2s ease !important"}

- unsupported-property at "/customCSS/6/.modal-btn.primary/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"inset 0 1px 0 rgba(212,168,87,0.12) !important"}

- font-availability at "/customCSS/6/.modal-btn.primary/font-family": family names retained; no font file is bundled or downloaded

    "'Cinzel','Inter',sans-serif !important"

- unsupported-property at "/customCSS/6/.modal-btn.primary/letter-spacing": CSS declaration not representable by this bounded converter

    {"letter-spacing":"0.12em !important"}

- unsupported-property at "/customCSS/6/.modal-btn.primary/transition": CSS declaration not representable by this bounded converter

    {"transition":"all .2s ease !important"}

- unsupported-property at "/customCSS/7/.add-btn:hover/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"inset 0 1px 0 rgba(212,168,87,0.2), 0 0 14px rgba(212,168,87,0.25) !important"}

- unsupported-property at "/customCSS/7/.modal-btn.primary:hover/box-shadow": CSS declaration not representable by this bounded converter

    {"box-shadow":"inset 0 1px 0 rgba(212,168,87,0.2), 0 0 14px rgba(212,168,87,0.25) !important"}

- unsupported-selector at "/customCSS/8/.modal-input/background": selector is not in the fixed selector-to-slot table

    {"background":"rgba(7,6,13,0.7) !important"}

- unsupported-selector at "/customCSS/8/.modal-input/border": selector is not in the fixed selector-to-slot table

    {"border":"1px solid rgba(212,168,87,0.18) !important"}

- unsupported-selector at "/customCSS/8/.modal-input/border-radius": selector is not in the fixed selector-to-slot table

    {"border-radius":"4px !important"}

- unsupported-selector at "/customCSS/8/.modal-input/color": selector is not in the fixed selector-to-slot table

    {"color":"#e8e2d2 !important"}

- unsupported-selector at "/customCSS/8/.modal-input/transition": selector is not in the fixed selector-to-slot table

    {"transition":"border-color .2s, box-shadow .2s !important"}

- unsupported-selector at "/customCSS/8/.task-input/background": selector is not in the fixed selector-to-slot table

    {"background":"rgba(7,6,13,0.7) !important"}

- unsupported-selector at "/customCSS/8/.task-input/border": selector is not in the fixed selector-to-slot table

    {"border":"1px solid rgba(212,168,87,0.18) !important"}

- unsupported-selector at "/customCSS/8/.task-input/border-radius": selector is not in the fixed selector-to-slot table

    {"border-radius":"4px !important"}

- unsupported-selector at "/customCSS/8/.task-input/color": selector is not in the fixed selector-to-slot table

    {"color":"#e8e2d2 !important"}

- unsupported-selector at "/customCSS/8/.task-input/transition": selector is not in the fixed selector-to-slot table

    {"transition":"border-color .2s, box-shadow .2s !important"}

- unsupported-selector at "/customCSS/8/.template-select/background": selector is not in the fixed selector-to-slot table

    {"background":"rgba(7,6,13,0.7) !important"}

- unsupported-selector at "/customCSS/8/.template-select/border": selector is not in the fixed selector-to-slot table

    {"border":"1px solid rgba(212,168,87,0.18) !important"}

- unsupported-selector at "/customCSS/8/.template-select/border-radius": selector is not in the fixed selector-to-slot table

    {"border-radius":"4px !important"}

- unsupported-selector at "/customCSS/8/.template-select/color": selector is not in the fixed selector-to-slot table

    {"color":"#e8e2d2 !important"}

- unsupported-selector at "/customCSS/8/.template-select/transition": selector is not in the fixed selector-to-slot table

    {"transition":"border-color .2s, box-shadow .2s !important"}

- unsupported-selector at "/customCSS/8/input[type='text']/background": selector is not in the fixed selector-to-slot table

    {"background":"rgba(7,6,13,0.7) !important"}

- unsupported-selector at "/customCSS/8/input[type='text']/border": selector is not in the fixed selector-to-slot table

    {"border":"1px solid rgba(212,168,87,0.18) !important"}

- unsupported-selector at "/customCSS/8/input[type='text']/border-radius": selector is not in the fixed selector-to-slot table

    {"border-radius":"4px !important"}

- unsupported-selector at "/customCSS/8/input[type='text']/color": selector is not in the fixed selector-to-slot table

    {"color":"#e8e2d2 !important"}

- unsupported-selector at "/customCSS/8/input[type='text']/transition": selector is not in the fixed selector-to-slot table

    {"transition":"border-color .2s, box-shadow .2s !important"}

- unsupported-property at "/customCSS/8/select/transition": CSS declaration not representable by this bounded converter

    {"transition":"border-color .2s, box-shadow .2s !important"}

- unsupported-property at "/customCSS/8/textarea/transition": CSS declaration not representable by this bounded converter

    {"transition":"border-color .2s, box-shadow .2s !important"}

- unsupported-selector at "/customCSS/9/.modal-input:focus/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(95,216,200,0.6) !important"}

- unsupported-selector at "/customCSS/9/.modal-input:focus/box-shadow": selector is not in the fixed selector-to-slot table

    {"box-shadow":"0 0 0 2px rgba(95,216,200,0.12) !important"}

- unsupported-selector at "/customCSS/9/.modal-input:focus/outline": selector is not in the fixed selector-to-slot table

    {"outline":"0 !important"}

- unsupported-selector at "/customCSS/9/.task-input:focus/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(95,216,200,0.6) !important"}

- unsupported-selector at "/customCSS/9/.task-input:focus/box-shadow": selector is not in the fixed selector-to-slot table

    {"box-shadow":"0 0 0 2px rgba(95,216,200,0.12) !important"}

- unsupported-selector at "/customCSS/9/.task-input:focus/outline": selector is not in the fixed selector-to-slot table

    {"outline":"0 !important"}

- unsupported-selector at "/customCSS/9/.template-select:focus/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(95,216,200,0.6) !important"}

- unsupported-selector at "/customCSS/9/.template-select:focus/box-shadow": selector is not in the fixed selector-to-slot table

    {"box-shadow":"0 0 0 2px rgba(95,216,200,0.12) !important"}

- unsupported-selector at "/customCSS/9/.template-select:focus/outline": selector is not in the fixed selector-to-slot table

    {"outline":"0 !important"}

- unsupported-selector at "/customCSS/9/input[type='text']:focus/border-color": selector is not in the fixed selector-to-slot table

    {"border-color":"rgba(95,216,200,0.6) !important"}

- unsupported-selector at "/customCSS/9/input[type='text']:focus/box-shadow": selector is not in the fixed selector-to-slot table

    {"box-shadow":"0 0 0 2px rgba(95,216,200,0.12) !important"}

- unsupported-selector at "/customCSS/9/input[type='text']:focus/outline": selector is not in the fixed selector-to-slot table

    {"outline":"0 !important"}

- unsupported-property at "/customCSS/9/select:focus/outline": CSS declaration not representable by this bounded converter

    {"outline":"0 !important"}

- unsupported-property at "/customCSS/9/textarea:focus/outline": CSS declaration not representable by this bounded converter

    {"outline":"0 !important"}

- unsupported-effect at "/effects/blur": inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed

    "14px"

- effect-utility-selector at "/effects/textShadow": effect token and known control usages retained; arbitrary v1 utility-class targets have no fixed v2 slot and require explicit token references

    {"selector":".text-shadow","effect":"0 1px 2px rgba(0,0,0,0.7)"}

- font-availability at "/fonts/primary/family": family names retained; no fonts bundled or fetched

    "'Inter', 'Segoe UI', Tahoma, sans-serif"

- unmapped-field at "/isDefault": v1 field is not interpreted by the v2 converter

    true

- unsupported-property at "/styles/clickThrough/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(10px) saturate(125%) brightness(0.85)"}

- unsupported-property at "/styles/clickThrough/container/borderRadius": CSS declaration not representable by this bounded converter

    {"borderRadius":"var(--border-radius)"}

- unsupported-property at "/styles/clickThrough/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(10px) saturate(125%) brightness(0.85)"}

- unsupported-mode-slot at "/styles/clickThrough/tasksSection": no mode-qualified section slot in v2; manual application decision required

    {"background":"transparent"}

- unsupported-property at "/styles/interactive/container/backdropFilter": CSS declaration not representable by this bounded converter

    {"backdropFilter":"blur(var(--blur)) saturate(140%)"}

- unsupported-property at "/styles/interactive/container/borderRadius": CSS declaration not representable by this bounded converter

    {"borderRadius":"var(--border-radius)"}

- unsupported-property at "/styles/interactive/container/overflow": CSS declaration not representable by this bounded converter

    {"overflow":"hidden"}

- unsupported-property at "/styles/interactive/container/position": CSS declaration not representable by this bounded converter

    {"position":"relative"}

- unsupported-property at "/styles/interactive/container/webkitBackdropFilter": CSS declaration not representable by this bounded converter

    {"webkitBackdropFilter":"blur(var(--blur)) saturate(140%)"}

- unsupported-property at "/styles/interactive/header/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid rgba(212,168,87,0.28)"}

- unsupported-property at "/styles/interactive/header/boxShadow": CSS declaration not representable by this bounded converter

    {"boxShadow":"inset 0 -1px 0 rgba(212,168,87,0.08)"}

- unsupported-property at "/styles/interactive/templateSection/borderBottom": CSS declaration not representable by this bounded converter

    {"borderBottom":"1px solid rgba(255,255,255,0.04)"}

- unmapped-field at "/transparency": v1 field is not interpreted by the v2 converter

    {"clickThrough":"var(--user-transparency, 0.7)","interactive":"var(--user-transparency, 0.7)"}
