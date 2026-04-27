import type { EchosightApi, Theme } from '../../../shared/types';

type LogSink = Pick<Console, 'log' | 'error'>;

export async function loadThemeState(
  api: EchosightApi,
  logger: LogSink = console
): Promise<Theme[]> {
  try {
    const themes = await api.loadThemes();
    logger.log('Themes loaded:', themes.length);
    return themes;
  } catch (error) {
    logger.error('Failed to load themes:', error);
    return [];
  }
}

export async function reloadThemeState(
  api: EchosightApi,
  logger: LogSink = console
): Promise<Theme[]> {
  try {
    return await api.reloadThemes();
  } catch (error) {
    logger.error('Error reloading themes:', error);
    throw error;
  }
}

export async function openThemesDirectory(
  api: EchosightApi,
  logger: LogSink = console
): Promise<void> {
  try {
    const result = await api.openThemesFolder();
    if (!result.success) {
      throw new Error(result.error || 'Unknown open folder error');
    }

    logger.log('Themes folder opened');
  } catch (error) {
    logger.error('Error opening themes folder:', error);
    throw error;
  }
}
