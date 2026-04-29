import path from 'path';
import type { Theme, ThemeSoundEvent } from '../../shared/types';

export interface ThemeValidationResult {
  theme: Theme | null;
  errors: string[];
  warnings: string[];
}

export interface ThemeValidationMessages {
  errors: string[];
  warnings: string[];
}

const OPTIONAL_STRING_FIELDS = [
  'author',
  'description',
  'version'
];

const OPTIONAL_BOOLEAN_FIELDS = [
  'isDefault',
  'supportsTransparency'
];

const RECORD_FIELDS = [
  'fonts',
  'effects',
  'transparency',
  'styles',
  'components',
  'backgrounds',
  'layout',
  'animations',
  'compatibility',
  'metadata'
];

const VARIANT_RECORD_FIELDS = [
  'colors',
  'effects',
  'fonts',
  'styles',
  'components',
  'backgrounds',
  'layout',
  'animations'
];

const THEME_SOUND_EVENTS = new Set<ThemeSoundEvent>([
  'taskCompleted',
  'backgroundActivated',
  'undo',
  'redo'
]);
const SOUND_FILE_EXTENSIONS = new Set(['.mp3', '.ogg', '.wav', '.m4a']);

export function validateTheme(value: unknown): ThemeValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!isRecord(value)) {
    return {
      theme: null,
      errors: ['theme must be a JSON object'],
      warnings
    };
  }

  requireNonEmptyString(value, 'id', errors);
  requireNonEmptyString(value, 'name', errors);
  validateOptionalStringFields(value, OPTIONAL_STRING_FIELDS, errors);
  validateOptionalBooleanFields(value, OPTIONAL_BOOLEAN_FIELDS, errors);
  validateCssFile(value.cssFile, errors);
  validateCustomCss(value.customCSS, errors);
  validateSounds(value.sounds, errors, warnings);
  validateColors(value.colors, errors);
  validateRecordFields(value, RECORD_FIELDS, errors);
  validateVariants(value.variants, warnings);

  return {
    theme: errors.length === 0 ? value as Theme : null,
    errors,
    warnings
  };
}

export function validateThemeVariantConfig(
  themeId: string,
  variantId: string,
  value: unknown
): ThemeValidationMessages {
  const errors: string[] = [];
  const warnings: string[] = [];
  const variantPath = `variants.${variantId}`;

  if (!isRecord(value)) {
    return {
      errors: [`${variantPath} must be an object`],
      warnings
    };
  }

  validateOptionalStringFields(value, ['name', 'description'], errors, variantPath);
  validateCustomCss(value.customCSS, errors, variantPath);
  validateSounds(value.sounds, errors, warnings, variantPath);
  validateVariantRecordFields(value, VARIANT_RECORD_FIELDS, errors, variantPath);

  if (errors.length > 0) {
    warnings.push(`Skipping invalid variant ${themeId}/${variantId}`);
  }

  return { errors, warnings };
}

export function formatThemeValidationMessages(messages: string[]): string {
  return messages.join('; ');
}

function requireNonEmptyString(
  record: Record<string, unknown>,
  field: string,
  errors: string[]
): void {
  if (typeof record[field] !== 'string' || !record[field].trim()) {
    errors.push(`${field} must be a non-empty string`);
  }
}

function validateOptionalStringFields(
  record: Record<string, unknown>,
  fields: string[],
  errors: string[],
  prefix?: string
): void {
  fields.forEach(field => {
    const value = record[field];
    if (value !== undefined && typeof value !== 'string') {
      errors.push(formatPath(field, prefix) + ' must be a string');
    }
  });
}

function validateOptionalBooleanFields(
  record: Record<string, unknown>,
  fields: string[],
  errors: string[]
): void {
  fields.forEach(field => {
    const value = record[field];
    if (value !== undefined && typeof value !== 'boolean') {
      errors.push(`${field} must be a boolean`);
    }
  });
}

function validateCssFile(value: unknown, errors: string[]): void {
  if (value === undefined) {
    return;
  }

  if (typeof value !== 'string' || !value.trim()) {
    errors.push('cssFile must be a non-empty string');
    return;
  }

  if (hasUnsafePathSegment(value)) {
    errors.push('cssFile must be a relative path inside the theme folder');
    return;
  }

  if (path.extname(value).toLowerCase() !== '.css') {
    errors.push('cssFile must point to a .css file');
  }
}

function validateCustomCss(value: unknown, errors: string[], prefix?: string): void {
  if (value === undefined || typeof value === 'string') {
    return;
  }

  const customCssPath = formatPath('customCSS', prefix);
  if (!isRecord(value)) {
    errors.push(`${customCssPath} must be a string or object`);
    return;
  }

  Object.entries(value).forEach(([key, styles]) => {
    const fieldPath = formatPath(key, customCssPath);
    if (typeof styles === 'string') {
      return;
    }

    if (isRecord(styles)) {
      validateCssValueRecord(styles, errors, fieldPath);
      return;
    }

    errors.push(`${fieldPath} must be a CSS string or object`);
  });
}

