import { describe, expect, test } from 'vitest';
import {
  convertHotkeyFormat,
  DEFAULT_HOTKEYS,
  normalizeHotkeys
} from '../../../src/main/utils/hotkeys';

describe('main hotkey utilities', () => {
  test('normalizes user-facing modifiers into Electron accelerators', () => {
    expect(convertHotkeyFormat('Ctrl+Shift+Y', undefined, silentLogger))
      .toBe('CommandOrControl+Shift+Y');
    expect(convertHotkeyFormat('Command+Alt+Z', undefined, silentLogger))
      .toBe('CommandOrControl+Alt+Z');
    expect(convertHotkeyFormat('Cmd+Control+Shift+T', undefined, silentLogger))
      .toBe('CommandOrControl+Shift+T');
  });

  test('fills missing hotkeys including undo and forward defaults', () => {
    expect(normalizeHotkeys({
      completeNextTask: 'Alt+N',
      undoLastAction: 'Alt+Z'
    }, DEFAULT_HOTKEYS, silentLogger)).toEqual({
      toggleVisibility: 'CommandOrControl+Shift+T',
      toggleInteractive: 'CommandOrControl+Shift+I',
      completeNextTask: 'Alt+N',
      undoLastAction: 'Alt+Z',
      redoLastAction: 'CommandOrControl+Shift+Y'
    });
  });
});

const silentLogger = {
  log: () => undefined
};
