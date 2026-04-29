import { describe, expect, test, vi } from 'vitest';
import { loadThemeAssets } from '../../../src/renderer/features/themes/themeRuntime';
import type { Theme } from '../../../src/shared/types';

describe('theme runtime assets', () => {
  test('loads visual assets for CSS and leaves sound assets lazy', async () => {
    const loadAsset = vi.fn(async (_themeId: string, assetName: string) => ({
      data: assetName === 'main_bg' ? 'image-data' : 'audio-data',
      type: assetName === 'main_bg' ? 'background' : 'sound',
      extension: assetName === 'main_bg' ? '.svg' : '.ogg',
      mimeType: assetName === 'main_bg' ? 'image/svg+xml' : 'audio/ogg',
      isText: false
    }));
    const theme: Theme = {
      id: 'sound-theme',
      name: 'Sound Theme',
      assets: {
        main_bg: {
          type: 'background',
          extension: '.svg'
        },
        complete: {
          type: 'sound',
          extension: '.ogg'
        }
      }
    };

    await loadThemeAssets(theme, loadAsset, silentLogger);

    expect(loadAsset).toHaveBeenCalledOnce();
    expect(loadAsset).toHaveBeenCalledWith('sound-theme', 'main_bg');
    expect(theme.loadedAssets).toEqual({
      main_bg: 'data:image/svg+xml;base64,image-data'
    });
  });
});

const silentLogger = {
  log: () => undefined,
  warn: () => undefined,
  error: () => undefined
};
