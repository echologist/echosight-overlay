import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import {
  loadThemeAssetData,
  loadThemeCssFile
} from '../../../src/main/themes/themeAssetLoaders';
import type { Theme } from '../../../src/shared/types';

describe('theme asset loaders', () => {
  let tempRoot = '';
  let themeDir = '';

  beforeEach(async () => {
    tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'echosight-theme-assets-'));
    themeDir = path.join(tempRoot, 'theme');
    await fs.mkdir(themeDir, { recursive: true });
  });

  afterEach(async () => {
    if (tempRoot) {
      await fs.rm(tempRoot, { recursive: true, force: true });
    }
  });

  test('loads the declared cssFile and rewrites local asset urls', async () => {
    await fs.writeFile(
      path.join(themeDir, 'styles.css'),
      '.panel { background-image: url("./main_bg.svg"); }',
      'utf8'
    );

    await expect(loadThemeCssFile(createTheme({ cssFile: 'styles.css' }), 'styles.css'))
      .resolves.toBe('.panel { background-image: var(--asset-main-bg); }');
  });

  test('rejects undeclared sibling css files', async () => {
    await fs.writeFile(path.join(themeDir, 'styles.css'), '.declared {}', 'utf8');
    await fs.writeFile(path.join(themeDir, 'debug.css'), '.debug {}', 'utf8');

    await expect(loadThemeCssFile(createTheme({ cssFile: 'styles.css' }), 'debug.css'))
      .resolves.toBeNull();
  });

  test('rejects css path traversal and themes without declared css', async () => {
    await fs.writeFile(path.join(tempRoot, 'outside.css'), '.outside {}', 'utf8');

    await expect(loadThemeCssFile(createTheme({ cssFile: 'styles.css' }), '../outside.css'))
      .resolves.toBeNull();
    await expect(loadThemeCssFile(createTheme(), 'styles.css'))
      .resolves.toBeNull();
  });

  test('loads only assets contained inside the theme directory', async () => {
    const assetPath = path.join(themeDir, 'button.svg');
    await fs.writeFile(assetPath, '<svg />', 'utf8');
    await fs.writeFile(path.join(tempRoot, 'outside.svg'), '<svg />', 'utf8');

    const theme = createTheme({
      assets: {
        button: {
          path: assetPath,
          relativePath: 'themes/custom/button.svg',
          type: 'button',
          extension: '.svg'
        },
        outside: {
          path: path.join(tempRoot, 'outside.svg'),
          relativePath: '../outside.svg',
          type: 'misc',
          extension: '.svg'
        }
      }
    });

    await expect(loadThemeAssetData(theme, 'button')).resolves.toMatchObject({
      type: 'button',
      extension: '.svg',
      mimeType: 'image/svg+xml',
      isText: false
    });
    await expect(loadThemeAssetData(theme, 'outside')).resolves.toBeNull();
  });

  function createTheme(overrides: Partial<Theme> = {}): Theme {
    return {
      id: 'custom',
      name: 'Custom',
      folderPath: themeDir,
      path: path.join(themeDir, 'theme.json'),
      ...overrides
    };
  }
});
