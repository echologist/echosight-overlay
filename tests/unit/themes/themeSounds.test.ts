import { describe, expect, test, vi } from 'vitest';
import type {
  EchosightApi,
  Theme,
  ThemeAssetData
} from '../../../src/shared/types';
import {
  createThemeSoundController,
  resolveThemeSoundDefinition,
  type AudioLike
} from '../../../src/renderer/features/themes/themeSounds';

describe('theme sounds', () => {
  test('resolves event sound definitions to asset names and event volume', () => {
    expect(resolveThemeSoundDefinition(themeFixture, 'taskCompleted')).toEqual({
      assetName: 'complete',
      volume: 1
    });
    expect(resolveThemeSoundDefinition(themeFixture, 'backgroundActivated')).toEqual({
      assetName: 'alert',
      volume: 0.5
    });
    expect(resolveThemeSoundDefinition(themeFixture, 'undo')).toBeNull();
  });

  test('plays enabled theme sounds with combined volume', async () => {
    const audio = createAudio();
    const api = createSoundApi({
      complete: {
        data: 'ZmFrZQ==',
        type: 'sound',
        extension: '.ogg',
        mimeType: 'audio/ogg',
        isText: false
      }
    });
    const controller = createThemeSoundController({
      api,
      createAudio: source => {
        expect(source).toBe('data:audio/ogg;base64,ZmFrZQ==');
        return audio;
      },
      logger: silentLogger
    });

    controller.applyTheme(themeFixture, {
      enabled: true,
      volume: 50
    });

    await controller.play('taskCompleted');
    await controller.play('taskCompleted');

    expect(api.getThemeAsset).toHaveBeenCalledOnce();
    expect(api.getThemeAsset).toHaveBeenCalledWith('sound-theme', 'complete');
    expect(audio.volume).toBe(0.4);
    expect(audio.play).toHaveBeenCalledTimes(2);
  });

  test('does nothing when global or theme sounds are disabled', async () => {
    const api = createSoundApi({});
    const controller = createThemeSoundController({
      api,
      createAudio,
      logger: silentLogger
    });

    controller.applyTheme(themeFixture, {
      enabled: false,
      volume: 50
    });
    await controller.play('taskCompleted');

    controller.applyTheme({
      ...themeFixture,
      sounds: {
        ...themeFixture.sounds,
        enabled: false
      }
    }, {
      enabled: true,
      volume: 50
    });
    await controller.play('taskCompleted');

    expect(api.getThemeAsset).not.toHaveBeenCalled();
  });
});

const themeFixture: Theme = {
  id: 'sound-theme',
  name: 'Sound Theme',
  sounds: {
    enabled: true,
    volume: 0.8,
    events: {
      taskCompleted: 'complete.ogg',
      backgroundActivated: {
        file: 'alert.wav',
        volume: 0.5
      }
    }
  }
};

function createSoundApi(assets: Record<string, ThemeAssetData>): Pick<EchosightApi, 'getThemeAsset'> {
  return {
    getThemeAsset: vi.fn(async (_themeId, assetName) => assets[assetName] || null)
  };
}

function createAudio(): AudioLike {
  return {
    currentTime: 0,
    volume: 1,
    play: vi.fn(async () => undefined)
  };
}

const silentLogger = {
  warn: () => undefined,
  error: () => undefined
};
