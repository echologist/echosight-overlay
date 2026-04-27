import { describe, expect, test } from 'vitest';
import type { Theme } from '../../../src/shared/types';
import {
  createVariantTheme,
  getAssetType,
  getMimeType,
  getThemeDirectory
} from '../../../src/main/themes/themeHelpers';

describe('theme helpers', () => {
  test('creates defu-merged variant themes without mutating the base theme', () => {
    const baseTheme: Theme = {
      id: 'base',
      name: 'Base',
      description: 'Base theme',
      path: 'C:\\themes\\base.json',
      colors: {
        background: {
          primary: '#000',
          secondary: '#111'
        },
        text: {
          primary: '#fff'
        }
      },
      components: {
        button: {
          border: '1px solid #fff',
          colors: {
            text: '#fff'
          }
        }
      },
      variants: {
        ember: {}
      }
    };

    const variant = createVariantTheme(baseTheme, 'ember', {
      name: 'Ember',
      colors: {
        background: {
          primary: '#300'
        }
      },
      components: {
        button: {
          colors: {
            text: '#f90'
          }
        }
      }
    });

    expect(variant.id).toBe('base-ember');
    expect(variant.name).toBe('Ember');
    expect(variant.colors).toEqual({
      background: {
        primary: '#300',
        secondary: '#111'
      },
      text: {
        primary: '#fff'
      }
    });
    expect(variant.components).toEqual({
      button: {
        border: '1px solid #fff',
        colors: {
          text: '#f90'
        }
      }
    });
    expect(variant.variants).toBeUndefined();
    expect(baseTheme.colors).toEqual({
      background: {
        primary: '#000',
        secondary: '#111'
      },
      text: {
        primary: '#fff'
      }
    });
  });

  test('classifies theme assets and resolves theme directories', () => {
    expect(getAssetType('boss-background.webp')).toBe('background');
    expect(getAssetType('button-frame.png')).toBe('button');
    expect(getAssetType('unknown.bin')).toBe('misc');
    expect(getMimeType('.WEBP')).toBe('image/webp');
    expect(getMimeType('.bin')).toBe('application/octet-stream');
    expect(getThemeDirectory({ folderPath: '/themes/custom' })).toBe('/themes/custom');
    expect(getThemeDirectory({ path: '/themes/base/theme.json' })).toBe('/themes/base');
  });
});
