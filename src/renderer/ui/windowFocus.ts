export interface FocusOverlayApi {
  focusWindow: () => void;
}

export function focusOverlayNow(api?: FocusOverlayApi): void {
  try {
    window.focus?.();
  } catch {
    console.log('Could not focus window');
  }

  if (!api) {
    return;
  }

  try {
    api.focusWindow();
  } catch {
    console.log('Could not focus window via IPC');
  }
}

export function focusOverlaySoon(api: FocusOverlayApi, delayMs = 200): void {
  setTimeout(() => {
    focusOverlayNow(api);
  }, delayMs);
}
