import type { EchosightApi, Theme } from '../../../shared/types';
import { generateThemeCSS } from './themeCss';
import {
  createThemeFontStyleElement,
  loadThemeAssets,
  loadThemeCss,
  removeThemeStyleElements
} from './themeRuntime';

type LogSink = Pick<Console, 'log' | 'warn' | 'error'>;

export interface ThemeApplyOptions {
  themeId: string;
  transparency: number;
  api: Pick<EchosightApi, 'getTheme' | 'getThemeAsset' | 'loadThemeCss'>;
  logger?: LogSink;
}

export async function applyRendererTheme(options: ThemeApplyOptions): Promise<Theme | null> {
  const logger = options.logger || console;
  const theme = await options.api.getTheme(options.themeId);
  if (!theme) {
    logger.error('No theme found for ID:', options.themeId);
    return null;
  }

  logger.log('Applying theme:', theme.name, 'with transparency:', options.transparency);
  removeThemeStyleElements();
  await loadThemeAssets(theme, options.api.getThemeAsset, logger);

  const cssFileStyles = await loadThemeCss(theme, options.api.loadThemeCss, logger);
  const css = generateThemeCSS(theme, options.transparency / 100, cssFileStyles);
  logger.log('Generated CSS with transparency:', css.substring(0, 500) + '...');

  appendStyleElement('theme-style', css);
  appendFontStyleElement(theme);
  document.documentElement.style.setProperty('--user-transparency', String(options.transparency / 100));

  return theme;
}

function appendStyleElement(id: string, css: string): void {
  const style = document.createElement('style');
  style.id = id;
  style.textContent = css;
  document.head.appendChild(style);
}

function appendFontStyleElement(theme: Theme): void {
  const style = createThemeFontStyleElement(theme);
  if (style) {
    document.head.appendChild(style);
  }
}
