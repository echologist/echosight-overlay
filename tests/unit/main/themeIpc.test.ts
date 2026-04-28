import { describe, expect, test } from 'vitest';
import type { IpcMain } from 'electron';
import { registerThemeIpc } from '../../../src/main/ipc/themeIpc';

describe('theme IPC', () => {
  test('reports shell.openPath failures when opening the themes folder', async () => {
    const ipcMain = createIpcMain();
    registerThemeIpc(ipcMain as unknown as IpcMain, {
      themesDir: '/themes',
      getThemes: () => new Map(),
      reloadThemes: async () => undefined,
      openPath: async () => 'No application can open this folder'
    }, silentLogger);

    await expect(ipcMain.invoke('open-themes-folder')).resolves.toEqual({
      success: false,
      error: 'No application can open this folder'
    });
  });
});

function createIpcMain(): FakeIpcMain {
  const handlers = new Map<string, Handler>();
  return {
    handle: (channel, handler) => {
      handlers.set(channel, handler);
    },
    invoke: (channel, ...args) => {
      const handler = handlers.get(channel);
      if (!handler) {
        throw new Error(`Missing handler for ${channel}`);
      }

      return handler({}, ...args);
    }
  };
}

type Handler = (event: unknown, ...args: unknown[]) => unknown;

interface FakeIpcMain {
  handle: (channel: string, handler: Handler) => void;
  invoke: (channel: string, ...args: unknown[]) => unknown;
}

const silentLogger = {
  error: () => undefined
};
