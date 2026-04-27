import { screen } from 'electron';
import type { BrowserWindow, IpcMain } from 'electron';
import type { HotkeySettings } from '../../shared/types';
import { normalizeHotkeys } from '../utils/hotkeys';
import {
  focusOverlayWindow,
  hideOverlayWindow,
  showOverlayWindow,
  toggleOverlayWindow
} from '../overlay/windowActions';

export interface WindowIpcOptions {
  getWindow: () => BrowserWindow | null;
  getHotkeys: () => HotkeySettings;
  setHotkeys: (hotkeys: HotkeySettings) => void;
  setHotkeyRecording: (recording: boolean) => void;
  setManuallyHidden: (hidden: boolean) => void;
  toggleInteractiveMode: () => void;
  quitApplication: () => void;
  logger?: Pick<Console, 'log'>;
}

export function registerWindowIpc(ipcMain: IpcMain, options: WindowIpcOptions): void {
  const logger = options.logger || console;

  ipcMain.on('update-hotkeys', (_event, hotkeys) => {
    logger.log('Received hotkey update request:', hotkeys);

    const nextHotkeys = normalizeHotkeys(hotkeys, options.getHotkeys(), logger);

    logger.log('Converted hotkeys:', {
      visibility: nextHotkeys.toggleVisibility,
      interactive: nextHotkeys.toggleInteractive,
      completeNextTask: nextHotkeys.completeNextTask,
      undoLastAction: nextHotkeys.undoLastAction,
      redoLastAction: nextHotkeys.redoLastAction
    });

    if (!areHotkeysEqual(nextHotkeys, options.getHotkeys())) {
      options.setHotkeys(nextHotkeys);
    }
  });

  ipcMain.on('focus-window', () => {
    const window = options.getWindow();
    if (window) {
      focusOverlayWindow(window);
    }
  });

  ipcMain.on('set-hotkey-recording', (_event, recording) => {
    logger.log('IPC: set-hotkey-recording received:', recording);
    options.setHotkeyRecording(recording === true);
  });

  ipcMain.on('reset-window-position', () => {
    const window = options.getWindow();
    if (window) {
      const primaryDisplay = screen.getPrimaryDisplay();
      const { width } = primaryDisplay.workAreaSize;
      window.setPosition(width - 520, 20);
    }
  });

  ipcMain.on('toggle-overlay', () => {
    logger.log('IPC: toggle-overlay received');
    const window = options.getWindow();
    if (window) {
      const isVisible = toggleOverlayWindow(window);
      logger.log(isVisible ? 'IPC: Showing overlay' : 'IPC: Hiding overlay');
      options.setManuallyHidden(!isVisible);
    }
  });

  ipcMain.on('minimize-overlay', () => {
    logger.log('IPC: minimize-overlay received');
    const window = options.getWindow();
    if (window) {
      logger.log('IPC: Hiding overlay');
      hideOverlayWindow(window);
      options.setManuallyHidden(true);
    }
  });

  ipcMain.on('show-overlay', () => {
    logger.log('IPC: show-overlay received');
    const window = options.getWindow();
    if (window) {
      showOverlayWindow(window);
      options.setManuallyHidden(false);
    }
  });

  ipcMain.on('toggle-interactive-mode', () => {
    logger.log('IPC: toggle-interactive-mode received');
    const window = options.getWindow();
    if (window?.isVisible()) {
      options.toggleInteractiveMode();
    }
  });

  ipcMain.on('quit-application', () => {
    logger.log('IPC: quit-application received');
    options.quitApplication();
  });
}

function areHotkeysEqual(left: HotkeySettings, right: HotkeySettings): boolean {
  return left.toggleVisibility === right.toggleVisibility &&
    left.toggleInteractive === right.toggleInteractive &&
    left.completeNextTask === right.completeNextTask &&
    left.undoLastAction === right.undoLastAction &&
    left.redoLastAction === right.redoLastAction;
}
