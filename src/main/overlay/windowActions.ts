import type { BrowserWindow } from 'electron';

export function isOverlayWindowReady(window: BrowserWindow | null): window is BrowserWindow {
  return window !== null && !window.isDestroyed();
}

export function keepOverlayOnTop(window: BrowserWindow): void {
  if (!window.isDestroyed()) {
    setAlwaysOnTop(window, 'screen-saver');
  }
}

export function focusOverlayWindow(window: BrowserWindow): void {
  if (window.isDestroyed()) {
    return;
  }

  setWindowFocusable(window, true);
  setAlwaysOnTop(window);
  window.focus();
  window.webContents.focus();
}

export function showOverlayWindow(window: BrowserWindow): void {
  if (window.isDestroyed()) {
    return;
  }

  setWindowFocusable(window, true);
  window.show();
  setAlwaysOnTop(window, 'screen-saver');
  window.focus();
  window.webContents.focus();
}

export function hideOverlayWindow(window: BrowserWindow): void {
  if (!window.isDestroyed()) {
    window.hide();
  }
}

export function toggleOverlayWindow(window: BrowserWindow): boolean {
  if (window.isVisible()) {
    hideOverlayWindow(window);
    return false;
  }

  showOverlayWindow(window);
  return true;
}

export function setOverlayInteractive(window: BrowserWindow, interactive: boolean): void {
  if (window.isDestroyed()) {
    return;
  }

  window.setIgnoreMouseEvents(!interactive);
  window.webContents.send('interactive-mode-changed', interactive);
}

export function completeNextOverlayTask(window: BrowserWindow): void {
  if (!window.isDestroyed()) {
    window.webContents.send('complete-next-task');
  }
}

export function undoLastOverlayTaskAction(window: BrowserWindow): void {
  if (!window.isDestroyed()) {
    window.webContents.send('undo-last-task-action');
  }
}

export function redoLastOverlayTaskAction(window: BrowserWindow): void {
  if (!window.isDestroyed()) {
    window.webContents.send('redo-last-task-action');
  }
}

export function showOverlayOnAllWorkspaces(window: BrowserWindow): void {
  try {
    window.setVisibleOnAllWorkspaces(true);
  } catch (error) {
    console.error('Failed to show overlay on all workspaces:', error);
  }
}

function setAlwaysOnTop(window: BrowserWindow, level?: Parameters<BrowserWindow['setAlwaysOnTop']>[1]): void {
  try {
    if (level) {
      window.setAlwaysOnTop(true, level);
    } else {
      window.setAlwaysOnTop(true);
    }
  } catch (error) {
    console.error('Failed to set overlay always-on-top level:', error);

    try {
      window.setAlwaysOnTop(true);
    } catch (fallbackError) {
      console.error('Failed to set overlay always on top:', fallbackError);
    }
  }
}

function setWindowFocusable(window: BrowserWindow, focusable: boolean): void {
  try {
    window.setFocusable(focusable);
  } catch (error) {
    console.error('Failed to set overlay focusable state:', error);
  }
}
