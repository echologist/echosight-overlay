import { describe, expect, test, vi } from 'vitest';

describe('settings domain', () => {
  test('defaults to the Echosight theme', async () => {
    const {
      createDefaultSettings,
      normalizeSettings
    } = await importSettingsDomain('Win32');

    expect(createDefaultSettings().theme).toBe('echosight');
    expect(createDefaultSettings().transparency).toBe(70);
    expect(normalizeSettings(null).theme).toBe('echosight');
    expect(normalizeSettings(null).transparency).toBe(70);
    expect(normalizeSettings({ theme: '' }).theme).toBe('echosight');
  });

  test('normalizes legacy settings and fills undo/forward defaults', async () => {
    const { normalizeSettings } = await importSettingsDomain('Win32');

    const settings = normalizeSettings({
      transparency: '78.4',
      backgroundColor: 'echosight',
      hotkeys: {
        completeNextTask: 'Alt+N',
        undoLastAction: 'Alt+Z'
      }
    });

    expect(settings).toEqual({
      transparency: 78,
      theme: 'echosight',
      hotkeys: {
        toggleVisibility: 'Ctrl+Shift+T',
        toggleInteractive: 'Ctrl+Shift+I',
        completeNextTask: 'Alt+N',
        undoLastAction: 'Alt+Z',
        redoLastAction: 'Ctrl+Shift+Y'
      }
    });
  });

  test('uses Cmd labels for default hotkeys on macOS', async () => {
    const {
      createDefaultHotkeys,
      getHotkeyRecordingResult
    } = await importSettingsDomain('MacIntel');

    expect(createDefaultHotkeys().redoLastAction).toBe('Cmd+Shift+Y');
    expect(getHotkeyRecordingResult({
      key: 'y',
      ctrlKey: false,
      metaKey: true,
      altKey: false,
      shiftKey: true
    })).toEqual({
      hotkey: 'Cmd+Shift+Y',
      warning: null
    });
  });
});

async function importSettingsDomain(platform: string): Promise<typeof import('../../../src/renderer/features/settings/settingsDomain')> {
  vi.resetModules();
  Object.defineProperty(window.navigator, 'platform', {
    configurable: true,
    value: platform
  });

  return import('../../../src/renderer/features/settings/settingsDomain');
}
