// Oracle: execute unchanged production modules, with only DOM/platform/time supplied.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test, vi } from 'vitest';
import { DEFAULT_HOTKEYS, convertHotkeyFormat, normalizeHotkeys } from '../../src/main/utils/hotkeys';
import { createDefaultSettingsState } from '../../src/shared/settingsDefaults';
import type { HotkeySettings } from '../../src/shared/types';

const target = resolve('crates/echosight-core/tests/golden/settings/parity.json');
const sources = [
  'src/shared/settingsDefaults.ts', 'src/renderer/features/settings/settingsDomain.ts',
  'src/main/utils/hotkeys.ts', 'src/renderer/ui/hotkeyDisplay.ts',
  'src/renderer/features/settings/hotkeyRecorder.ts', 'src/renderer/features/settings/settingsUi.ts',
  'src/renderer/ui/modalUi.ts', 'src/renderer/ui/dialogTypes.ts',
  'tests/unit/settings/settingsDomain.test.ts', 'tests/unit/settings/hotkeyRecorder.test.ts',
  'tests/unit/settings/settingsUi.test.ts', 'tests/unit/main/hotkeys.test.ts',
  'tools/golden/settings.gen.test.ts', 'tools/golden/vitest.settings.config.ts'
];
const silentLogger = { log: () => undefined };
const actions = Object.keys(DEFAULT_HOTKEYS) as (keyof HotkeySettings)[];
const platforms = ['Win32', 'MacIntel', 'Linux x86_64', 'iPhone', 'iPad', 'iPod', 'Darwin', 'MAC'];
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
const key = (key: string, modifiers: object = {}) => ({ key, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...modifiers });
const values: unknown[] = [null, false, true, '', ' ', '\ufeff\u00a0', '\u0085', '78.4', '74.5',
  '0x1f', '0o17', '0b11', '+0x10', '-Infinity', 'Infinity', 'NaN', 'inf', '1e309', '.5', '5.', '1e',
  -1, -0.5, 0, 0.49999999999999994, 0.5, 1.5, 99.49, 99.5, 100, 500, [], [null], ['74.5'],
  [[['3.5']]], [true], [1, 2], {}, { volume: 40 }];
const hotkeyInputs: unknown[] = [null, 3, false, [], {}, '', '  ', '+', '++Ctrl++Shift++T++',
  'Ctrl+Shift+Y', 'Command+Alt+Z', 'Cmd+Control+Shift+T', 'Control+T', 'ctrl+CONTROL+cMd+cOMMand+T',
  'CommandOrControl+Ctrl+Cmd+Command+Control+T', 'CommandOrControl+CommandOrControl+T',
  'commandorcontrol+T', 'CTRL+T', ' Ctrl + Shift + T ', 'ControlPanel+Cmdlet+T',
  'éCtrlé+Ctrl_+_Ctrl+Ctrl1+1Ctrl', '⌘+T', 'Alt+F12', 'Shift+Shift+T', 'CommandOrControlSuffix+Ctrl+T',
  '\ufeffCtrl\u00a0+\u0085T', 'Ctrl-Alt-T', 'Ctrl+ +', 'Ctrl+ß', 'Ctrl+ArrowUp'];

