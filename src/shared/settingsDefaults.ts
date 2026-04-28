import type {
  HotkeySettings,
  Settings
} from './types';

export const CURRENT_SETTINGS_VERSION = 1;

export function createDefaultSettingsState(hotkeys: HotkeySettings): Settings {
  return {
    settingsVersion: CURRENT_SETTINGS_VERSION,
    transparency: 70,
    theme: 'echosight',
    hotkeys: { ...hotkeys }
  };
}
