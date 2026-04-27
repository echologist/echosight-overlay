import path from 'path';
import { promises as fs } from 'fs';
import { getErrorMessage } from '../../shared/errors';
import type { Theme, ThemeAsset } from '../../shared/types';
import { readJsonFile } from '../utils/jsonStorage';
import { resolveChildPath } from '../utils/pathSafety';
import {
  createVariantTheme,
  getAssetType
} from './themeHelpers';

export interface ThemeLibraryOptions {
  dataDir: string;
  themesDir: string;
  defaultThemesDir: string;
  isPackaged: boolean;
  platform: NodeJS.Platform;
  logger?: Pick<Console, 'log' | 'warn' | 'error'>;
}

export class ThemeLibrary {
  readonly themes = new Map<string, Theme>();

  private readonly dataDir: string;
  private readonly themesDir: string;
  private readonly defaultThemesDir: string;
  private readonly isPackaged: boolean;
  private readonly platform: NodeJS.Platform;
  private readonly logger: Pick<Console, 'log' | 'warn' | 'error'>;

  constructor(options: ThemeLibraryOptions) {
    this.dataDir = options.dataDir;
    this.themesDir = options.themesDir;
    this.defaultThemesDir = options.defaultThemesDir;
    this.isPackaged = options.isPackaged;
    this.platform = options.platform;
    this.logger = options.logger || console;
  }

  async initialize(): Promise<void> {
    try {
      this.logger.log('Creating data directory at:', this.dataDir);
      await fs.mkdir(this.dataDir, { recursive: true });
      await fs.mkdir(this.themesDir, { recursive: true });
      this.logger.log('Data directory created successfully');
      await this.copyDefaultThemes();
      await this.loadThemes();
    } catch (error) {
      this.logger.error('Failed to initialize theme library:', error);
    }
  }

  async reloadThemes(): Promise<void> {
    this.themes.clear();
    await this.loadThemes();
  }

  private async copyDefaultThemes(): Promise<void> {
    try {
      this.logger.log('Copying default themes from:', this.defaultThemesDir);
      this.logger.log('Platform:', this.platform, 'Packaged:', this.isPackaged);

      const themeEntries = await fs.readdir(this.defaultThemesDir, { withFileTypes: true });

      for (const entry of themeEntries) {
        const sourcePath = path.join(this.defaultThemesDir, entry.name);
        const targetPath = path.join(this.themesDir, entry.name);

        if (entry.isFile() && entry.name.endsWith('.json')) {
          await this.syncDefaultThemeFile(sourcePath, targetPath, entry.name);
        } else if (entry.isDirectory()) {
          await this.syncDefaultThemeFolder(sourcePath, targetPath, entry.name);
        }
      }
    } catch (error) {
      this.logger.error('Failed to copy default themes:', error);
      this.logger.error('Error details:', getErrorMessage(error));
    }
  }

  private async syncDefaultThemeFile(sourcePath: string, targetPath: string, themeName: string): Promise<void> {
    try {
      await fs.access(targetPath);
      if (await this.shouldUpdateBundledTheme(sourcePath, targetPath)) {
        this.logger.log(`Updating bundled default theme file: ${themeName}`);
        await fs.copyFile(sourcePath, targetPath);
      } else {
        this.logger.log(`Theme already exists: ${themeName}`);
      }
    } catch {
      this.logger.log(`Copying default theme file: ${themeName}`);
      await fs.copyFile(sourcePath, targetPath);
    }
  }

  private async syncDefaultThemeFolder(sourcePath: string, targetPath: string, themeName: string): Promise<void> {
    try {
      await fs.access(targetPath);
      if (await this.shouldUpdateBundledTheme(
        path.join(sourcePath, 'theme.json'),
        path.join(targetPath, 'theme.json')
      )) {
        this.logger.log(`Updating bundled default theme folder: ${themeName}`);
        await this.copyFolderRecursively(sourcePath, targetPath);
      } else {
        this.logger.log(`Theme already exists: ${themeName}`);
      }
    } catch {
      this.logger.log(`Copying default theme folder: ${themeName}`);
      await this.copyFolderRecursively(sourcePath, targetPath);
    }
  }

