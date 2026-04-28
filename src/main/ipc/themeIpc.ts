import type { IpcMain } from 'electron';
import { getErrorMessage } from '../../shared/errors';
import type { Theme } from '../../shared/types';
import {
  loadThemeAssetData,
  loadThemeCssFile
} from '../themes/themeAssetLoaders';

export interface ThemeIpcOptions {
  themesDir: string;
  getThemes: () => Map<string, Theme>;
  reloadThemes: () => Promise<void>;
  openPath: (path: string) => Promise<string>;
}

type LogSink = Pick<Console, 'error'>;

export function registerThemeIpc(
  ipcMain: IpcMain,
  options: ThemeIpcOptions,
  logger: LogSink = console
): void {
  ipcMain.handle('load-themes', async () => {
    try {
      return Array.from(options.getThemes().values());
    } catch (error) {
      logger.error('Failed to get themes:', error);
      return [];
    }
  });

  ipcMain.handle('get-theme', async (_event, themeId) => {
    try {
      return typeof themeId === 'string'
        ? options.getThemes().get(themeId) || null
        : null;
    } catch (error) {
      logger.error('Failed to get theme:', error);
      return null;
    }
  });

  ipcMain.handle('reload-themes', async () => {
    try {
      await options.reloadThemes();
      return Array.from(options.getThemes().values());
    } catch (error) {
      logger.error('Failed to reload themes:', error);
      return [];
    }
  });

  ipcMain.handle('open-themes-folder', async () => {
    try {
      const errorMessage = await options.openPath(options.themesDir);
      if (errorMessage) {
        return { success: false, error: errorMessage };
      }

      return { success: true };
    } catch (error) {
      logger.error('Failed to open themes folder:', error);
      return { success: false, error: getErrorMessage(error) };
    }
  });

  ipcMain.handle('get-themes-path', async () => options.themesDir);

  ipcMain.handle('load-theme-css', async (_event, themeId, cssFileName) => {
    try {
      const theme = typeof themeId === 'string' ? options.getThemes().get(themeId) : null;
      return await loadThemeCssFile(theme, cssFileName);
    } catch (error) {
      logger.error(`Failed to load CSS file ${cssFileName} for theme ${themeId}:`, error);
      return null;
    }
  });

  ipcMain.handle('get-theme-asset', async (_event, themeId, assetName) => {
    try {
      const theme = typeof themeId === 'string' ? options.getThemes().get(themeId) : null;
      return await loadThemeAssetData(theme, assetName);
    } catch (error) {
      logger.error('Failed to get theme asset:', error);
      return null;
    }
  });
}
