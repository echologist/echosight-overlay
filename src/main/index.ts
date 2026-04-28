// Main Electron process for Task Overlay
import { app, BrowserWindow, ipcMain, shell } from 'electron';
import path from 'path';
import { registerStorageIpc } from './ipc/storageIpc';
import { registerThemeIpc } from './ipc/themeIpc';
import { registerWindowIpc } from './ipc/windowIpc';
import {
  createDataPaths,
  getDevelopmentBundledDataDir
} from './utils/dataPaths';
import { DEFAULT_HOTKEYS, normalizeHotkeys } from './utils/hotkeys';
import { readJsonFile } from './utils/jsonStorage';
import { createDefaultSettingsState } from '../shared/settingsDefaults';
import { getErrorMessage } from '../shared/errors';
import type { HotkeySettings, Settings } from '../shared/types';
import {
  hideOverlayWindow,
  isOverlayWindowReady,
  keepOverlayOnTop,
  setOverlayInteractive,
  showOverlayWindow
} from './overlay/windowActions';
import { createOverlayWindow as createOverlayBrowserWindow } from './overlay/createOverlayWindow';
import { installOverlayMenu } from './menu/overlayMenu';
import {
  isOverlayHotkeyRegistered,
  registerOverlayHotkeys,
  unregisterOverlayHotkeys
} from './hotkeys/globalHotkeys';
import { createPoe2Monitor } from './game/poe2Monitor';
import type { Poe2Monitor } from './game/poe2Monitor';
import { ThemeLibrary } from './themes/themeLibrary';
import { initializeRuntimeData } from './storage/runtimeDataBootstrap';

let overlayWindow: BrowserWindow | null = null;
let gameMonitor: Poe2Monitor | null = null;
let manuallyHidden = false;
let isInteractive = true; // Start interactive so inputs work
let currentHotkeys: HotkeySettings = { ...DEFAULT_HOTKEYS };

// Data storage paths - Use proper user data directory
const {
  DATA_DIR,
  THEMES_DIR,
  TASKS_FILE,
  TEMPLATES_FILE,
  SETTINGS_FILE
} = createDataPaths(app);

const DEFAULT_DATA_DIR = app.isPackaged
  ? path.join(process.resourcesPath, 'data')
  : getDevelopmentBundledDataDir(__dirname);
const DEFAULT_THEMES_DIR = path.join(DEFAULT_DATA_DIR, 'themes');

class PoE2TaskOverlay {
  private readonly themeLibrary: ThemeLibrary;

  constructor() {
    this.themeLibrary = new ThemeLibrary({
      dataDir: DATA_DIR,
      themesDir: THEMES_DIR,
      defaultThemesDir: DEFAULT_THEMES_DIR,
      isPackaged: app.isPackaged,
      platform: process.platform
    });
  }

  async initializeDataDirectory() {
    await initializeRuntimeData({
      dataDir: DATA_DIR,
      defaultDataDir: DEFAULT_DATA_DIR,
      settings: createDefaultSettingsState(DEFAULT_HOTKEYS)
    });
    await this.themeLibrary.initialize();
  }

  async initialize() {
    await app.whenReady();
    await this.initializeDataDirectory();
    await this.loadSettingsOnStartup();
    this.openOverlayWindow();
    this.setupIPC();
  }

  async loadSettingsOnStartup() {
    try {
      console.log('Loading settings on startup...');
      const settings = await readJsonFile<Partial<Settings>>(SETTINGS_FILE);
      console.log('Settings loaded on startup:', settings);
      
      if (isRecord(settings.hotkeys)) {
        console.log('Updating hotkeys from settings:', settings.hotkeys);
        currentHotkeys = normalizeHotkeys(settings.hotkeys, DEFAULT_HOTKEYS, console);
        console.log('Updated currentHotkeys:', currentHotkeys);
      }
    } catch (error) {
      console.log('No settings file found on startup, using defaults:', getErrorMessage(error));
    }
  }

