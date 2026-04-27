import { readFile, readdir } from 'fs/promises';
import path from 'path';
import { describe, expect, test } from 'vitest';
import type { Theme } from '../../../src/shared/types';
import { generateThemeCSS } from '../../../src/renderer/features/themes/themeCss';

describe('theme modal styles', () => {
  test('loads hand-tuned modal styling for every bundled theme', async () => {
    const themes = await loadBundledThemes();

    expect(themes.length).toBeGreaterThan(0);
    themes.forEach(({ css, theme }) => {
      expect(css, theme.id).toContain('Theme Modal');
      expect(css, theme.id).toContain('.modal-content');
      expect(css, theme.id).toContain('.modal-btn');
    });
  });

  test('keeps Echosight dialog styling bespoke instead of generator-produced', async () => {
    const echosight = (await loadBundledThemes()).find(({ theme }) => theme.id === 'echosight');
    if (!echosight) {
      throw new Error('Missing Echosight theme fixture');
    }

    expect(echosight.css).toContain('/* ===== Theme Modal ===== */');
    expect(echosight.css).toContain("font-family: 'Cinzel','Inter',serif !important");
    expect(echosight.css).toContain('rgba(212,168,87,0.4)');
  });

  test('keeps minimal modals readable without a generated fallback', async () => {
    const minimal = (await loadBundledThemes()).find(({ theme }) => theme.id === 'minimal');
    if (!minimal) {
      throw new Error('Missing minimal theme fixture');
    }

    expect(minimal.css).toContain('background: rgba(0,0,0,0.9) !important');
    expect(minimal.css).toContain('border: 2px solid #fff !important');
  });
});

interface ThemeFixture {
  css: string;
  theme: Theme;
}

async function loadBundledThemes(): Promise<ThemeFixture[]> {
  const themesDir = path.join(process.cwd(), 'data', 'themes');
  const entries = await readdir(themesDir, { withFileTypes: true });
  const themes: ThemeFixture[] = [];

  for (const entry of entries) {
    const themePath = entry.isDirectory()
      ? path.join(themesDir, entry.name, 'theme.json')
      : path.join(themesDir, entry.name);

    if (!themePath.endsWith('.json')) {
      continue;
    }

    const theme = JSON.parse(await readFile(themePath, 'utf8')) as Theme;
    const cssFileStyles = typeof theme.cssFile === 'string'
      ? await readFile(path.join(path.dirname(themePath), theme.cssFile), 'utf8')
      : '';

    themes.push({
      css: generateThemeCSS(theme, 0.7, cssFileStyles),
      theme
    });
  }

  return themes;
}
