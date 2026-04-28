import path from 'path';
import { describe, expect, test } from 'vitest';
import {
  createDataPaths,
  getDevelopmentBundledDataDir
} from '../../../src/main/utils/dataPaths';

describe('main data paths', () => {
  test('resolves bundled development data from a compiled main directory', () => {
    const appDir = path.join('project', 'dist-electron', 'main');

    expect(getDevelopmentBundledDataDir(appDir)).toBe(path.resolve('project', 'data'));
  });

  test('resolves bundled development data from a source main directory', () => {
    const appDir = path.join('project', 'src', 'main');

    expect(getDevelopmentBundledDataDir(appDir)).toBe(path.resolve('project', 'data'));
  });

  test('keeps mutable runtime data inside Electron userData', () => {
    const paths = createDataPaths({
      getPath: () => path.join('user-data', 'Echosight')
    });

    expect(paths.DATA_DIR).toBe(path.join('user-data', 'Echosight', 'data'));
    expect(paths.THEMES_DIR).toBe(path.join(paths.DATA_DIR, 'themes'));
    expect(paths.TASKS_FILE).toBe(path.join(paths.DATA_DIR, 'current_tasks.json'));
    expect(paths.TEMPLATES_FILE).toBe(path.join(paths.DATA_DIR, 'templates.json'));
    expect(paths.SETTINGS_FILE).toBe(path.join(paths.DATA_DIR, 'settings.json'));
  });
});
