export function getPrimaryModifierLabel(): 'Cmd' | 'Ctrl' {
  return isApplePlatform() ? 'Cmd' : 'Ctrl';
}

export function formatHotkeyForDisplay(hotkey: string): string {
  const primaryModifier = getPrimaryModifierLabel();

  return hotkey
    .replace(/\bCommandOrControl\b/g, primaryModifier)
    .replace(/\bCommand\b/g, 'Cmd')
    .replace(/\bCmd\b/g, primaryModifier)
    .replace(/\bCtrl\b/g, primaryModifier);
}

function isApplePlatform(): boolean {
  return /mac|iphone|ipad|ipod/i.test(window.navigator.platform);
}
