import type { EchosightApi, Settings } from '../../../shared/types';
import { normalizeSettings } from './settingsDomain';

export async function loadSettingsState(
  api: EchosightApi,
  fallback: Settings,
  logger: Pick<Console, 'log' | 'error'> = console
): Promise<Settings> {
  try {
    logger.log('Loading settings...');
    const loadedSettings = await api.loadSettings();
    logger.log('Loaded settings:', loadedSettings);

    if (loadedSettings?.backgroundColor && !loadedSettings.theme) {
      logger.log('Migrating backgroundColor to theme:', loadedSettings.backgroundColor);
    }

    const settings = normalizeSettings(loadedSettings, fallback);
    logger.log('Final settings:', settings);
    return settings;
  } catch (error) {
    logger.error('Failed to load settings:', error);
    return fallback;
  }
}

export async function saveSettingsState(
  api: EchosightApi,
  settings: Settings,
  logger: Pick<Console, 'log'> = console
): Promise<void> {
  logger.log('Current settings:', settings);

  const saveResult = await api.saveSettings(settings);
  logger.log('Save result:', saveResult);

  if (!saveResult.success) {
    throw new Error(saveResult.error || 'Unknown save error');
  }

  logger.log('Updating hotkeys:', settings.hotkeys);
  api.updateHotkeys(settings.hotkeys);
}
