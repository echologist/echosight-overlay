import { promises as fs } from 'fs';
import path from 'path';
import { describe, expect, test } from 'vitest';
import {
  validateTheme,
  validateThemeVariantConfig
} from '../../../src/main/themes/themeValidation';

describe('theme validation', () => {
  test('accepts a minimal valid theme', () => {
    const result = validateTheme({
      id: 'minimal',
      name: 'Minimal',
      colors: {
        primary: '#ffffff',
        background: {
          primary: 'rgba(0, 0, 0, 0.7)'
        }
      },
      supportsTransparency: true
    });

    expect(result.errors).toEqual([]);
    expect(result.theme?.id).toBe('minimal');
  });

  test('reports invalid required fields and known sections', () => {
    const result = validateTheme({
      id: '',
      name: 42,
      supportsTransparency: 'yes',
      colors: {
        primary: ['#fff']
      },
      styles: 'not-an-object',
      customCSS: []
    });

    expect(result.theme).toBeNull();
    expect(result.errors).toEqual([
      'id must be a non-empty string',
      'name must be a non-empty string',
      'supportsTransparency must be a boolean',
      'customCSS must be a string or object',
      'colors.primary must be a CSS string or number',
      'styles must be an object'
    ]);
  });

  test('rejects unsafe css file paths', () => {
    expect(validateTheme({
      id: 'unsafe',
      name: 'Unsafe',
      cssFile: '../outside.css'
    }).errors).toContain('cssFile must be a relative path inside the theme folder');

    expect(validateTheme({
      id: 'wrong-extension',
      name: 'Wrong Extension',
      cssFile: 'theme.txt'
    }).errors).toContain('cssFile must point to a .css file');
  });

  test('reports invalid variants without rejecting the base theme', () => {
    const result = validateTheme({
      id: 'base',
      name: 'Base',
      variants: {
        valid: {
          name: 'Valid'
        },
        broken: null
      }
    });

    expect(result.errors).toEqual([]);
    expect(result.theme?.id).toBe('base');
    expect(result.warnings).toEqual([
      'variants.broken must be an object; variant will be ignored'
    ]);
  });

  test('validates variant override shape before creating variant themes', () => {
    const result = validateThemeVariantConfig('base', 'corrupt', {
      name: 123,
      colors: {
        primary: false
      },
      customCSS: []
    });

    expect(result.errors).toEqual([
      'variants.corrupt.name must be a string',
      'variants.corrupt.customCSS must be a string or object',
      'variants.corrupt.colors.primary must be a CSS string or number'
    ]);
    expect(result.warnings).toEqual([
      'Skipping invalid variant base/corrupt'
    ]);
  });

  test('accepts all bundled themes', async () => {
    const themesDir = path.join(process.cwd(), 'data', 'themes');
    const entries = await fs.readdir(themesDir, { withFileTypes: true });

    for (const entry of entries) {
      const themePath = entry.isDirectory()
        ? path.join(themesDir, entry.name, 'theme.json')
        : path.join(themesDir, entry.name);

      if (!themePath.endsWith('.json')) {
        continue;
      }

      const theme = JSON.parse(await fs.readFile(themePath, 'utf8')) as unknown;
      expect(validateTheme(theme).errors, themePath).toEqual([]);
    }
  });
});