  private async shouldUpdateBundledTheme(sourceThemePath: string, targetThemePath: string): Promise<boolean> {
    try {
      const sourceTheme = await readJsonFile<Partial<Theme>>(sourceThemePath);
      const targetTheme = await readJsonFile<Partial<Theme>>(targetThemePath);
      if (!isTheme(sourceTheme) || !isTheme(targetTheme)) {
        return false;
      }

      if (sourceTheme.id !== targetTheme.id || sourceTheme.author !== targetTheme.author) {
        return false;
      }

      return compareThemeVersion(sourceTheme.version, targetTheme.version) > 0;
    } catch {
      return false;
    }
  }

  private async copyFolderRecursively(source: string, target: string): Promise<void> {
    try {
      await fs.mkdir(target, { recursive: true });
      const entries = await fs.readdir(source, { withFileTypes: true });

      for (const entry of entries) {
        const sourcePath = path.join(source, entry.name);
        const targetPath = path.join(target, entry.name);

        if (entry.isDirectory()) {
          await this.copyFolderRecursively(sourcePath, targetPath);
        } else {
          await fs.copyFile(sourcePath, targetPath);
        }
      }
    } catch (error) {
      this.logger.error(`Failed to copy folder ${source} to ${target}:`, error);
    }
  }

  private async loadThemes(): Promise<void> {
    try {
      this.logger.log('Loading themes from:', this.themesDir);
      const entries = await fs.readdir(this.themesDir, { withFileTypes: true });

      for (const entry of entries) {
        if (entry.isFile() && entry.name.endsWith('.json')) {
          await this.loadSingleFileTheme(entry.name);
        } else if (entry.isDirectory()) {
          await this.loadFolderTheme(entry.name);
        }
      }

      this.logger.log(`Total themes loaded: ${this.themes.size}`);
    } catch (error) {
      this.logger.error('Failed to load themes:', error);
    }
  }

  private async loadSingleFileTheme(filename: string): Promise<void> {
    try {
      const themePath = resolveChildPath(this.themesDir, filename);
      const theme = await readJsonFile<Partial<Theme>>(themePath);

      if (isTheme(theme)) {
        theme.assets = {};
        theme.path = themePath;
        this.themes.set(theme.id, theme);
        this.logger.log(`Loaded theme: ${theme.name} (${theme.id}) [single-file]`);
      } else {
        this.logger.warn(`Invalid theme file: ${filename} - missing id or name`);
      }
    } catch (error) {
      this.logger.error(`Failed to load theme file ${filename}:`, error);
    }
  }

  private async loadFolderTheme(folderName: string): Promise<void> {
    try {
      const themeFolderPath = resolveChildPath(this.themesDir, folderName);
      const themeJsonPath = resolveChildPath(themeFolderPath, 'theme.json');

      try {
        await fs.access(themeJsonPath);
      } catch {
        this.logger.warn(`No theme.json found in folder: ${folderName}`);
        return;
      }

      const theme = await readJsonFile<Partial<Theme>>(themeJsonPath);

      if (!isTheme(theme)) {
        this.logger.warn(`Invalid theme in folder: ${folderName} - missing id or name`);
        return;
      }

      theme.assets = await this.loadThemeAssets(themeFolderPath);
      theme.folderPath = themeFolderPath;
      theme.path = themeJsonPath;

      await this.loadThemeMetadata(theme, themeFolderPath);

      if (isRecord(theme.variants)) {
        await this.loadThemeVariants(theme, themeFolderPath);
      }

      this.themes.set(theme.id, theme);
      this.logger.log(
        `Loaded theme: ${theme.name} (${theme.id}) [folder] with ${Object.keys(theme.assets).length} assets`
      );
    } catch (error) {
      this.logger.error(`Failed to load theme folder ${folderName}:`, error);
    }
  }

  private async loadThemeAssets(themeFolderPath: string): Promise<Record<string, ThemeAsset>> {
    const assets: Record<string, ThemeAsset> = {};
    const supportedExtensions = ['.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp'];

    try {
      const files = await fs.readdir(themeFolderPath);

      for (const file of files) {
        if (file === 'theme.json') continue;

        const ext = path.extname(file).toLowerCase();
        if (supportedExtensions.includes(ext)) {
          const assetName = path.basename(file, ext);
          const assetPath = resolveChildPath(themeFolderPath, file);

          assets[assetName] = {
            path: assetPath,
            relativePath: `themes/${path.basename(themeFolderPath)}/${file}`,
            type: getAssetType(assetName),
            extension: ext
          };
        }
      }
    } catch (error) {
      this.logger.error('Failed to load theme assets:', error);
    }

    return assets;
  }

