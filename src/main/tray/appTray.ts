import { Menu, Tray } from 'electron';
import path from 'path';

export interface AppTrayOptions {
  iconPath: string;
  onShow: () => void;
  onHide: () => void;
  onExit: () => void;
}

export interface TrayIconPathOptions {
  isPackaged: boolean;
  resourcesPath: string;
  mainDirectory: string;
}

export function createAppTray(options: AppTrayOptions): Tray {
  const tray = new Tray(options.iconPath);
  const contextMenu = Menu.buildFromTemplate([
    {
      label: 'Show Echosight',
      click: options.onShow
    },
    {
      label: 'Hide Echosight',
      click: options.onHide
    },
    { type: 'separator' },
    {
      label: 'Exit Echosight',
      click: options.onExit
    }
  ]);

  tray.setToolTip('Echosight');
  tray.setContextMenu(contextMenu);
  tray.on('click', options.onShow);

  return tray;
}

export function getTrayIconPath(options: TrayIconPathOptions): string {
  if (options.isPackaged) {
    return path.join(options.resourcesPath, 'tray-icon.ico');
  }

  return path.join(options.mainDirectory, '../../assets/tray-icon.ico');
}

export function shouldCreateAppTray(platform: NodeJS.Platform): boolean {
  return platform === 'win32';
}
