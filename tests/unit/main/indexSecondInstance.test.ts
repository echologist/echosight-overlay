import { beforeEach, describe, expect, test, vi } from 'vitest';

const mocks = vi.hoisted(() => {
  const listeners = new Map<string, () => unknown>();
  const overlayWindow = {};

  return {
    listeners,
    overlayWindow,
    app: {
      getPath: vi.fn(() => '/tmp/echosight-index-test'),
      isPackaged: false,
      on: vi.fn((event: string, listener: () => unknown) => {
        listeners.set(event, listener);
      }),
      quit: vi.fn(),
      requestSingleInstanceLock: vi.fn(() => true),
      whenReady: vi.fn(async () => undefined)
    },
    createOverlayWindow: vi.fn(() => overlayWindow),
    showOverlayWindow: vi.fn(),
    monitor: {
      start: vi.fn(),
      stop: vi.fn()
    }
  };
});

vi.mock('electron', () => ({
  app: mocks.app,
  BrowserWindow: {
    getAllWindows: vi.fn(() => [])
  },
  ipcMain: {},
  shell: {
    openPath: vi.fn()
  }
}));

vi.mock('../../../src/main/ipc/storageIpc', () => ({
  registerStorageIpc: vi.fn()
}));

vi.mock('../../../src/main/ipc/themeIpc', () => ({
  registerThemeIpc: vi.fn()
}));

vi.mock('../../../src/main/ipc/windowIpc', () => ({
  registerWindowIpc: vi.fn()
}));

vi.mock('../../../src/main/utils/jsonStorage', () => ({
  readJsonFile: vi.fn(async () => ({}))
}));

vi.mock('../../../src/main/overlay/windowActions', () => ({
  hideOverlayWindow: vi.fn(),
  isOverlayWindowReady: vi.fn((window: unknown) => window !== null),
  keepOverlayOnTop: vi.fn(),
  setOverlayInteractive: vi.fn(),
  showOverlayWindow: mocks.showOverlayWindow
}));

vi.mock('../../../src/main/overlay/createOverlayWindow', () => ({
  createOverlayWindow: mocks.createOverlayWindow
}));

vi.mock('../../../src/main/menu/overlayMenu', () => ({
  installOverlayMenu: vi.fn()
}));

vi.mock('../../../src/main/hotkeys/globalHotkeys', () => ({
  isOverlayHotkeyRegistered: vi.fn(() => true),
  registerOverlayHotkeys: vi.fn(),
  unregisterOverlayHotkeys: vi.fn()
}));

vi.mock('../../../src/main/game/poe2Monitor', () => ({
  createPoe2Monitor: vi.fn(() => mocks.monitor)
}));

vi.mock('../../../src/main/themes/themeLibrary', () => ({
  ThemeLibrary: class {
    themes = [];
    initialize = vi.fn(async () => undefined);
    reloadThemes = vi.fn(async () => []);
  }
}));

vi.mock('../../../src/main/storage/runtimeDataBootstrap', () => ({
  initializeRuntimeData: vi.fn(async () => undefined)
}));

describe('main second-instance listener', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    mocks.listeners.clear();
  });

  test('returns void and handles reveal failures inside its promise chain', async () => {
    const revealError = new Error('show failed');
    mocks.showOverlayWindow.mockImplementationOnce(() => {
      throw revealError;
    });
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await import('../../../src/main/index');
    await vi.waitFor(() => expect(mocks.createOverlayWindow).toHaveBeenCalledOnce());
    const listener = mocks.listeners.get('second-instance');
    expect(listener).toBeDefined();

    const result = listener?.();
    if (result instanceof Promise) {
      void result.catch(() => undefined);
    }

    expect(result).toBeUndefined();
    await vi.waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        'Failed to reveal Echosight for second instance:',
        revealError
      );
    });
  });
});
