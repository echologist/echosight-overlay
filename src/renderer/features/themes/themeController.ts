import type {
  EchosightApi,
  Theme
} from '../../../shared/types';
import {
  loadThemeState,
  openThemesDirectory,
  reloadThemeState
} from './themePersistence';
import {
  ignoreAlert,
  type AlertHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log' | 'error'>;

export interface ThemeControllerOptions {
  api: EchosightApi;
  alertUser?: AlertHandler;
  logger?: LogSink;
  afterReload?: () => Promise<void> | void;
}

export interface ThemeController {
  getThemes: () => Theme[];
  loadThemes: () => Promise<void>;
  openThemesFolder: () => Promise<void>;
  reloadThemes: () => Promise<void>;
}

export function createThemeController(options: ThemeControllerOptions): ThemeController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  let themes: Theme[] = [];

  function getThemes(): Theme[] {
    return themes;
  }

  async function loadThemes(): Promise<void> {
    themes = await loadThemeState(options.api, logger);
  }

  async function openThemesFolder(): Promise<void> {
    try {
      await openThemesDirectory(options.api, logger);
    } catch {
      await alertUser('Error opening themes folder. Check console for details.');
    }
  }

  async function reloadThemes(): Promise<void> {
    try {
      themes = await reloadThemeState(options.api, logger);
      await options.afterReload?.();
      await alertUser(`Themes reloaded! Found ${themes.length} themes.`);
    } catch (error) {
      logger.error('Error reloading themes:', error);
      await alertUser('Error reloading themes. Check console for details.');
    }
  }

  return {
    getThemes,
    loadThemes,
    openThemesFolder,
    reloadThemes
  };
}
