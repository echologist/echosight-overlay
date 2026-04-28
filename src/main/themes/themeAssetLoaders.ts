import { promises as fs } from 'fs';
import path from 'path';
import type { Theme, ThemeAssetData } from '../../shared/types';
import { isPathInside, resolveChildPath } from '../utils/pathSafety';
import {
  getMimeType,
  getThemeDirectory
} from './themeHelpers';

export async function loadThemeCssFile(
  theme: Theme | null | undefined,
  cssFileName: unknown
): Promise<string | null> {
  if (!theme || typeof cssFileName !== 'string' || typeof theme.cssFile !== 'string') {
    return null;
  }

  const declaredCssPath = normalizeThemeCssPath(theme.cssFile);
  const requestedCssPath = normalizeThemeCssPath(cssFileName);
  if (!declaredCssPath || requestedCssPath !== declaredCssPath) {
    return null;
  }

  const themeDir = getThemeDirectory(theme);
  if (!themeDir) {
    return null;
  }

  const cssPath = resolveChildPath(themeDir, declaredCssPath);
  if (path.extname(cssPath).toLowerCase() !== '.css') {
    return null;
  }

  const cssContent = await fs.readFile(cssPath, 'utf8');
  return cssContent.replace(/url\(['"]?\.\/([^'")\s]+)['"]?\)/g, (_match, filename: string) => {
    const assetName = filename.replace(/\.[^.]+$/, '').replace(/[^a-zA-Z0-9]/g, '-');
    return `var(--asset-${assetName})`;
  });
}

function normalizeThemeCssPath(value: string): string | null {
  const trimmedValue = value.trim();
  if (!trimmedValue || trimmedValue.includes('\0') || path.win32.isAbsolute(trimmedValue)) {
    return null;
  }

  const normalizedPath = path.posix.normalize(trimmedValue.replace(/\\/g, '/'));
  if (
    normalizedPath === '.' ||
    normalizedPath === '..' ||
    normalizedPath.startsWith('../') ||
    path.posix.isAbsolute(normalizedPath)
  ) {
    return null;
  }

  return path.extname(normalizedPath).toLowerCase() === '.css'
    ? normalizedPath
    : null;
}

export async function loadThemeAssetData(
  theme: Theme | null | undefined,
  assetName: unknown
): Promise<ThemeAssetData | null> {
  if (!theme || !theme.assets || typeof assetName !== 'string') {
    return null;
  }

  const asset = theme.assets[assetName];
  const themeDir = getThemeDirectory(theme);
  if (!asset || !themeDir || !asset.path || !isPathInside(themeDir, asset.path)) {
    return null;
  }

  if (asset.type === 'css') {
    const cssContent = await fs.readFile(asset.path, 'utf8');
    return {
      data: cssContent,
      type: asset.type,
      extension: asset.extension,
      mimeType: 'text/css',
      isText: true
    };
  }

  const assetData = await fs.readFile(asset.path);
  return {
    data: assetData.toString('base64'),
    type: asset.type,
    extension: asset.extension,
    mimeType: getMimeType(asset.extension),
    isText: false
  };
}
