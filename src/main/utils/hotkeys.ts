import type { HotkeySettings } from '../../shared/types';

export const DEFAULT_HOTKEYS: HotkeySettings = {
  toggleVisibility: 'CommandOrControl+Shift+T',
  toggleInteractive: 'CommandOrControl+Shift+I',
  completeNextTask: 'CommandOrControl+Shift+N',
  undoLastAction: 'CommandOrControl+Shift+Z',
  redoLastAction: 'CommandOrControl+Shift+Y'
};

type LogSink = Pick<Console, 'log'>;

export function normalizeHotkeys(
  hotkeys: unknown,
  fallback: HotkeySettings = DEFAULT_HOTKEYS,
  logger: LogSink = console
): HotkeySettings {
  const source = isRecord(hotkeys) ? hotkeys : {};

  return {
    toggleVisibility: convertHotkeyFormat(
      source.toggleVisibility,
      fallback.toggleVisibility,
      logger
    ),
    toggleInteractive: convertHotkeyFormat(
      source.toggleInteractive,
      fallback.toggleInteractive,
      logger
    ),
    completeNextTask: convertHotkeyFormat(
      source.completeNextTask,
      fallback.completeNextTask,
      logger
    ),
    undoLastAction: convertHotkeyFormat(
      source.undoLastAction,
      fallback.undoLastAction,
      logger
    ),
    redoLastAction: convertHotkeyFormat(
      source.redoLastAction,
      fallback.redoLastAction,
      logger
    )
  };
}

export function convertHotkeyFormat(
  userHotkey: unknown,
  fallback: string = DEFAULT_HOTKEYS.toggleVisibility,
  logger: LogSink = console
): string {
  if (typeof userHotkey !== 'string' || !userHotkey.trim()) {
    return fallback;
  }

  logger.log('Converting hotkey:', userHotkey);

  if (userHotkey.includes('CommandOrControl')) {
    logger.log('Already in Electron format:', userHotkey);
    return dedupePrimaryModifier(userHotkey);
  }

  const converted = userHotkey
    .replace(/\bCtrl\b/gi, 'CommandOrControl')
    .replace(/\bControl\b/gi, 'CommandOrControl')
    .replace(/\bCmd\b/gi, 'CommandOrControl')
    .replace(/\bCommand\b/gi, 'CommandOrControl');

  const normalized = dedupePrimaryModifier(converted);
  logger.log('Converted to:', normalized);
  return normalized;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function dedupePrimaryModifier(hotkey: string): string {
  let hasPrimaryModifier = false;
  return hotkey
    .split('+')
    .map(part => part.trim())
    .filter(Boolean)
    .filter(part => {
      if (part !== 'CommandOrControl') {
        return true;
      }

      if (hasPrimaryModifier) {
        return false;
      }

      hasPrimaryModifier = true;
      return true;
    })
    .join('+');
}
