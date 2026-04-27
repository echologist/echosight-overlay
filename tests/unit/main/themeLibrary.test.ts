import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { ThemeLibrary } from '../../../src/main/themes/themeLibrary';

describe('theme library', () => {
  let tempRoot = '';

  beforeEach(async () => {
    tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'echosight-themes-'));
  });

  afterEach(async () => {
    if (tempRoot) {
      await fs.rm(tempRoot, { recursive: true, force: true });
    }
  });

  test('updates older bundled default theme copies by version', async () => {
    const paths = await createThemePaths();
    await writeTheme(paths.defaultThemesDir, 'echosight.json', {
      id: 'echosight',
      name: 'Echosight',
      author: 'Echosight',
      version: '1.0.1',
      supportsTransparency: true
    });
    await writeTheme(paths.themesDir, 'echosight.json', {
      id: 'echosight',
      name: 'Echosight',
      author: 'Echosight',
      version: '1.0.0',
      supportsTransparency: false
    });

    await createLibrary(paths).initialize();

    const copiedTheme = JSON.parse(await fs.readFile(path.join(paths.themesDir, 'echosight.json'), 'utf8'));
    expect(copiedTheme.version).toBe('1.0.1');
    expect(copiedTheme.supportsTransparency).toBe(true);
  });

  test('keeps same-version bundled theme copies untouched', async () => {
    const paths = await createThemePaths();
    await writeTheme(paths.defaultThemesDir, 'echosight.json', {
      id: 'echosight',
      name: 'Echosight',
      author: 'Echosight',
      version: '1.0.1',
      supportsTransparency: true
    });
    await writeTheme(paths.themesDir, 'echosight.json', {
      id: 'echosight',
      name: 'Echosight',
      author: 'Echosight',
      version: '1.0.1',
      supportsTransparency: false
    });

    await createLibrary(paths).initialize();

    const copiedTheme = JSON.parse(await fs.readFile(path.join(paths.themesDir, 'echosight.json'), 'utf8'));
    expect(copiedTheme.supportsTransparency).toBe(false);
  });

  async function createThemePaths() {
    const dataDir = path.join(tempRoot, 'data');
    const themesDir = path.join(dataDir, 'themes');
    const defaultThemesDir = path.join(tempRoot, 'default-themes');
    await fs.mkdir(themesDir, { recursive: true });
    await fs.mkdir(defaultThemesDir, { recursive: true });

    return {
      dataDir,
      themesDir,
      defaultThemesDir
    };
  }
});

function createLibrary(paths: {
  dataDir: string;
  themesDir: string;
  defaultThemesDir: string;
}): ThemeLibrary {
  return new ThemeLibrary({
    ...paths,
    isPackaged: false,
    platform: process.platform,
    logger: silentLogger
  });
}

async function writeTheme(directory: string, filename: string, theme: unknown): Promise<void> {
  await fs.writeFile(path.join(directory, filename), JSON.stringify(theme, null, 2), 'utf8');
}

const silentLogger = {
  log: () => undefined,
  warn: () => undefined,
  error: () => undefined
};
