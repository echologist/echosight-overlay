import path from 'path';
import { defu } from 'defu';
import type { Theme, ThemeSoundConfig } from '../../shared/types';

const MIME_TYPES: Record<string, string> = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.mp3': 'audio/mpeg',
  '.ogg': 'audio/ogg',
  '.wav': 'audio/wav',
  '.m4a': 'audio/mp4'
};

const AUDIO_EXTENSIONS = new Set(['.mp3', '.ogg', '.wav', '.m4a']);
const IMAGE_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp']);

export function getSupportedThemeAssetExtensions(): string[] {
  return [...IMAGE_EXTENSIONS, ...AUDIO_EXTENSIONS];
}

export function isAudioAssetExtension(extension: string): boolean {
  return AUDIO_EXTENSIONS.has(extension.toLowerCase());
}

export function getAssetType(assetName: string, extension = ''): string {
  if (isAudioAssetExtension(extension)) {
    return 'sound';
  }

  const name = assetName.toLowerCase();

  if (name.includes('background') || name.includes('bg')) return 'background';
  if (name.includes('button')) return 'button';
  if (name.includes('progress')) return 'progress';
  if (name.includes('icon')) return 'icon';
  if (name.includes('texture')) return 'texture';
  if (name.includes('border')) return 'border';

  return 'misc';
}

export function getMimeType(extension: string): string {
  return MIME_TYPES[extension.toLowerCase()] || 'application/octet-stream';
}

export function getThemeDirectory(theme: Partial<Theme> | null | undefined): string | null {
  if (typeof theme?.folderPath === 'string' && theme.folderPath) {
    return theme.folderPath;
  }
  if (typeof theme?.path === 'string' && theme.path) {
    return path.dirname(theme.path);
  }
  return null;
}

export function createVariantTheme(
  baseTheme: Theme,
  variantId: string,
  variantConfig: Record<string, unknown>
): Theme {
  const variantTheme = structuredClone(baseTheme);

  variantTheme.id = `${baseTheme.id}-${variantId}`;
  variantTheme.name = getString(variantConfig.name, `${baseTheme.name} (${variantId})`);
  variantTheme.description = getString(
    variantConfig.description,
    `${baseTheme.description || baseTheme.name} - ${variantId} variant`
  );

  variantTheme.colors = mergeThemeSection(variantTheme.colors, variantConfig.colors);
  variantTheme.effects = mergeThemeSection(variantTheme.effects, variantConfig.effects);
  variantTheme.fonts = mergeThemeSection(variantTheme.fonts, variantConfig.fonts);
  variantTheme.styles = mergeThemeSection(variantTheme.styles, variantConfig.styles);
  variantTheme.components = mergeThemeSection(variantTheme.components, variantConfig.components);

  if (variantConfig.backgrounds) {
    variantTheme.backgrounds = mergeThemeSection(variantTheme.backgrounds, variantConfig.backgrounds);
  }
  if (variantConfig.layout) {
    variantTheme.layout = mergeThemeSection(variantTheme.layout, variantConfig.layout);
  }
  if (variantConfig.animations) {
    variantTheme.animations = mergeThemeSection(variantTheme.animations, variantConfig.animations);
  }
  if (variantConfig.customCSS) {
    variantTheme.customCSS = mergeThemeSection(variantTheme.customCSS, variantConfig.customCSS);
  }
  if (variantConfig.sounds) {
    variantTheme.sounds = mergeThemeSection(variantTheme.sounds, variantConfig.sounds) as ThemeSoundConfig;
  }

  delete variantTheme.variants;
  return variantTheme;
}

function mergeThemeSection(base: unknown, overrides: unknown): unknown {
  if (overrides === undefined || overrides === null) return base;
  if (base === undefined || base === null) return overrides;
  if (!isRecord(base) || !isRecord(overrides)) return overrides;

  return defu(overrides, base);
}

function getString(value: unknown, fallback: string): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
