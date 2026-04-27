import type { Theme } from '../../../shared/types';

export type ThemeRecord = Record<string, unknown>;
export type ThemeForCss = Theme & ThemeRecord;

export function isRecord(value: unknown): value is ThemeRecord {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function getRecord(value: unknown): ThemeRecord | null {
  return isRecord(value) ? value : null;
}

export function objectToCSS(obj: unknown): string {
  if (!isRecord(obj)) return '';

  return Object.entries(obj)
    .map(([key, value]) => `${toKebabCase(key)}: ${String(value)} !important`)
    .join(';\n      ');
}

export function toKebabCase(value: string): string {
  return value.replace(/([A-Z])/g, '-$1').toLowerCase();
}
