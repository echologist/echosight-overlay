import { describe, expect, test, vi } from 'vitest';
import type {
  EchosightApi,
  Settings
} from '../../../src/shared/types';
import { loadSettingsState } from '../../../src/renderer/features/settings/settingsPersistence';

describe('settings persistence', () => {
  test('persists normalized legacy settings after loading', async () => {
    const api = createSettingsApi({
      transparency: '64.2',
      backgroundColor: 'dark',
      hotkeys: {
        completeNextTask: 'Alt+N'
      }
    });

    const settings = await loadSettingsState(api, fallbackSettings, silentLogger);

    expect(settings).toEqual({
      settingsVersion: 1,
      transparency: 64,
      theme: 'dark',
      hotkeys: {
        toggleVisibility: 'Ctrl+Shift+T',
        toggleInteractive: 'Ctrl+Shift+I',
        completeNextTask: 'Alt+N',
        undoLastAction: 'Ctrl+Shift+Z',
        redoLastAction: 'Ctrl+Shift+Y'
      }
    });
    expect(api.saveSettings).toHaveBeenCalledWith(settings);
  });

  test('does not rewrite current settings on load', async () => {
    const api = createSettingsApi(fallbackSettings);

    await expect(loadSettingsState(api, fallbackSettings, silentLogger))
      .resolves.toEqual(fallbackSettings);
    expect(api.saveSettings).not.toHaveBeenCalled();
  });
});

const fallbackSettings: Settings = {
  settingsVersion: 1,
  transparency: 70,
  theme: 'echosight',
  hotkeys: {
    toggleVisibility: 'Ctrl+Shift+T',
    toggleInteractive: 'Ctrl+Shift+I',
    completeNextTask: 'Ctrl+Shift+N',
    undoLastAction: 'Ctrl+Shift+Z',
    redoLastAction: 'Ctrl+Shift+Y'
  }
};

function createSettingsApi(settings: unknown): EchosightApi {
  return {
    loadSettings: vi.fn(async () => clone(settings) as Partial<Settings> | null),
    saveSettings: vi.fn(async () => ({ success: true })),
    loadTasks: async () => ({ tasks: [], currentTemplate: null, snapshots: [] }),
    saveTasks: async () => ({ success: true }),
    loadTemplates: async () => [],
    saveTemplates: async () => ({ success: true }),
    loadThemes: async () => [],
    getTheme: async () => null,
    reloadThemes: async () => [],
    openThemesFolder: async () => ({ success: true }),
    getThemesPath: async () => '',
    loadThemeCss: async () => null,
    getThemeAsset: async () => null,
    updateHotkeys: () => undefined,
    setHotkeyRecording: () => undefined,
    focusWindow: () => undefined,
    resetWindowPosition: () => undefined,
    toggleOverlay: () => undefined,
    minimizeOverlay: () => undefined,
    toggleInteractiveMode: () => undefined,
    quitApplication: () => undefined,
    onInteractiveModeChanged: () => () => undefined,
    onCompleteNextTask: () => () => undefined,
    onUndoLastTaskAction: () => () => undefined,
    onRedoLastTaskAction: () => () => undefined
  };
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

const silentLogger = {
  log: () => undefined,
  error: () => undefined
};