  private async loadThemeMetadata(theme: Theme, themeFolderPath: string): Promise<void> {
    try {
      const previewFiles = ['preview.png', 'preview.jpg', 'preview.jpeg', 'screenshot.png'];
      for (const previewFile of previewFiles) {
        const previewPath = resolveChildPath(themeFolderPath, previewFile);
        try {
          await fs.access(previewPath);
          theme.preview = {
            path: previewPath,
            relativePath: `themes/${path.basename(themeFolderPath)}/${previewFile}`
          };
          break;
        } catch {
          // Preview file doesn't exist, continue.
        }
      }

      const docFiles = ['README.md', 'readme.txt', 'info.txt'];
      for (const docFile of docFiles) {
        const docPath = resolveChildPath(themeFolderPath, docFile);
        try {
          const docContent = await fs.readFile(docPath, 'utf8');
          theme.documentation = {
            file: docFile,
            content: docContent
          };
          break;
        } catch {
          // Documentation file doesn't exist, continue.
        }
      }

      theme.metadata = {
        loadedAt: new Date().toISOString(),
        assetCount: Object.keys(theme.assets || {}).length,
        hasVariants: isRecord(theme.variants) && Object.keys(theme.variants).length > 0,
        hasPreview: !!theme.preview,
        hasDocumentation: !!theme.documentation,
        isAdvanced: !!(theme.animations || theme.layout || theme.backgrounds),
        compatibility: theme.compatibility || {
          minVersion: '1.0.0',
          maxVersion: '*'
        }
      };
    } catch (error) {
      this.logger.error('Failed to load theme metadata:', error);
    }
  }

  private async loadThemeVariants(theme: Theme, themeFolderPath: string): Promise<void> {
    try {
      const variants = isRecord(theme.variants) ? theme.variants : null;
      if (!variants) {
        return;
      }

      const variantThemes = new Map<string, Theme>();

      for (const [variantId, variantConfig] of Object.entries(variants)) {
        if (!isRecord(variantConfig)) {
          this.logger.warn(`Invalid variant config for ${theme.id}/${variantId}`);
          continue;
        }

        const variantTheme = createVariantTheme(theme, variantId, variantConfig);

        const variantAssetsPath = resolveChildPath(themeFolderPath, 'variants', variantId);
        try {
          await fs.access(variantAssetsPath);
          const variantAssets = await this.loadThemeAssets(variantAssetsPath);
          variantTheme.assets = { ...variantTheme.assets, ...variantAssets };
        } catch {
          // No variant-specific assets.
        }

        variantTheme.metadata = {
          ...(isRecord(theme.metadata) ? theme.metadata : {}),
          isVariant: true,
          baseTheme: theme.id,
          variantId
        };

        variantThemes.set(variantTheme.id, variantTheme);
      }

      for (const [variantId, variantTheme] of variantThemes) {
        this.themes.set(variantId, variantTheme);
        this.logger.log(`Loaded variant: ${variantTheme.name} (${variantTheme.id})`);
      }
    } catch (error) {
      this.logger.error('Failed to load theme variants:', error);
    }
  }
}

function isTheme(value: unknown): value is Theme {
  return isRecord(value) &&
    typeof value.id === 'string' &&
    value.id.length > 0 &&
    typeof value.name === 'string' &&
    value.name.length > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function compareThemeVersion(left: unknown, right: unknown): number {
  const leftVersion = parseThemeVersion(left);
  const rightVersion = parseThemeVersion(right);
  if (!leftVersion || !rightVersion) {
    return 0;
  }

  for (let index = 0; index < Math.max(leftVersion.length, rightVersion.length); index += 1) {
    const leftPart = leftVersion[index] || 0;
    const rightPart = rightVersion[index] || 0;
    if (leftPart !== rightPart) {
      return leftPart - rightPart;
    }
  }

  return 0;
}

function parseThemeVersion(value: unknown): number[] | null {
  if (typeof value !== 'string') {
    return null;
  }

  const parts = value.split('.').map(part => Number.parseInt(part, 10));
  return parts.length > 0 && parts.every(Number.isFinite) ? parts : null;
}
