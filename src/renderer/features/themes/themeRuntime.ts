import type { Theme, ThemeAssetData } from '../../../shared/types';

type ThemeAssetLoader = (themeId: string, assetName: string) => Promise<ThemeAssetData | null>;
type ThemeCssLoader = (themeId: string, cssFileName: string) => Promise<string | null>;
type LogSink = Pick<Console, 'log' | 'warn' | 'error'>;

const THEME_STYLE_IDS = [
  'theme-style',
  'background-style',
  'transparency-style',
  'font-style',
  'theme-assets'
];

export function removeThemeStyleElements(): void {
  THEME_STYLE_IDS.forEach(id => {
    document.getElementById(id)?.remove();
  });
}

export async function loadThemeAssets(
  theme: Theme,
  loadAsset: ThemeAssetLoader,
  logger: LogSink = console
): Promise<void> {
  if (!theme.assets || Object.keys(theme.assets).length === 0) {
    theme.loadedAssets = {};
    return;
  }

  logger.log('Loading theme assets for:', theme.name);
  theme.loadedAssets = {};

  for (const [assetName, assetInfo] of Object.entries(theme.assets)) {
    try {
      const assetData = await loadAsset(theme.id, assetName);
      if (!assetData) {
        continue;
      }

      if (assetData.type === 'css') {
        theme.loadedAssets[assetName] = assetData.data;
        logger.log(`Loaded CSS: ${assetName}`);
      } else {
        theme.loadedAssets[assetName] = `data:${assetData.mimeType};base64,${assetData.data}`;
        logger.log(`Loaded asset: ${assetName} (${assetInfo.type})`);
      }
    } catch (error) {
      logger.error(`Failed to load asset ${assetName}:`, error);
    }
  }
}

export async function loadThemeCss(
  theme: Theme,
  loadCss: ThemeCssLoader,
  logger: LogSink = console
): Promise<string> {
  if (typeof theme.cssFile !== 'string' || !theme.cssFile.trim()) {
    return '';
  }

  try {
    return await loadCss(theme.id, theme.cssFile) || '';
  } catch (error) {
    logger.warn(`Failed to load CSS file ${theme.cssFile}:`, error);
    return '';
  }
}

export function createThemeFontStyleElement(theme: Theme): HTMLStyleElement | null {
  const fontCSS = generateThemeFontCSS(theme);
  if (!fontCSS.trim()) {
    return null;
  }

  const style = document.createElement('style');
  style.id = 'font-style';
  style.textContent = fontCSS;
  return style;
}

export function generateThemeFontCSS(theme: Theme): string {
  const fonts = getRecord(theme.fonts);
  const primary = getRecord(fonts?.primary);
  if (!primary) {
    return '';
  }

  const sizes = getRecord(primary.sizes);
  const weights = getRecord(primary.weights);
  const family = normalizeCssValue(primary.family);
  const headerSize = normalizeCssValue(sizes?.header);
  const taskSize = normalizeCssValue(sizes?.task);
  const progressSize = normalizeCssValue(sizes?.progress);
  const uiSize = normalizeCssValue(sizes?.ui);
  const normalWeight = normalizeCssValue(weights?.normal);
  const boldWeight = normalizeCssValue(weights?.bold);

  let fontCSS = `
    body, .overlay-container {
      ${family ? `font-family: ${family} !important;` : ''}
    }

    .header h2 {
      ${headerSize ? `font-size: ${headerSize} !important;` : ''}
    }

    .task-text {
      ${taskSize ? `font-size: ${taskSize} !important;` : ''}
    }

    .progress-text {
      ${progressSize ? `font-size: ${progressSize} !important;` : ''}
    }

    button, select, input {
      ${uiSize ? `font-size: ${uiSize} !important;` : ''}
      ${family ? `font-family: ${family} !important;` : ''}
    }
  `;

  if (normalWeight || boldWeight) {
    fontCSS += `
      :root {
        ${normalWeight ? `--font-weight-normal: ${normalWeight};` : ''}
        ${boldWeight ? `--font-weight-bold: ${boldWeight};` : ''}
      }
    `;
  }

  return fontCSS;
}

function normalizeCssValue(value: unknown): string {
  return typeof value === 'string' || typeof value === 'number'
    ? String(value).trim()
    : '';
}

function getRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}