  createOverlayWindow() {
    overlayWindow = createOverlayBrowserWindow({
      preloadPath: path.join(__dirname, '../preload/index.js'),
      rendererPath: path.join(__dirname, '../renderer/index.html'),
      fallbackRendererPath: path.join(__dirname, '../../dist-electron/renderer/index.html'),
      devRendererUrl: process.env['ELECTRON_RENDERER_URL'],
      isPackaged: app.isPackaged,
      registerHotkeys: () => this.registerHotkeys(),
      isToggleHotkeyRegistered: () => isOverlayHotkeyRegistered(currentHotkeys.toggleVisibility)
    });

    isInteractive = true;
  }

  openOverlayWindow() {
    this.createOverlayWindow();
    this.startGameMonitoring();
    this.setupMenu();
  }

  setupMenu() {
    installOverlayMenu({
      hotkeys: currentHotkeys,
      getWindow: getOverlayWindow,
      setManuallyHidden: (hidden) => {
        manuallyHidden = hidden;
      },
      toggleInteractiveMode: () => this.toggleInteractiveMode()
    });
  }

  registerHotkeys() {
    registerOverlayHotkeys({
      hotkeys: currentHotkeys,
      isPackaged: app.isPackaged,
      getWindow: getOverlayWindow,
      setManuallyHidden: (hidden) => {
        manuallyHidden = hidden;
      },
      toggleInteractiveMode: () => this.toggleInteractiveMode()
    });
  }

  toggleInteractiveMode() {
    const window = overlayWindow;
    if (!isOverlayWindowReady(window)) {
      return;
    }

    isInteractive = !isInteractive;
    console.log('Toggling interactive mode to:', isInteractive ? 'ON' : 'OFF');
    
    setOverlayInteractive(window, isInteractive);
  }

  startGameMonitoring() {
    gameMonitor?.stop();

    gameMonitor = createPoe2Monitor({
      onStarted: () => {
        const window = getOverlayWindow();
        if (!window || manuallyHidden) {
          return;
        }

        console.log('PoE2 detected - showing overlay in interactive mode');
        isInteractive = true;
        showOverlayWindow(window);
        setOverlayInteractive(window, true);
      },
      onStopped: () => {
        const window = getOverlayWindow();
        if (!window) {
          return;
        }

        console.log('PoE2 closed - hiding overlay');
        hideOverlayWindow(window);
        manuallyHidden = false;
        isInteractive = true;
      },
      onStillRunning: () => {
        const window = getOverlayWindow();
        if (window?.isVisible()) {
          keepOverlayOnTop(window);
        }
      }
    });

    gameMonitor.start();
  }

  setupIPC() {
    registerStorageIpc(ipcMain, {
      tasksFile: TASKS_FILE,
      templatesFile: TEMPLATES_FILE,
      settingsFile: SETTINGS_FILE
    });

    registerThemeIpc(ipcMain, {
      themesDir: THEMES_DIR,
      getThemes: () => this.themeLibrary.themes,
      reloadThemes: () => this.themeLibrary.reloadThemes(),
      openPath: (pathToOpen) => shell.openPath(pathToOpen)
    });

    registerWindowIpc(ipcMain, {
      getWindow: getOverlayWindow,
      getHotkeys: () => currentHotkeys,
      setHotkeys: (nextHotkeys) => {
        currentHotkeys = nextHotkeys;
        this.registerHotkeys();
        this.setupMenu();
      },
      setHotkeyRecording: (recording) => {
        if (recording) {
          console.log('Suspending global hotkeys while recording a new shortcut');
          unregisterOverlayHotkeys();
          return;
        }

        console.log('Restoring global hotkeys after shortcut recording');
        this.registerHotkeys();
      },
      setManuallyHidden: (hidden) => {
        manuallyHidden = hidden;
      },
      toggleInteractiveMode: () => this.toggleInteractiveMode(),
      quitApplication: () => {
        this.cleanup();
        app.quit();
      }
    });
  }

  cleanup() {
    gameMonitor?.stop();
    gameMonitor = null;
    unregisterOverlayHotkeys();
  }
}

function getOverlayWindow(): BrowserWindow | null {
  return isOverlayWindowReady(overlayWindow) ? overlayWindow : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

// Initialize overlay
const overlay = new PoE2TaskOverlay();

app.whenReady().then(() => {
  overlay.initialize();
});

app.on('window-all-closed', () => {
  overlay.cleanup();
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    overlay.openOverlayWindow();
  }
});

app.on('will-quit', () => {
  unregisterOverlayHotkeys();
});