test('generate settings and hotkey parity from real TypeScript modules', async () => {
  const fixture: any = {
    schemaVersion: 1,
    sourceSha256: Object.fromEntries(sources.map(path => [path, createHash('sha256').update(readFileSync(path)).digest('hex')])),
    bootstrapDefaults: createDefaultSettingsState(DEFAULT_HOTKEYS),
    defaults: [], settings: [], percentages: [], conversions: [], hotkeySets: [], displays: [], keys: [], recorders: []
  };
  for (const input of hotkeyInputs) {
    fixture.conversions.push({ input, expected: convertHotkeyFormat(input, undefined, silentLogger) });
    fixture.conversions.push({ input, fallback: 'RAW fallback', expected: convertHotkeyFormat(input, 'RAW fallback', silentLogger) });
  }
  for (const input of [null, [], {}, { completeNextTask: 'Alt+N', undoLastAction: 'Alt+Z' },
    { toggleVisibility: 42, toggleInteractive: ' Ctrl+I ', redoLastAction: 'Cmd+Control+Y', extra: 'kept?' }]) {
    fixture.hotkeySets.push({ input, expected: normalizeHotkeys(input, DEFAULT_HOTKEYS, silentLogger) });
  }
  for (const platform of platforms) {
    vi.resetModules();
    Object.defineProperty(window.navigator, 'platform', { configurable: true, value: platform });
    const domain = await import('../../src/renderer/features/settings/settingsDomain');
    const display = await import('../../src/renderer/ui/hotkeyDisplay');
    fixture.defaults.push({ platform, expected: domain.createDefaultSettings(), primary: display.getPrimaryModifierLabel() });
    for (const input of hotkeyInputs.filter(v => typeof v === 'string')) {
      fixture.displays.push({ platform, input, expected: display.formatHotkeyForDisplay(input as string) });
    }
    if (!['Win32', 'MacIntel'].includes(platform)) continue;
    const defaults = domain.createDefaultSettings();
    const custom = { ...defaults, theme: 'fallback-theme', transparency: 42, sounds: { enabled: true, volume: 23 }, hotkeys: { ...defaults.hotkeys, undoLastAction: 'F9' } };
    const inputs: unknown[] = [null, false, 42, 'hi', [], {},
      { transparency: '78.4', backgroundColor: 'echosight', hotkeys: { completeNextTask: 'Alt+N', undoLastAction: 'Alt+Z' } },
      { backgroundColor: '  dark\ufeff ', theme: '\u00a0' }, { theme: 'new', backgroundColor: 'old' },
      { theme: 3, backgroundColor: true }, { sounds: { enabled: true, volume: '74.5' } },
      { sounds: { enabled: 'yes', volume: 500 } },
      defaults, { ...defaults, settingsVersion: '2' }, { ...defaults, backgroundColor: null },
      { ...defaults, extra: 'drop', hotkeys: { ...defaults.hotkeys, extra: 'drop' }, sounds: { ...defaults.sounds, extra: 'drop' } },
      { hotkeys: { toggleVisibility: '  Ctrl+T  ', toggleInteractive: '', completeNextTask: null, undoLastAction: false, redoLastAction: 'Not an accelerator' } },
      { hotkeys: [], sounds: [] }, { theme: '\u0085', transparency: null, sounds: { enabled: false, volume: null } },
      ...values.map(value => ({ transparency: value, sounds: { enabled: value, volume: value } }))];
    for (const fallback of [undefined, custom]) for (const input of inputs) {
      const expected = domain.normalizeSettings(input, fallback);
      fixture.settings.push({ platform, input, ...(fallback ? { fallback } : {}), expected,
        changed: domain.shouldPersistSettingsMigration(input, expected) });
    }
    for (const input of [undefined, ...values]) {
      fixture.percentages.push({ ...(input === undefined ? {} : { input }), fallback: 37,
        expected: domain.normalizeTransparency(input, 37) });
    }
    const keys = ['Control', 'Alt', 'Shift', 'Meta', '', ' ', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight',
      'Escape', 'Enter', 'Tab', 'Backspace', 'Delete', 'Home', 'PageUp', 'a', 'z', '1', '+', '-', '=', ';', ',', '.', '/', '?', 'ß', 'é', 'ı', 'ſ', '😀',
      'F0', 'F1', 'F9', 'F10', 'F11', 'F12', 'F13', 'f1', 'Unidentified', 'Dead'];
    for (const main of keys) for (const modifiers of [{}, { shiftKey: true }, { ctrlKey: true }, { metaKey: true }, { altKey: true }, { ctrlKey: true, metaKey: true, altKey: true, shiftKey: true }]) {
      const event = key(main, modifiers);
      fixture.keys.push({ platform, event, expected: domain.getHotkeyRecordingResult(event) });
    }
    const { createHotkeyRecorder } = await import('../../src/renderer/features/settings/hotkeyRecorder');
    const sequences: { name: string; ops: any[]; hotkeys?: any; missing?: string }[] = [
      { name: 'success-delay', ops: [{ record: 'redoLastAction' }, { key: key('y', { ctrlKey: true, shiftKey: true }) }, { advance: 199 }, { advance: 1 }] },
      { name: 'click-cancel', ops: [{ record: 'redoLastAction' }, { record: 'redoLastAction' }, { stop: true }] },
      { name: 'switch', ops: [{ record: 'undoLastAction' }, { record: 'redoLastAction' }, { stop: true }] },
      { name: 'duplicate-and-function', ops: [{ record: 'redoLastAction' }, { key: key('z', { ctrlKey: true, shiftKey: true }) }, { key: key('F8') }, { advance: 200 }] },
      { name: 'ignored-warning-escape', ops: [{ key: key('F1') }, { record: 'bad' }, { record: 'redoLastAction' }, { key: key('Shift', { shiftKey: true }) }, { key: key('a') }, { key: key('a', { shiftKey: true }) }, { key: key('Escape') }, { key: key('Escape', { ctrlKey: true }) }, { stop: true }, { advance: 200 }] },
      { name: 'missing-control', missing: 'redoLastAction', ops: [{ record: 'undoLastAction' }, { record: 'redoLastAction' }] },
      { name: 'stale-timer-stops-new-recording', ops: [{ record: 'redoLastAction' }, { key: key('F8') }, { record: 'undoLastAction' }, { advance: 200 }] },
      { name: 'multiple-success-before-timeout', ops: [{ record: 'redoLastAction' }, { key: key('F8') }, { key: key('F9') }, { advance: 200 }] },
      { name: 'same-action-not-duplicate', ops: [{ record: 'undoLastAction' }, { key: key('z', { ctrlKey: true, shiftKey: true }) }, { advance: 200 }] },
      ...['CommandOrControl+Shift+Z', 'Command+Shift+Z', 'Cmd+Shift+Z', 'Ctrl+Shift+Z', 'Control+Shift+Z', 'CTRL+SHIFT+Z', ' Shift+Ctrl+Z ', ' Ctrl + Shift + Z ', 'Ctrl+Ctrl+Shift+Z'].map(assigned => ({
        name: `duplicate-${assigned}`, hotkeys: { ...defaults.hotkeys, undoLastAction: assigned },
        ops: [{ record: 'redoLastAction' }, { key: key('z', { ctrlKey: true, shiftKey: true }) }, { advance: 200 }]
      })),
      { name: 'unknown-first-match-blocks-known-duplicate', hotkeys: { unknown: 'F8', ...defaults.hotkeys, undoLastAction: 'F8' }, ops: [{ record: 'redoLastAction' }, { key: key('F8') }, { advance: 200 }] }
    ];
    for (const sequence of sequences) {
      vi.useFakeTimers(); vi.setSystemTime(0);
      const hotkeys = sequence.hotkeys ?? defaults.hotkeys;
      document.body.innerHTML = actions.filter(a => a !== sequence.missing).map(a => `<input id="${a}Hotkey"><button id="recordBtn${actions.indexOf(a) + 1}">Record</button>`).join('');
      for (const action of actions) {
        const input = document.getElementById(`${action}Hotkey`) as HTMLInputElement | null;
        if (input) input.value = display.formatHotkeyForDisplay(hotkeys[action]);
      }
      const recorded: any[] = [], alerts: string[] = [], recordingChanges: boolean[] = [];
      const recorder = createHotkeyRecorder({ getHotkeys: () => hotkeys, onRecorded: (action, hotkey) => recorded.push({ action, hotkey }),
        alertUser: message => { alerts.push(message); }, onRecordingChanged: state => recordingChanges.push(state) });
      const snapshots: any[] = [];
      for (const op of sequence.ops) {
        let consumed = false;
        if ('record' in op) recorder.record(op.record);
        if ('stop' in op) recorder.stop();
        if ('advance' in op) vi.advanceTimersByTime(op.advance);
        if ('key' in op) {
          const event = new KeyboardEvent('keydown', { ...op.key, cancelable: true });
          let propagationStopped = false;
          const originalStop = event.stopPropagation.bind(event);
          event.stopPropagation = () => { propagationStopped = true; originalStop(); };
          recorder.handleKeydown(event);
          expect(event.defaultPrevented).toBe(propagationStopped);
          consumed = event.defaultPrevented;
        }
        snapshots.push(clone({ active: recorder.isRecording(), consumed, recorded, alerts, recordingChanges,
          controls: Object.fromEntries(actions.filter(a => a !== sequence.missing).map(a => {
            const input = document.getElementById(`${a}Hotkey`) as HTMLInputElement;
            const button = document.getElementById(`recordBtn${actions.indexOf(a) + 1}`) as HTMLButtonElement;
            return [a, { value: input.value, recording: input.classList.contains('hotkey-recording-input'), button: button.textContent }];
          })) }));
      }
      fixture.recorders.push({ platform, ...sequence, hotkeys, snapshots });
      vi.clearAllTimers(); vi.useRealTimers(); document.body.replaceChildren();
    }
  }
  const bytes = JSON.stringify(fixture, null, 2) + '\n';
  if (process.env.SETTINGS_GOLDEN_CHECK === '1') {
    expect(readFileSync(target, 'utf8'), 'stale settings oracle or source provenance; regenerate intentionally').toBe(bytes);
  } else {
    mkdirSync(resolve(target, '..'), { recursive: true });
    writeFileSync(target, bytes);
  }
  console.log('Settings oracle cases:', Object.fromEntries(Object.entries(fixture).filter(([,v]) => Array.isArray(v)).map(([k,v]) => [k, (v as unknown[]).length])));
});
