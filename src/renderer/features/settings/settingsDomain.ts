import type { HotkeySettings, Settings } from '../../../shared/types';
import {
  CURRENT_SETTINGS_VERSION,
  createDefaultSettingsState
} from '../../../shared/settingsDefaults';
import { getPrimaryModifierLabel } from '../../ui/hotkeyDisplay';

export type HotkeyAction = keyof HotkeySettings;

const DEFAULT_HOTKEYS: HotkeySettings = {
  toggleVisibility: `${getPrimaryModifierLabel()}+Shift+T`,
  toggleInteractive: `${getPrimaryModifierLabel()}+Shift+I`,
  completeNextTask: `${getPrimaryModifierLabel()}+Shift+N`,
  undoLastAction: `${getPrimaryModifierLabel()}+Shift+Z`,
  redoLastAction: `${getPrimaryModifierLabel()}+Shift+Y`
};

export function createDefaultHotkeys(): HotkeySettings {
  return { ...DEFAULT_HOTKEYS };
}

export function createDefaultSettings(): Settings {
  return createDefaultSettingsState(createDefaultHotkeys());
}

export function normalizeTransparency(value: unknown, fallback = 70): number {
  const numericValue = Number(value);
  if (!Number.isFinite(numericValue)) {
    return fallback;
  }

  return Math.min(100, Math.max(0, Math.round(numericValue)));
}

export function isHotkeyAction(value: unknown): value is HotkeyAction {
  return value === 'toggleVisibility' ||
    value === 'toggleInteractive' ||
    value === 'completeNextTask' ||
    value === 'undoLastAction' ||
    value === 'redoLastAction';
}

export interface HotkeyRecordingResult {
  hotkey: string | null;
  warning: string | null;
}

export function getHotkeyRecordingResult(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey' | 'shiftKey'>): HotkeyRecordingResult | null {
  const mainKey = normalizeHotkeyMainKey(event.key);
  if (!mainKey) {
    return null;
  }

  const primaryModifier = getPrimaryModifierLabel();
  const keys: string[] = [];
  if (event.ctrlKey || event.metaKey) keys.push(primaryModifier);
  if (event.altKey) keys.push('Alt');
  if (event.shiftKey) keys.push('Shift');
  keys.push(mainKey);

  const hasRequiredModifier = keys.includes(primaryModifier) ||
    keys.includes('Alt') ||
    isFunctionKey(mainKey);

  if (hasRequiredModifier) {
    return {
      hotkey: keys.join('+'),
      warning: null
    };
  }

  if (keys.length >= 2) {
    return {
      hotkey: null,
      warning: `Please use ${primaryModifier}+${mainKey}, Alt+${mainKey}, or a function key to avoid conflicts with normal typing.`
    };
  }

  return null;
}

export function normalizeSettings(input: unknown, fallback: Settings = createDefaultSettings()): Settings {
  const source = isRecord(input) ? input : {};
  const legacyTheme = typeof source.backgroundColor === 'string' ? source.backgroundColor.trim() : '';
  const theme = normalizeString(source.theme, legacyTheme || fallback.theme);

  return {
    settingsVersion: CURRENT_SETTINGS_VERSION,
    transparency: normalizeTransparency(source.transparency, fallback.transparency),
    theme,
    hotkeys: normalizeHotkeys(source.hotkeys, fallback.hotkeys)
  };
}

export function shouldPersistSettingsMigration(input: unknown, settings: Settings): boolean {
  if (!isRecord(input)) {
    return true;
  }

  if (input.settingsVersion !== CURRENT_SETTINGS_VERSION || 'backgroundColor' in input) {
    return true;
  }

  return input.transparency !== settings.transparency ||
    input.theme !== settings.theme ||
    !areHotkeysEqual(input.hotkeys, settings.hotkeys);
}

function normalizeHotkeys(input: unknown, fallback: HotkeySettings): HotkeySettings {
  const source = isRecord(input) ? input : {};

  return {
    toggleVisibility: normalizeString(source.toggleVisibility, fallback.toggleVisibility),
    toggleInteractive: normalizeString(source.toggleInteractive, fallback.toggleInteractive),
    completeNextTask: normalizeString(source.completeNextTask, fallback.completeNextTask),
    undoLastAction: normalizeString(source.undoLastAction, fallback.undoLastAction),
    redoLastAction: normalizeString(source.redoLastAction, fallback.redoLastAction)
  };
}

function areHotkeysEqual(input: unknown, hotkeys: HotkeySettings): boolean {
  if (!isRecord(input)) {
    return false;
  }

  return input.toggleVisibility === hotkeys.toggleVisibility &&
    input.toggleInteractive === hotkeys.toggleInteractive &&
    input.completeNextTask === hotkeys.completeNextTask &&
    input.undoLastAction === hotkeys.undoLastAction &&
    input.redoLastAction === hotkeys.redoLastAction;
}

function normalizeString(value: unknown, fallback: string): string {
  if (typeof value !== 'string') {
    return fallback;
  }

  const trimmedValue = value.trim();
  return trimmedValue || fallback;
}

function normalizeHotkeyMainKey(key: string): string | null {
  switch (key) {
    case 'Control':
    case 'Alt':
    case 'Shift':
    case 'Meta':
      return null;
    case ' ':
      return 'Space';
    case 'ArrowUp':
      return 'Up';
    case 'ArrowDown':
      return 'Down';
    case 'ArrowLeft':
      return 'Left';
    case 'ArrowRight':
      return 'Right';
    default:
      return key.length === 1 ? key.toUpperCase() : key;
  }
}

function isFunctionKey(key: string): boolean {
  return /^F(?:[1-9]|1[0-2])$/.test(key);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}
