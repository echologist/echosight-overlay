import type { App } from 'electron';
import path from 'path';

export interface DataPaths {
  DATA_DIR: string;
  THEMES_DIR: string;
  TASKS_FILE: string;
  TEMPLATES_FILE: string;
  SETTINGS_FILE: string;
}

export function createDataPaths(app: Pick<App, 'getPath'>): DataPaths {
  const dataDir = path.join(app.getPath('userData'), 'data');

  return {
    DATA_DIR: dataDir,
    THEMES_DIR: path.join(dataDir, 'themes'),
    TASKS_FILE: path.join(dataDir, 'current_tasks.json'),
    TEMPLATES_FILE: path.join(dataDir, 'templates.json'),
    SETTINGS_FILE: path.join(dataDir, 'settings.json')
  };
}

export function getDevelopmentBundledDataDir(appDir: string): string {
  return path.resolve(appDir, '..', '..', 'data');
}
