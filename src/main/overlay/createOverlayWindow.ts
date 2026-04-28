import { BrowserWindow, screen } from 'electron';
import { existsSync } from 'fs';
import {
  keepOverlayOnTop,
  showOverlayOnAllWorkspaces
} from './windowActions';
import { getOverlayWindowBounds } from './windowPlacement';

export interface CreateOverlayWindowOptions {
  preloadPath: string;
  rendererPath: string;
  fallbackRendererPath: string;
  devRendererUrl?: string;
  isPackaged: boolean;
  registerHotkeys: () => void;
  isToggleHotkeyRegistered: () => boolean;
}

export function createOverlayWindow(options: CreateOverlayWindowOptions): BrowserWindow {
  const primaryDisplay = screen.getPrimaryDisplay();
  const bounds = getOverlayWindowBounds(primaryDisplay.workArea);

  const window = new BrowserWindow({
    width: bounds.width,
    height: bounds.height,
    minWidth: 450,
    minHeight: 600,
    x: bounds.x,
    y: bounds.y,
    transparent: true,
    backgroundColor: '#00000000',
    frame: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    focusable: true,
    resizable: true,
    minimizable: false,
    maximizable: false,
    show: false,
    webPreferences: {
      preload: options.preloadPath,
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
      webSecurity: true
    }
  });

  keepOverlayOnTop(window);
  showOverlayOnAllWorkspaces(window);

  window.webContents.on('did-fail-load', (_event, errorCode, errorDescription, validatedURL) => {
    console.error('Failed to load renderer:', errorCode, errorDescription, validatedURL);

    console.log('Trying alternative path:', options.fallbackRendererPath);
    if (existsSync(options.fallbackRendererPath)) {
      window.loadFile(options.fallbackRendererPath);
    }
  });

  window.webContents.on('did-finish-load', () => {
    console.log('Renderer loaded successfully');

    window.webContents.executeJavaScript('typeof window !== "undefined"')
      .then((result: unknown) => {
        console.log('JavaScript working:', result);
      })
      .catch((error: unknown) => {
        console.error('JavaScript execution failed:', error);
      });

    console.log('Registering hotkeys after window load...');
    setTimeout(() => {
      options.registerHotkeys();

      setTimeout(() => {
        const isRegistered = options.isToggleHotkeyRegistered();
        console.log(`Hotkey registration check: ${isRegistered ? 'SUCCESS' : 'FAILED'}`);
      }, 1000);
    }, options.isPackaged ? 2000 : 500);
  });

  loadRenderer(window, options);

  window.setIgnoreMouseEvents(false);

  window.on('blur', () => {
    keepOverlayOnTop(window);
  });

  window.on('focus', () => {
    keepOverlayOnTop(window);
  });

  setTimeout(() => {
    if (!window.isDestroyed()) {
      window.show();
    }
  }, 1000);

  return window;
}

function loadRenderer(window: BrowserWindow, options: CreateOverlayWindowOptions): void {
  if (!options.isPackaged && options.devRendererUrl) {
    console.log('Loading dev URL:', options.devRendererUrl);
    window.loadURL(options.devRendererUrl);
    window.webContents.openDevTools({ mode: 'detach' });
    return;
  }

  console.log('Loading file:', options.rendererPath);
  console.log('File exists:', existsSync(options.rendererPath));
  window.loadFile(options.rendererPath);
}