function validateSounds(
  value: unknown,
  errors: string[],
  warnings: string[],
  prefix?: string
): void {
  if (value === undefined) {
    return;
  }

  const soundsPath = formatPath('sounds', prefix);
  if (!isRecord(value)) {
    errors.push(`${soundsPath} must be an object`);
    return;
  }

  if (value.enabled !== undefined && typeof value.enabled !== 'boolean') {
    errors.push(`${soundsPath}.enabled must be a boolean`);
  }

  validateVolume(value.volume, errors, `${soundsPath}.volume`);

  if (value.events === undefined) {
    return;
  }

  if (!isRecord(value.events)) {
    errors.push(`${soundsPath}.events must be an object`);
    return;
  }

  Object.entries(value.events).forEach(([eventName, definition]) => {
    const eventPath = `${soundsPath}.events.${eventName}`;
    if (!THEME_SOUND_EVENTS.has(eventName as ThemeSoundEvent)) {
      warnings.push(`${eventPath} is not a supported sound event; event will be ignored`);
      return;
    }

    validateSoundDefinition(definition, errors, eventPath);
  });
}

function validateSoundDefinition(value: unknown, errors: string[], prefix: string): void {
  if (typeof value === 'string') {
    validateSoundFile(value, errors, prefix);
    return;
  }

  if (!isRecord(value)) {
    errors.push(`${prefix} must be a sound file string or object`);
    return;
  }

  validateSoundFile(value.file, errors, `${prefix}.file`);
  validateVolume(value.volume, errors, `${prefix}.volume`);
}

function validateSoundFile(value: unknown, errors: string[], prefix: string): void {
  if (typeof value !== 'string' || !value.trim()) {
    errors.push(`${prefix} must be a non-empty sound file string`);
    return;
  }

  if (hasUnsafePathSegment(value)) {
    errors.push(`${prefix} must be a relative path inside the theme folder`);
    return;
  }

  if (hasPathSeparator(value)) {
    errors.push(`${prefix} must be a file name in the theme folder`);
    return;
  }

  if (!SOUND_FILE_EXTENSIONS.has(path.extname(value).toLowerCase())) {
    errors.push(`${prefix} must point to a supported audio file`);
  }
}

function validateVolume(value: unknown, errors: string[], prefix: string): void {
  if (value === undefined) {
    return;
  }

  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > 1) {
    errors.push(`${prefix} must be a number between 0 and 1`);
  }
}

function validateColors(value: unknown, errors: string[], prefix = 'colors'): void {
  if (value === undefined) {
    return;
  }

  if (!isRecord(value)) {
    errors.push(`${prefix} must be an object`);
    return;
  }

  validateCssValueRecord(value, errors, prefix);
}

function validateRecordFields(
  record: Record<string, unknown>,
  fields: string[],
  errors: string[]
): void {
  fields.forEach(field => {
    const value = record[field];
    if (value !== undefined && !isRecord(value)) {
      errors.push(`${field} must be an object`);
    }
  });
}

function validateVariantRecordFields(
  record: Record<string, unknown>,
  fields: string[],
  errors: string[],
  prefix: string
): void {
  fields.forEach(field => {
    const value = record[field];
    if (value === undefined) {
      return;
    }

    if (!isRecord(value)) {
      errors.push(formatPath(field, prefix) + ' must be an object');
      return;
    }

    if (field === 'colors') {
      validateCssValueRecord(value, errors, formatPath(field, prefix));
    }
  });
}

function validateVariants(value: unknown, warnings: string[]): void {
  if (value === undefined) {
    return;
  }

  if (!isRecord(value)) {
    warnings.push('variants must be an object; variants will be ignored');
    return;
  }

  Object.entries(value).forEach(([variantId, variantConfig]) => {
    if (!isRecord(variantConfig)) {
      warnings.push(`variants.${variantId} must be an object; variant will be ignored`);
    }
  });
}

function validateCssValueRecord(
  record: Record<string, unknown>,
  errors: string[],
  prefix: string
): void {
  Object.entries(record).forEach(([key, value]) => {
    const fieldPath = formatPath(key, prefix);
    if (isRecord(value)) {
      validateCssValueRecord(value, errors, fieldPath);
      return;
    }

    if (typeof value !== 'string' && typeof value !== 'number') {
      errors.push(`${fieldPath} must be a CSS string or number`);
    }
  });
}

function formatPath(field: string, prefix?: string): string {
  return prefix ? `${prefix}.${field}` : field;
}

function hasUnsafePathSegment(value: string): boolean {
  if (value.includes('\0') || path.isAbsolute(value) || path.win32.isAbsolute(value)) {
    return true;
  }

  const normalizedPath = value.replace(/\\/g, '/');
  return normalizedPath.split('/').some(segment => segment === '..');
}

function hasPathSeparator(value: string): boolean {
  return value.includes('/') || value.includes('\\');
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
