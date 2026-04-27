import { globalShortcut } from 'electron';
import type { BrowserWindow } from 'electron';
import type { HotkeySettings } from '../../shared/types';
import {
  completeNextOverlayTask,
  redoLastOverlayTaskAction,
  toggleOverlayWindow,
  undoLastOverlayTaskAction
} from '../overlay/windowActions';

export interface OverlayHotkeyOptions {
  hotkeys: HotkeySettings;
  isPackaged: boolean;
  getWindow: () => BrowserWindow | null;
  setManuallyHidden: (hidden: boolean) => void;
  toggleInteractiveMode: () => void;
  logger?: Pick<Console, 'log' | 'error'>;
}

export function registerOverlayHotkeys(options: OverlayHotkeyOptions): void {
  const logger = options.logger || console;

  try {
    logger.log('=== REGISTERING HOTKEYS ===');
    logger.log('App packaged:', options.isPackaged);
    logger.log('Current hotkeys:', options.hotkeys);

    globalShortcut.unregisterAll();

    const toggleRet = globalShortcut.register(options.hotkeys.toggleVisibility, () => {
      logger.log('Toggle visibility shortcut triggered!');
      const window = options.getWindow();
      if (window) {
        const isVisible = toggleOverlayWindow(window);
        logger.log(isVisible ? 'Showing overlay' : 'Hiding overlay');
        options.setManuallyHidden(!isVisible);
      }
    });

    const interactiveRet = globalShortcut.register(options.hotkeys.toggleInteractive, () => {
      logger.log('Toggle interactive mode triggered!');
      const window = options.getWindow();
      if (window?.isVisible()) {
        options.toggleInteractiveMode();
      }
    });

    const nextTaskRet = globalShortcut.register(options.hotkeys.completeNextTask, () => {
      logger.log('Complete next task shortcut triggered!');
      const window = options.getWindow();
      if (window?.isVisible()) {
        completeNextOverlayTask(window);
      }
    });

    const undoRet = globalShortcut.register(options.hotkeys.undoLastAction, () => {
      logger.log('Undo last task action shortcut triggered!');
      const window = options.getWindow();
      if (window?.isVisible()) {
        undoLastOverlayTaskAction(window);
      }
    });

    const redoRet = globalShortcut.register(options.hotkeys.redoLastAction, () => {
      logger.log('Forward last task action shortcut triggered!');
      const window = options.getWindow();
      if (window?.isVisible()) {
        redoLastOverlayTaskAction(window);
      }
    });

    logger.log('Hotkey Registration Results:');
    logger.log(`  ${options.hotkeys.toggleVisibility}: ${toggleRet ? 'SUCCESS' : 'FAILED'}`);
    logger.log(`  ${options.hotkeys.toggleInteractive}: ${interactiveRet ? 'SUCCESS' : 'FAILED'}`);
    logger.log(`  ${options.hotkeys.completeNextTask}: ${nextTaskRet ? 'SUCCESS' : 'FAILED'}`);
    logger.log(`  ${options.hotkeys.undoLastAction}: ${undoRet ? 'SUCCESS' : 'FAILED'}`);
    logger.log(`  ${options.hotkeys.redoLastAction}: ${redoRet ? 'SUCCESS' : 'FAILED'}`);

    const successCount = [toggleRet, interactiveRet, nextTaskRet, undoRet, redoRet].filter(Boolean).length;
    logger.log(`Total: ${successCount}/5 hotkeys registered`);

    if (successCount === 0) {
      logger.error('No hotkeys registered successfully');
      throw new Error('Hotkey registration completely failed');
    }
  } catch (error) {
    logger.error('Error in registerHotkeys:', error);
  }
}

export function unregisterOverlayHotkeys(): void {
  globalShortcut.unregisterAll();
}

export function isOverlayHotkeyRegistered(hotkey: string): boolean {
  return globalShortcut.isRegistered(hotkey);
}
