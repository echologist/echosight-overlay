import type {
  EchosightApi,
  SoundSettings,
  Theme,
  ThemeSoundDefinition,
  ThemeSoundEvent
} from '../../../shared/types';

type ThemeSoundApi = Pick<EchosightApi, 'getThemeAsset'>;
type LogSink = Pick<Console, 'warn' | 'error'>;

export interface AudioLike {
  currentTime: number;
  volume: number;
  play: () => Promise<void> | void;
}

export interface ThemeSoundController {
  applyTheme: (theme: Theme, settings: SoundSettings) => void;
  clear: () => void;
  play: (event: ThemeSoundEvent) => Promise<void>;
  updateSettings: (settings: SoundSettings) => void;
}

export interface ThemeSoundControllerOptions {
  api: ThemeSoundApi;
  createAudio?: (source: string) => AudioLike;
  logger?: LogSink;
}

interface ResolvedSoundDefinition {
  assetName: string;
  volume: number;
}

export function createThemeSoundController(options: ThemeSoundControllerOptions): ThemeSoundController {
  const logger = options.logger || console;
  const createAudio = options.createAudio || ((source: string) => new Audio(source));
  const audioCache = new Map<ThemeSoundEvent, AudioLike>();
  let activeTheme: Theme | null = null;
  let settings: SoundSettings = {
    enabled: false,
    volume: 60
  };

  function clear(): void {
    audioCache.clear();
  }

  function applyTheme(theme: Theme, nextSettings: SoundSettings): void {
    activeTheme = theme;
    settings = nextSettings;
    clear();
  }

  function updateSettings(nextSettings: SoundSettings): void {
    settings = nextSettings;
  }

  async function play(event: ThemeSoundEvent): Promise<void> {
    if (!canPlaySounds(activeTheme, settings)) {
      return;
    }

    const definition = resolveThemeSoundDefinition(activeTheme, event);
    if (!definition) {
      return;
    }

    try {
      const audio = audioCache.get(event) || await loadThemeSound(activeTheme, event, definition);
      if (!audio) {
        return;
      }

      audioCache.set(event, audio);
      audio.volume = calculatePlaybackVolume(settings, activeTheme, definition);
      audio.currentTime = 0;
      await audio.play();
    } catch (error) {
      logger.warn(`Failed to play theme sound for ${event}:`, error);
    }
  }

  async function loadThemeSound(
    theme: Theme,
    event: ThemeSoundEvent,
    definition: ResolvedSoundDefinition
  ): Promise<AudioLike | null> {
    const asset = await options.api.getThemeAsset(theme.id, definition.assetName);
    if (!asset || !asset.mimeType.startsWith('audio/')) {
      logger.warn(`Theme sound ${event} references missing audio asset "${definition.assetName}"`);
      return null;
    }

    return createAudio(`data:${asset.mimeType};base64,${asset.data}`);
  }

  return {
    applyTheme,
    clear,
    play,
    updateSettings
  };
}

export function resolveThemeSoundDefinition(
  theme: Theme | null,
  event: ThemeSoundEvent
): ResolvedSoundDefinition | null {
  if (!theme?.sounds?.events) {
    return null;
  }

  const rawDefinition = theme.sounds.events[event];
  if (typeof rawDefinition === 'string') {
    return {
      assetName: getSoundAssetName(rawDefinition),
      volume: 1
    };
  }

  if (!isSoundObject(rawDefinition)) {
    return null;
  }

  return {
    assetName: getSoundAssetName(rawDefinition.file),
    volume: normalizeUnitVolume(rawDefinition.volume, 1)
  };
}

function canPlaySounds(theme: Theme | null, settings: SoundSettings): theme is Theme {
  return !!theme &&
    settings.enabled &&
    theme.sounds?.enabled !== false;
}

function calculatePlaybackVolume(
  settings: SoundSettings,
  theme: Theme,
  definition: ResolvedSoundDefinition
): number {
  return normalizeUnitVolume(settings.volume / 100, 0.6) *
    normalizeUnitVolume(theme.sounds?.volume, 1) *
    definition.volume;
}

function getSoundAssetName(file: string): string {
  const filename = file.split(/[\\/]/).pop() || file;
  return filename.replace(/\.[^.]+$/, '');
}

function normalizeUnitVolume(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.min(1, Math.max(0, value))
    : fallback;
}

function isSoundObject(value: ThemeSoundDefinition | undefined): value is Exclude<ThemeSoundDefinition, string> {
  return !!value &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    typeof value.file === 'string';
}
