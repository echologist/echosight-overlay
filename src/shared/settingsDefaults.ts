import type {
  HotkeySettings,
  Settings,
  SoundSettings
} from './types';

export const CURRENT_SETTINGS_VERSION = 2;

export function createDefaultSoundSettings(): SoundSettings {
  return {
    enabled: false,
    volume: 60
  };
}

export function createDefaultSettingsState(hotkeys: HotkeySettings): Settings {
  return {
    settingsVersion: CURRENT_SETTINGS_VERSION,
    transparency: 70,
    theme: 'echosight',
    hotkeys: { ...hotkeys },
    sounds: createDefaultSoundSettings()
  };
}
