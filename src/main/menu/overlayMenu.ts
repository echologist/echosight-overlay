import { Menu } from 'electron';
import type { BrowserWindow, MenuItemConstructorOptions } from 'electron';
import type { HotkeySettings } from '../../shared/types';
import {
  completeNextOverlayTask,
  redoLastOverlayTaskAction,
  toggleOverlayWindow,
  undoLastOverlayTaskAction
} from '../overlay/windowActions';

export interface OverlayMenuOptions {
  hotkeys: HotkeySettings;
  getWindow: () => BrowserWindow | null;
  setManuallyHidden: (hidden: boolean) => void;
  toggleInteractiveMode: () => void;
}

export function installOverlayMenu(options: OverlayMenuOptions): void {
  const template: MenuItemConstructorOptions[] = [
    {
      label: 'File',
      submenu: [
        {
          label: 'Show/Hide Overlay',
          accelerator: options.hotkeys.toggleVisibility,
          click: () => {
            console.log('Menu shortcut clicked');
            const window = options.getWindow();
            if (!window) {
              return;
            }

            options.setManuallyHidden(!toggleOverlayWindow(window));
          }
        },
        {
          label: 'Toggle Interactive Mode',
          accelerator: options.hotkeys.toggleInteractive,
          click: () => {
            const window = options.getWindow();
            if (window?.isVisible()) {
              options.toggleInteractiveMode();
            }
          }
        },
        {
          label: 'Complete Next Task',
          accelerator: options.hotkeys.completeNextTask,
          click: () => {
            const window = options.getWindow();
            if (window?.isVisible()) {
              completeNextOverlayTask(window);
            }
          }
        },
        {
          label: 'Undo Last Task Action',
          accelerator: options.hotkeys.undoLastAction,
          click: () => {
            const window = options.getWindow();
            if (window?.isVisible()) {
              undoLastOverlayTaskAction(window);
            }
          }
        },
        {
          label: 'Forward Last Task Action',
          accelerator: options.hotkeys.redoLastAction,
          click: () => {
            const window = options.getWindow();
            if (window?.isVisible()) {
              redoLastOverlayTaskAction(window);
            }
          }
        },
        { type: 'separator' },
        { role: 'quit' }
      ]
    }
  ];

  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}
