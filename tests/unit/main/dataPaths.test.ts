import path from 'path';
import { describe, expect, test } from 'vitest';
import {
  createDataPaths,
  getDevelopmentDataDir
} from '../../../src/main/utils/dataPaths';

describe('main data paths', () => {
  test('resolves development data from a compiled main directory', () => {
    const appDir = path.join('project', 'dist-electron', 'main');

    expect(getDevelopmentDataDir(appDir)).toBe(path.resolve('project', 'data'));
  });

  test('resolves development data from a source main directory', () => {
    const appDir = path.join('project', 'src', 'main');

    expect(getDevelopmentDataDir(appDir)).toBe(path.resolve('project', 'data'));
  });

  test('keeps packaged data inside Electron userData', () => {
    const paths = createDataPaths({
      isPackaged: true,
      getPath: () => path.join('user-data', 'Echosight')
    }, path.join('project', 'dist-electron', 'main'));

    expect(paths.DATA_DIR).toBe(path.join('user-data', 'Echosight', 'data'));
    expect(paths.THEMES_DIR).toBe(path.join(paths.DATA_DIR, 'themes'));
  });
});
