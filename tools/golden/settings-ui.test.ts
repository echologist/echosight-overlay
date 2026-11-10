// C1: OBSERVATIONAL settings/hotkey oracle. Runs the UNCHANGED TypeScript source and records what it
// does. Nothing here is a stored golden output and nothing is a fake RED: expectations were written
// from the source and then executed against it.
//
// Real (unmocked) modules: settingsController, hotkeyRecorder, settingsUi, settingsPersistence,
// settingsDomain, ui/staticControls, ui/modalUi, ui/domEvents, ui/overlayUi, ui/overlayController,
// main/ipc/windowIpc, main/utils/hotkeys, main/overlay/windowActions, main/hotkeys/globalHotkeys and
// their transitive modules (see settings-ui.provenance.json).
//
// Stubs (external effects only): the `electron` dependency (globalShortcut/screen), a fake ipcMain, a
// fake BrowserWindow, EchosightApi spies, dialog handlers, fake timers, navigator.platform = Win32.
//
// HARNESS WIRING, NOT EXECUTED SOURCE: the `setHotkeys` / `setHotkeyRecording` callbacks handed to
// registerWindowIpc, and the initial registration, replicate what src/main/index.ts does at lines
// ~257-271 (setHotkeys => currentHotkeys=next; registerHotkeys(); setHotkeyRecording(true) =>
// unregisterOverlayHotkeys(); (false) => registerHotkeys()). main/index.ts is NOT imported or
// executed; this mirror is harness code and could drift from it.
//
// LIMITS: a stubbed globalShortcut proves only the source's call order/arguments and how it reacts to
// the stub's return values/throws. It proves NOTHING about real Electron/OS registration, duplicate or
// invalid accelerators, or key consumption by the OS.
import { afterEach, expect, test, vi } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Must run before the source modules are evaluated: settingsDomain computes DEFAULT_HOTKEYS at import.
vi.hoisted(() => {
  Object.defineProperty(window.navigator, 'platform', { value: 'Win32', configurable: true });
});

const stub = vi.hoisted(() => {
  const s = {
    sink: [] as string[],
    callbacks: new Map<string, () => void>(),
    registerResult: (_accelerator: string): boolean => true,
    registerThrows: (_accelerator: string): boolean => false,
    unregisterAll(): void { s.sink.push('shortcut.unregisterAll'); s.callbacks.clear(); },
    register(accelerator: string, callback: () => void): boolean {
      s.sink.push(`shortcut.register(${accelerator})`);
      if (s.registerThrows(accelerator)) throw new Error(`stub register threw for ${accelerator}`);
      const ok = s.registerResult(accelerator);
      if (ok) s.callbacks.set(accelerator, callback);
      return ok;
    },
    isRegistered(accelerator: string): boolean { return s.callbacks.has(accelerator); }
  };
  return s;
});

vi.mock('electron', () => ({
  globalShortcut: {
    unregisterAll: () => stub.unregisterAll(),
    register: (accelerator: string, callback: () => void) => stub.register(accelerator, callback),
    isRegistered: (accelerator: string) => stub.isRegistered(accelerator)
  },
  screen: { getPrimaryDisplay: () => ({ workArea: { x: 0, y: 0, width: 1920, height: 1080 } }) }
}));

import { createSettingsController } from '../../src/renderer/features/settings/settingsController';
import { setupRendererControls } from '../../src/renderer/ui/staticControls';
import { hideModal } from '../../src/renderer/ui/modalUi';
import { createOverlayController } from '../../src/renderer/ui/overlayController';
import { registerWindowIpc } from '../../src/main/ipc/windowIpc';
import { registerOverlayHotkeys, unregisterOverlayHotkeys } from '../../src/main/hotkeys/globalHotkeys';
import { DEFAULT_HOTKEYS, normalizeHotkeys } from '../../src/main/utils/hotkeys';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const BASE_SHA = '93d2833dbed5d27b32be959a0d1194695c381c85';
const PLACEHOLDER = 'Press keys now...';
const DISPLAY_DEFAULTS = ['Ctrl+Shift+T', 'Ctrl+Shift+I', 'Ctrl+Shift+N', 'Ctrl+Shift+Z', 'Ctrl+Shift+Y'];
const MAIN_DEFAULTS = [
  'CommandOrControl+Shift+T', 'CommandOrControl+Shift+I', 'CommandOrControl+Shift+N',
  'CommandOrControl+Shift+Z', 'CommandOrControl+Shift+Y'
];
const INPUT_IDS = ['toggleVisibilityHotkey', 'toggleInteractiveHotkey', 'completeNextTaskHotkey', 'undoLastActionHotkey', 'redoLastActionHotkey'];
const regSequence = (accelerators: string[]) => ['shortcut.unregisterAll', ...accelerators.map(a => `shortcut.register(${a})`)];
const REG_DEFAULT = regSequence(MAIN_DEFAULTS);
const SAVED_ALERT = 'alert: Settings saved successfully! Hotkey changes are active now.';

// ---------------------------------------------------------------- provenance
interface Manifest {
  baseSha: string;
  typeOnlyExcluded: string[];
  entryModules: string[];
  markup: Record<string, string>;
  modules: Record<string, string>;
}
const sha256 = (rel: string) => createHash('sha256').update(readFileSync(resolve(ROOT, rel))).digest('hex');

const IMPORT_RE = /(?:^|\n)\s*(?:import|export)\s+(type\s+)?(?:[^'";]*?\s+from\s+)?['"](\.{1,2}\/[^'"]+)['"]/g;
function scanClosure(entries: string[]): { values: Set<string>; typeOnly: Set<string>; unresolved: string[] } {
  const values = new Set<string>();
  const typeOnly = new Set<string>();
  const unresolved: string[] = [];
  const queue = [...entries];
  const resolveSpec = (from: string, spec: string) => {
    const candidate = relative(ROOT, resolve(dirname(resolve(ROOT, from)), `${spec}.ts`));
    return existsSync(resolve(ROOT, candidate)) ? candidate : null;
  };
  while (queue.length) {
    const file = queue.pop() as string;
    if (values.has(file)) continue;
    values.add(file);
    const source = readFileSync(resolve(ROOT, file), 'utf8');
    for (const match of source.matchAll(IMPORT_RE)) {
      const target = resolveSpec(file, match[2]);
      if (!target) { unresolved.push(`${file} -> ${match[2]}`); continue; }
      if (match[1]) typeOnly.add(target); else queue.push(target);
    }
  }
  return { values, typeOnly, unresolved };
}

test('source provenance matches pinned manifest', () => {
  const manifest = JSON.parse(readFileSync(resolve(ROOT, 'tools/golden/settings-ui.provenance.json'), 'utf8')) as Manifest;
  expect(manifest.baseSha).toBe(BASE_SHA);
  expect(window.navigator.platform).toBe('Win32');
  const mismatches: string[] = [];
  for (const [file, expected] of [...Object.entries(manifest.markup), ...Object.entries(manifest.modules)]) {
    const actual = sha256(file);
    console.log(`provenance base=${manifest.baseSha} sha256=${actual} ${file}`);
    if (actual !== expected) mismatches.push(`${file}: expected ${expected} actual ${actual}`);
  }
  expect(mismatches).toEqual([]);

  // Closure completeness: every local value-import reachable from the entries is pinned, nothing stale.
  const closure = scanClosure(manifest.entryModules);
  expect(closure.unresolved).toEqual([]);
  expect([...closure.values].sort()).toEqual(Object.keys(manifest.modules).sort());
  const unexplainedTypeOnly = [...closure.typeOnly].filter(f => !closure.values.has(f) && !manifest.typeOnlyExcluded.includes(f));
  expect(unexplainedTypeOnly).toEqual([]);
  // Type-only imports are erased at runtime and are deliberately excluded from the manifest.
  for (const file of manifest.typeOnlyExcluded) expect(Object.keys(manifest.modules)).not.toContain(file);
});

// ---------------------------------------------------------------- harness
type Listener = (...args: unknown[]) => void;
interface TrackedListener { target: EventTarget; type: string; listener: EventListenerOrEventListenerObject; options?: boolean | AddEventListenerOptions }
let teardownCurrent: (() => void) | null = null;
afterEach(() => { teardownCurrent?.(); });
const flush = () => vi.advanceTimersByTimeAsync(0);

function settingsFragment(): string {
  const html = readFileSync(resolve(ROOT, 'src/renderer/index.html'), 'utf8');
  const start = html.indexOf('<!-- Settings Modal -->');
  const end = html.indexOf('<!-- Theme Selection Modal -->');
  expect(start).toBeGreaterThan(-1);
  expect(end).toBeGreaterThan(start);
  const fragment = html.slice(start, end);
  expect(fragment).toContain('id="settingsModal"');
  return fragment;
}

function boot(options: { hotkeysFromMain?: boolean } = {}) {
  teardownCurrent?.();
  void options;
  vi.useFakeTimers();
  document.body.innerHTML = settingsFragment();
  const events: string[] = [];
  stub.sink = events;
  stub.callbacks.clear();
  stub.registerResult = () => true;
  stub.registerThrows = () => false;

  const tracked: TrackedListener[] = [];
  for (const target of [window, document] as EventTarget[]) {
    const original = target.addEventListener.bind(target);
    vi.spyOn(target, 'addEventListener').mockImplementation((type: string, listener: EventListenerOrEventListenerObject, opts?: boolean | AddEventListenerOptions) => {
      tracked.push({ target, type, listener, options: opts });
      original(type, listener, opts);
    });
  }

  // --- fake window + renderer event bus (only external effects)
  const bus = new Map<string, Listener[]>();
  const win = {
    visible: true,
    destroyed: false,
    isVisible: () => win.visible,
    isDestroyed: () => win.destroyed,
    show: () => { win.visible = true; events.push('window.show'); },
    hide: () => { win.visible = false; events.push('window.hide'); },
    focus: () => { events.push('window.focus'); },
    setFocusable: (value: boolean) => { events.push(`window.setFocusable(${value})`); },
    setAlwaysOnTop: (...args: unknown[]) => { events.push(`window.setAlwaysOnTop(${args.join(',')})`); },
    getBounds: () => ({ width: 500 }),
    setPosition: (x: number, y: number) => { events.push(`window.setPosition(${x},${y})`); },
    webContents: {
      focus: () => { events.push('webContents.focus'); },
      send: (channel: string) => { events.push(`webContents.send(${channel})`); (bus.get(channel) ?? []).forEach(l => l()); }
    }
  };

  // --- fake ipcMain wired to the REAL registerWindowIpc
  const handlers = new Map<string, (event: unknown, payload?: unknown) => void>();
  const ipcMain = { on: (channel: string, handler: (event: unknown, payload?: unknown) => void) => { handlers.set(channel, handler); } };
  const ipc = { emit: (channel: string, payload?: unknown) => handlers.get(channel)?.({}, payload) };

  const mainLogger = { log: vi.fn(), error: vi.fn() };
  const main = {
    current: { ...DEFAULT_HOTKEYS },
    window: win as typeof win | null,
    setHotkeysCalls: [] as unknown[],
    logger: mainLogger,
    register: () => registerOverlayHotkeys({
      hotkeys: main.current,
      isPackaged: false,
      getWindow: () => main.window as never,
      setManuallyHidden: hidden => { events.push(`main.setManuallyHidden(${hidden})`); },
      toggleInteractiveMode: () => { events.push('main.toggleInteractiveMode'); },
      logger: mainLogger
    })
  };
  // HARNESS WIRING (mirrors src/main/index.ts setupIPC; not executed main/index.ts).
  registerWindowIpc(ipcMain as never, {
    getWindow: () => main.window as never,
    getHotkeys: () => main.current,
    setHotkeys: next => { main.setHotkeysCalls.push(next); main.current = next; main.register(); },
    setHotkeyRecording: recording => { if (recording) unregisterOverlayHotkeys(); else main.register(); },
    setManuallyHidden: hidden => { events.push(`main.setManuallyHidden(${hidden})`); },
    toggleInteractiveMode: () => { events.push('main.toggleInteractiveMode'); },
    quitApplication: () => { events.push('main.quit'); },
    logger: mainLogger
  });
  main.register(); // harness mirror of the initial registerHotkeys() at window creation
  events.length = 0;

  // --- renderer api spies (application boundary)
  const api = {
    loadSettings: vi.fn(async (): Promise<unknown> => undefined),
    saveSettings: vi.fn(async (_settings: unknown): Promise<{ success: boolean; error?: string }> => { events.push('api.saveSettings'); return { success: true }; }),
    updateHotkeys: vi.fn((hotkeys: unknown) => { events.push('api.updateHotkeys'); ipc.emit('update-hotkeys', hotkeys); }),
    setHotkeyRecording: vi.fn((recording: boolean) => { events.push(`api.setHotkeyRecording(${recording})`); ipc.emit('set-hotkey-recording', recording); }),
    getTheme: vi.fn(async () => null),
    getThemeAsset: vi.fn(async () => null),
    loadThemeCss: vi.fn(async () => null),
    onInteractiveModeChanged: (cb: Listener) => { bus.set('interactive-mode-changed', [...(bus.get('interactive-mode-changed') ?? []), cb]); return () => undefined; },
    onCompleteNextTask: (cb: Listener) => { bus.set('complete-next-task', [...(bus.get('complete-next-task') ?? []), cb]); return () => undefined; },
    onUndoLastTaskAction: (cb: Listener) => { bus.set('undo-last-task-action', [...(bus.get('undo-last-task-action') ?? []), cb]); return () => undefined; },
    onRedoLastTaskAction: (cb: Listener) => { bus.set('redo-last-task-action', [...(bus.get('redo-last-task-action') ?? []), cb]); return () => undefined; }
  };

  const alertSnapshots: string[][] = [];
  const alerts: string[] = [];
  const confirms: Array<[string, unknown]> = [];
  const dialogs = { confirmResult: true };
  const alertUser = vi.fn(async (message: string) => {
    alerts.push(message);
    alertSnapshots.push(INPUT_IDS.map(id => (document.getElementById(id) as HTMLInputElement).value));
    events.push(`alert: ${message}`);
  });
  const confirmUser = vi.fn(async (message: string, opts?: unknown) => { confirms.push([message, opts]); events.push(`confirm: ${message}`); return dialogs.confirmResult; });
  const logger = { log: vi.fn(), warn: vi.fn(), error: vi.fn() };
  const onSoundSettingsChanged = vi.fn();

  const controller = createSettingsController({
    api: api as never,
    alertUser,
    confirmUser,
    getIsInteractiveMode: () => true,
    getThemes: () => [{ id: 'echosight', name: 'Echosight' }] as never,
    logger,
    onInteractiveRefresh: vi.fn(),
    onSoundSettingsChanged
  });

  const unrelated = vi.fn();
  setupRendererControls({
    addNewBackgroundTaskFromTriggerModal: unrelated, addTask: unrelated, clearAllTasks: unrelated,
    closeCommunityModal: unrelated, closeImportModal: unrelated, closeOverlay: unrelated,
    closeSaveTemplateModal: unrelated, closeSettingsModal: controller.closeSettingsModal,
    closeSubTaskModal: unrelated, closeTaskEditor: unrelated,
    closeThemeSelectionModal: controller.closeThemeSelectionModal, closeTriggersModal: unrelated,
    deleteTemplate: unrelated, exportTemplate: unrelated, handleFileImport: unrelated,
    hotkeyRecorder: controller.hotkeyRecorder, importTemplate: unrelated, loadCommunityTemplates: unrelated,
    loadTemplate: unrelated, logger: { log() {}, error() {} }, minimizeOverlay: unrelated,
    openThemesFolder: unrelated, reloadThemes: unrelated, resetHotkeys: controller.resetHotkeys,
    resetPosition: unrelated, saveSettings: controller.saveSettings, saveSubTask: unrelated,
    saveTaskEdit: unrelated, saveTemplate: unrelated, saveTriggers: unrelated, showImportModal: unrelated,
    showSaveTemplateModal: unrelated, showSettingsModal: controller.showSettingsModal,
    showThemeSelection: controller.showThemeSelection, toggleInteractiveMode: unrelated,
    updateSoundEnabled: controller.updateSoundEnabled, updateSoundVolume: controller.updateSoundVolume,
    updateTheme: controller.updateTheme, updateTransparency: controller.updateTransparency
  } as never);

  const byId = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
  const harness = {
    events, api, controller, main, win, ipc, stub, logger, alertUser, confirmUser, alerts, alertSnapshots, confirms,
    dialogs, onSoundSettingsChanged, bus,
    byId,
    value: (id: string) => byId<HTMLInputElement>(id).value,
    inputs: () => INPUT_IDS.map(id => byId<HTMLInputElement>(id).value),
    modalVisible: () => byId('settingsModal').classList.contains('is-visible'),
    apiEvents: () => events.filter(e => e.startsWith('api.')),
    /** showSettingsModal, then flush the modal focus-retry timers (<=450ms) so later timers are recorder-only. */
    open: () => { controller.showSettingsModal(); vi.advanceTimersByTime(450); },
    click: async (id: string) => { byId(id).click(); await flush(); },
    key: (init: KeyboardEventInit) => {
      const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
      document.body.dispatchEvent(event);
      return event;
    },
    record: async (buttonId: string, init: KeyboardEventInit) => { await harness.click(buttonId); return harness.key(init); }
  };

  teardownCurrent = () => {
    hideModal('settingsModal'); // real source: releases modalUi's module-level focus stack/listener
    for (const t of tracked) t.target.removeEventListener(t.type, t.listener, t.options);
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.restoreAllMocks();
    document.body.replaceChildren();
    teardownCurrent = null;
  };
  return harness;
}
type Harness = ReturnType<typeof boot>;

const idxOf = (events: string[], needle: string) => events.indexOf(needle);
const tail = (events: string[], marker: string) => events.slice(events.indexOf(marker) + 1);

// ---------------------------------------------------------------- renderer observations
test('cancel keeps recorded in-memory hotkey and reopen displays it without save or updateHotkeys', async () => {
  const h = boot();
  h.open();
  expect(h.inputs()).toEqual(DISPLAY_DEFAULTS);

  const keydown = await h.record('recordBtn1', { key: 'F1', ctrlKey: true });
  expect(keydown.defaultPrevented).toBe(true);
  expect(h.value('toggleVisibilityHotkey')).toBe('Ctrl+F1');
  expect(h.controller.getSettings().hotkeys.toggleVisibility).toBe('Ctrl+F1');
  vi.advanceTimersByTime(200); // stop({restorePrevious:false}) keeps the recorded text
  expect(h.value('toggleVisibilityHotkey')).toBe('Ctrl+F1');
  const beforeCancel = h.events.length;

  await h.click('closeSettingsButton');
  expect(h.modalVisible()).toBe(false);
  // Cancel while idle: hotkeyRecorder.stop() is a no-op => NO setHotkeyRecording, NO registration churn.
  expect(h.events.slice(beforeCancel)).toEqual([]);

  h.open();
  expect(h.value('toggleVisibilityHotkey')).toBe('Ctrl+F1');
  expect(h.controller.getSettings().hotkeys.toggleVisibility).toBe('Ctrl+F1');
  expect(h.api.saveSettings).not.toHaveBeenCalled();
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();
  // Recording end emitted false => main resumes COMMITTED registrations, never the recorded mapping.
  expect(h.events).toEqual([
    'api.setHotkeyRecording(true)', 'shortcut.unregisterAll',
    'api.setHotkeyRecording(false)', ...REG_DEFAULT
  ]);
  expect(h.main.current).toEqual(DEFAULT_HOTKEYS);
});

test('cancel while recording ends it with setHotkeyRecording false and restores display, keeping in-memory value (extra trace)', async () => {
  const h = boot();
  h.open();
  await h.click('recordBtn1');
  expect(h.value('toggleVisibilityHotkey')).toBe(PLACEHOLDER);
  await h.click('closeSettingsButton');
  expect(h.modalVisible()).toBe(false);
  expect(h.value('toggleVisibilityHotkey')).toBe('Ctrl+Shift+T');
  expect(h.byId('recordBtn1').textContent?.trim()).toBe('Record');
  expect(h.events).toEqual(['api.setHotkeyRecording(true)', 'shortcut.unregisterAll', 'api.setHotkeyRecording(false)', ...REG_DEFAULT]);

  // Pending 200ms stop + Cancel before it fires: restore (default) overwrites input, memory keeps recorded value.
  h.events.length = 0;
  h.open();
  await h.record('recordBtn2', { key: 'F2', ctrlKey: true });
  expect(h.controller.getSettings().hotkeys.toggleInteractive).toBe('Ctrl+F2');
  await h.click('closeSettingsButton');
  expect(h.value('toggleInteractiveHotkey')).toBe('Ctrl+Shift+I');
  expect(h.controller.getSettings().hotkeys.toggleInteractive).toBe('Ctrl+F2');
  const emitted = h.apiEvents().length;
  vi.advanceTimersByTime(200); // stale timer: recorder already idle => nothing more
  expect(h.apiEvents().length).toBe(emitted);
  h.open();
  expect(h.value('toggleInteractiveHotkey')).toBe('Ctrl+F2');
});

test('save stops active recording with restore before reading controls', async () => {
  const h = boot();
  h.open();
  await h.click('recordBtn1');
  expect(h.value('toggleVisibilityHotkey')).toBe(PLACEHOLDER);
  expect(h.controller.hotkeyRecorder.isRecording()).toBe(true);

  await h.click('saveSettingsButton');
  expect(h.events).toEqual([
    'api.setHotkeyRecording(true)', 'shortcut.unregisterAll',
    'api.setHotkeyRecording(false)', ...REG_DEFAULT,
    'api.saveSettings', 'api.updateHotkeys',
    SAVED_ALERT
  ]);
  const saved = h.api.saveSettings.mock.calls[0][0] as { hotkeys: Record<string, string> };
  expect(saved.hotkeys.toggleVisibility).toBe('Ctrl+Shift+T'); // restored text, not the placeholder
  expect(h.modalVisible()).toBe(false);
});

test('save persists display-text hotkeys then calls updateHotkeys once', async () => {
  const h = boot();
  h.open();
  await h.record('recordBtn3', { key: 'q', altKey: true });
  expect(h.value('completeNextTaskHotkey')).toBe('Alt+Q');
  vi.advanceTimersByTime(200);

  await h.click('saveSettingsButton');
  const expectedHotkeys = {
    toggleVisibility: 'Ctrl+Shift+T', toggleInteractive: 'Ctrl+Shift+I', completeNextTask: 'Alt+Q',
    undoLastAction: 'Ctrl+Shift+Z', redoLastAction: 'Ctrl+Shift+Y'
  };
  expect(h.api.saveSettings).toHaveBeenCalledTimes(1);
  expect(h.api.saveSettings.mock.calls[0][0]).toEqual({
    settingsVersion: 2, transparency: 70, theme: 'echosight', hotkeys: expectedHotkeys,
    sounds: { enabled: false, volume: 60 }
  });
  expect(h.api.updateHotkeys).toHaveBeenCalledTimes(1);
  expect(h.api.updateHotkeys.mock.calls[0][0]).toEqual(expectedHotkeys); // display text (Ctrl+), not CommandOrControl
  expect(h.apiEvents()).toEqual([
    'api.setHotkeyRecording(true)', 'api.setHotkeyRecording(false)', 'api.saveSettings', 'api.updateHotkeys'
  ]);
  expect(h.alerts).toEqual(['Settings saved successfully! Hotkey changes are active now.']);
  expect(h.modalVisible()).toBe(false);
  expect(h.logger.log).toHaveBeenCalledWith('Saving settings...');
});

test('save failure alerts exact text, keeps modal, skips updateHotkeys', async () => {
  const h = boot();
  h.open();
  (h.byId('transparencySlider') as HTMLInputElement).value = '55';
  const failWith = (impl: () => Promise<{ success: boolean; error?: string }>) =>
    h.api.saveSettings.mockImplementation(async () => { h.events.push('api.saveSettings'); return impl(); });

  failWith(async () => ({ success: false, error: 'disk full' }));
  await h.click('saveSettingsButton');
  expect(h.alerts).toEqual(['Error saving settings: disk full. Please try again.']);
  expect(h.modalVisible()).toBe(true);
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();
  expect(h.logger.error).toHaveBeenCalledWith('Error saving settings:', expect.objectContaining({ message: 'disk full' }));
  // Pin: in-memory settings were already replaced by the control values before persistence failed.
  expect(h.controller.getSettings().transparency).toBe(55);

  failWith(async () => ({ success: false }));
  await h.click('saveSettingsButton');
  expect(h.alerts[1]).toBe('Error saving settings: Unknown save error. Please try again.');

  failWith(async () => { throw new Error('boom'); });
  await h.click('saveSettingsButton');
  expect(h.alerts[2]).toBe('Error saving settings: boom. Please try again.');

  failWith(async () => { throw 'plain'; }); // eslint-disable-line no-throw-literal
  await h.click('saveSettingsButton');
  expect(h.alerts[3]).toBe('Error saving settings: plain. Please try again.');

  expect(h.alerts).toHaveLength(4);
  expect(h.modalVisible()).toBe(true);
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();
  expect(h.events.filter(e => e.startsWith('shortcut.'))).toEqual([]);
});

test('reset confirm danger text then OK writes defaults and alerts without save', async () => {
  const h = boot();
  h.open();
  await h.record('recordBtn3', { key: 'q', altKey: true });
  vi.advanceTimersByTime(200);
  expect(h.controller.getSettings().hotkeys.completeNextTask).toBe('Alt+Q');
  h.events.length = 0;

  await h.click('resetHotkeysButton');
  expect(h.confirms).toEqual([[
    'Reset hotkeys to default values (Ctrl+Shift+T, Ctrl+Shift+I, Ctrl+Shift+N, Ctrl+Shift+Z, Ctrl+Shift+Y)?',
    { title: 'Reset Hotkeys', tone: 'danger' }
  ]]);
  expect(h.events).toEqual([
    'confirm: Reset hotkeys to default values (Ctrl+Shift+T, Ctrl+Shift+I, Ctrl+Shift+N, Ctrl+Shift+Z, Ctrl+Shift+Y)?',
    'alert: Hotkeys reset to defaults. Click "Save Settings" to apply.'
  ]);
  expect(h.alertSnapshots[0]).toEqual(DISPLAY_DEFAULTS); // controls already rewritten when the alert shows
  expect(h.inputs()).toEqual(DISPLAY_DEFAULTS);
  expect(Object.values(h.controller.getSettings().hotkeys)).toEqual(DISPLAY_DEFAULTS);
  expect(h.api.saveSettings).not.toHaveBeenCalled();
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();
  expect(h.modalVisible()).toBe(true);

  // Declined confirm: nothing changes, no alert.
  h.dialogs.confirmResult = false;
  await h.record('recordBtn3', { key: 'w', altKey: true });
  vi.advanceTimersByTime(200);
  h.events.length = 0;
  await h.click('resetHotkeysButton');
  expect(h.events).toEqual([expect.stringMatching(/^confirm: /)]);
  expect(h.controller.getSettings().hotkeys.completeNextTask).toBe('Alt+W');
});

test('switching record emits setHotkeyRecording false then true', async () => {
  const h = boot();
  h.open();
  await h.click('recordBtn1');
  await h.click('recordBtn2');
  expect(h.apiEvents()).toEqual(['api.setHotkeyRecording(true)', 'api.setHotkeyRecording(false)', 'api.setHotkeyRecording(true)']);
  expect(h.events).toEqual([
    'api.setHotkeyRecording(true)', 'shortcut.unregisterAll',
    'api.setHotkeyRecording(false)', ...REG_DEFAULT,
    'api.setHotkeyRecording(true)', 'shortcut.unregisterAll'
  ]);
  expect(h.value('toggleVisibilityHotkey')).toBe('Ctrl+Shift+T');
  expect(h.byId('recordBtn1').textContent?.trim()).toBe('Record');
  expect(h.value('toggleInteractiveHotkey')).toBe(PLACEHOLDER);
  expect(h.byId('recordBtn2').textContent?.trim()).toBe('Cancel');
  expect(stub.callbacks.size).toBe(0); // stub-level: nothing registered while recording

  await h.click('recordBtn2'); // same button toggles off, restoring
  expect(h.apiEvents().slice(3)).toEqual(['api.setHotkeyRecording(false)']);
  expect(h.value('toggleInteractiveHotkey')).toBe('Ctrl+Shift+I');
});

test('bare Escape while recording is consumed and modal stays open', async () => {
  const h = boot();
  h.open();
  await h.click('recordBtn1');
  let cancelClicks = 0;
  h.byId('closeSettingsButton').addEventListener('click', () => { cancelClicks++; });

  const escape = h.key({ key: 'Escape' });
  expect(escape.defaultPrevented).toBe(true);
  expect(cancelClicks).toBe(0);
  expect(h.modalVisible()).toBe(true);
  expect(h.controller.hotkeyRecorder.isRecording()).toBe(true);
  expect(h.value('toggleVisibilityHotkey')).toBe(PLACEHOLDER);
  expect(h.apiEvents()).toEqual(['api.setHotkeyRecording(true)']); // no false emitted
  expect(h.alerts).toEqual([]);
});

test('Escape when idle clicks Cancel via data-modal-close', async () => {
  const h = boot();
  h.open();
  const cancel = h.byId('closeSettingsButton');
  expect(cancel.hasAttribute('data-modal-close')).toBe(true);
  expect(cancel.textContent?.trim()).toBe('Cancel');
  const clicked: string[] = [];
  document.addEventListener('click', event => { clicked.push((event.target as HTMLElement).id); });
  // (listener added after boot's spy on document => tracked and removed in teardown)

  const escape = h.key({ key: 'Escape' });
  expect(escape.defaultPrevented).toBe(true);
  expect(clicked).toEqual(['closeSettingsButton']);
  expect(h.modalVisible()).toBe(false);
  expect(h.byId('settingsModal').getAttribute('aria-hidden')).toBe('true');
  expect(h.events).toEqual([]); // idle: no recording/registration events
});

test('stale 200ms stop ends newer recording without restore; save then persists placeholder text', async () => {
  const h = boot();
  h.open();
  await h.record('recordBtn1', { key: 'F1', ctrlKey: true }); // schedules stop({restorePrevious:false}) at +200ms
  expect(h.controller.getSettings().hotkeys.toggleVisibility).toBe('Ctrl+F1');
  vi.advanceTimersByTime(100);
  await h.click('recordBtn2'); // switch: stop(restore) on #1 (restores input), start #2
  expect(h.value('toggleVisibilityHotkey')).toBe('Ctrl+Shift+T');
  expect(h.value('toggleInteractiveHotkey')).toBe(PLACEHOLDER);
  expect(h.controller.hotkeyRecorder.isRecording()).toBe(true);
  vi.advanceTimersByTime(100); // the STALE timer fires against the newer recording
  expect(h.controller.hotkeyRecorder.isRecording()).toBe(false);
  expect(h.value('toggleInteractiveHotkey')).toBe(PLACEHOLDER); // no restore
  expect(h.byId('recordBtn2').textContent?.trim()).toBe('Record');
  expect(h.apiEvents()).toEqual([
    'api.setHotkeyRecording(true)', 'api.setHotkeyRecording(false)',
    'api.setHotkeyRecording(true)', 'api.setHotkeyRecording(false)'
  ]);

  await h.click('saveSettingsButton');
  const saved = h.api.saveSettings.mock.calls[0][0] as { hotkeys: Record<string, string> };
  expect(saved.hotkeys.toggleInteractive).toBe(PLACEHOLDER);
  expect(saved.hotkeys.toggleVisibility).toBe('Ctrl+Shift+T'); // recorded Ctrl+F1 lost: save reads controls
  expect(h.api.updateHotkeys.mock.calls[0][0]).toEqual(saved.hotkeys);
  // Stub-level only: main normalizes (placeholder passes through) and the source asks the stub to register it.
  expect(tail(h.events, 'api.updateHotkeys').slice(0, 6)).toEqual(
    regSequence(['CommandOrControl+Shift+T', PLACEHOLDER, 'CommandOrControl+Shift+N', 'CommandOrControl+Shift+Z', 'CommandOrControl+Shift+Y'])
  );
  expect(h.events[h.events.length - 1]).toBe(SAVED_ALERT);
});

test('pending save completion while a newer recording active (immediate update/no generation guard)', async () => {
  const h = boot();
  h.open();
  await h.record('recordBtn3', { key: 'q', altKey: true });
  vi.advanceTimersByTime(200);
  h.events.length = 0;

  let resolveSave!: (value: { success: boolean }) => void;
  h.api.saveSettings.mockImplementation(() => { h.events.push('api.saveSettings'); return new Promise(resolve => { resolveSave = resolve; }); });
  await h.click('saveSettingsButton'); // pending: controls already read, awaiting persistence
  expect(h.events).toEqual(['api.saveSettings']);
  expect((h.api.saveSettings.mock.calls[0][0] as { hotkeys: Record<string, string> }).hotkeys.toggleInteractive).toBe('Ctrl+Shift+I');

  await h.click('recordBtn2'); // newer recording starts while save is in flight
  expect(h.controller.hotkeyRecorder.isRecording()).toBe(true);
  resolveSave({ success: true });
  await flush();

  expect(h.events).toEqual([
    'api.saveSettings',
    'api.setHotkeyRecording(true)', 'shortcut.unregisterAll',
    // updateHotkeys fires immediately although a recording is active (no generation guard);
    // main sees a changed mapping and re-registers while recording:
    'api.updateHotkeys', ...regSequence(['CommandOrControl+Shift+T', 'CommandOrControl+Shift+I', 'Alt+Q', 'CommandOrControl+Shift+Z', 'CommandOrControl+Shift+Y']),
    // closeSettingsModal() then stops the newer recording (restore) => false => re-register again:
    'api.setHotkeyRecording(false)', ...regSequence(['CommandOrControl+Shift+T', 'CommandOrControl+Shift+I', 'Alt+Q', 'CommandOrControl+Shift+Z', 'CommandOrControl+Shift+Y']),
    SAVED_ALERT
  ]);
  expect(h.modalVisible()).toBe(false);
  expect(h.controller.hotkeyRecorder.isRecording()).toBe(false);
  expect(h.value('toggleInteractiveHotkey')).toBe('Ctrl+Shift+I');
});

test('missing/null settings startup migration writes defaults with no updateHotkeys', async () => {
  for (const loaded of [undefined, null]) {
    const h = boot();
    h.api.loadSettings.mockResolvedValue(loaded);
    await h.controller.loadSettings();
    const defaults = {
      settingsVersion: 2, transparency: 70, theme: 'echosight',
      hotkeys: { toggleVisibility: 'Ctrl+Shift+T', toggleInteractive: 'Ctrl+Shift+I', completeNextTask: 'Ctrl+Shift+N', undoLastAction: 'Ctrl+Shift+Z', redoLastAction: 'Ctrl+Shift+Y' },
      sounds: { enabled: false, volume: 60 }
    };
    expect(h.api.saveSettings).toHaveBeenCalledTimes(1);
    expect(h.api.saveSettings.mock.calls[0][0]).toEqual(defaults);
    expect(h.api.updateHotkeys).not.toHaveBeenCalled();
    expect(h.events).toEqual(['api.saveSettings']);
    expect(h.controller.getSettings()).toEqual(defaults);
    expect(h.logger.log).toHaveBeenCalledWith('Persisting migrated settings:', defaults);
    expect(h.onSoundSettingsChanged).toHaveBeenCalledTimes(1);
    teardownCurrent?.();
  }
  // Contrast: already-current settings are not re-persisted.
  const h = boot();
  h.api.loadSettings.mockResolvedValue({
    settingsVersion: 2, transparency: 70, theme: 'echosight',
    hotkeys: { toggleVisibility: 'Ctrl+Shift+T', toggleInteractive: 'Ctrl+Shift+I', completeNextTask: 'Ctrl+Shift+N', undoLastAction: 'Ctrl+Shift+Z', redoLastAction: 'Ctrl+Shift+Y' },
    sounds: { enabled: false, volume: 60 }
  });
  await h.controller.loadSettings();
  expect(h.api.saveSettings).not.toHaveBeenCalled();
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();
});

test('migration failure logged nonfatal', async () => {
  const h = boot();
  h.api.loadSettings.mockResolvedValue(null);
  h.api.saveSettings.mockResolvedValue({ success: false, error: 'EACCES' });
  await expect(h.controller.loadSettings()).resolves.toBeUndefined();
  expect(h.logger.error).toHaveBeenCalledWith('Failed to persist migrated settings:', 'EACCES');
  expect(h.controller.getSettings().hotkeys.toggleVisibility).toBe('Ctrl+Shift+T'); // normalized settings still adopted
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();

  h.logger.error.mockClear();
  h.api.saveSettings.mockResolvedValue({ success: false });
  await h.controller.loadSettings();
  expect(h.logger.error).toHaveBeenCalledWith('Failed to persist migrated settings:', 'Unknown save error');

  // A rejected save is caught by loadSettingsState's outer catch and returns the fallback.
  h.logger.error.mockClear();
  const failure = new Error('ipc down');
  h.api.saveSettings.mockRejectedValue(failure);
  await expect(h.controller.loadSettings()).resolves.toBeUndefined();
  expect(h.logger.error).toHaveBeenCalledWith('Failed to load settings:', failure);
  expect(h.api.updateHotkeys).not.toHaveBeenCalled();
});

// ---------------------------------------------------------------- main-side observations (stubbed electron)
test('update-hotkeys normalizes against current and re-registers only when changed', () => {
  const h = boot();
  const reRegistrations = () => h.events.filter(e => e === 'shortcut.unregisterAll').length;

  // 1. renderer display text equal to current after normalization => no setHotkeys, no registration.
  h.ipc.emit('update-hotkeys', Object.fromEntries(
    ['toggleVisibility', 'toggleInteractive', 'completeNextTask', 'undoLastAction', 'redoLastAction'].map((k, i) => [k, DISPLAY_DEFAULTS[i]])
  ));
  expect(h.main.setHotkeysCalls).toEqual([]);
  expect(h.events).toEqual([]);

  // 2. one changed chord => one setHotkeys and one full re-registration with the normalized mapping.
  h.ipc.emit('update-hotkeys', { ...DEFAULT_HOTKEYS, completeNextTask: 'Alt+Q' });
  expect(h.main.setHotkeysCalls).toHaveLength(1);
  expect(h.events).toEqual(regSequence(['CommandOrControl+Shift+T', 'CommandOrControl+Shift+I', 'Alt+Q', 'CommandOrControl+Shift+Z', 'CommandOrControl+Shift+Y']));

  // 3. unchanged repeat, null and non-object payloads fall back to CURRENT (Alt+Q kept), no re-register.
  h.events.length = 0;
  h.ipc.emit('update-hotkeys', { ...DEFAULT_HOTKEYS, completeNextTask: 'Alt+Q' });
  h.ipc.emit('update-hotkeys', null);
  h.ipc.emit('update-hotkeys', 'garbage');
  h.ipc.emit('update-hotkeys', { toggleVisibility: '   ' });
  expect(h.main.setHotkeysCalls).toHaveLength(1);
  expect(reRegistrations()).toBe(0);

  // 4. partial payload: missing keys fall back to current (not to defaults); Ctrl/Cmd collapse and dedupe.
  h.ipc.emit('update-hotkeys', { toggleVisibility: 'Ctrl+F2', undoLastAction: 'Cmd+Ctrl+Q' });
  expect(h.main.setHotkeysCalls).toHaveLength(2);
  expect(h.main.current).toEqual({
    toggleVisibility: 'CommandOrControl+F2', toggleInteractive: 'CommandOrControl+Shift+I', completeNextTask: 'Alt+Q',
    undoLastAction: 'CommandOrControl+Q', redoLastAction: 'CommandOrControl+Shift+Y'
  });
  expect(h.events).toEqual(regSequence(['CommandOrControl+F2', 'CommandOrControl+Shift+I', 'Alt+Q', 'CommandOrControl+Q', 'CommandOrControl+Shift+Y']));
  expect(normalizeHotkeys({ toggleVisibility: 'Ctrl+F2' }, h.main.current, { log() {} }).toggleVisibility).toBe('CommandOrControl+F2');
});

test('registerOverlayHotkeys unregisters all then five in order, partial failure nonfatal', () => {
  const h = boot();
  const hotkeys = { toggleVisibility: 'A+1', toggleInteractive: 'A+2', completeNextTask: 'A+3', undoLastAction: 'A+4', redoLastAction: 'A+5' };
  h.main.current = hotkeys;

  h.main.register();
  expect(h.events).toEqual(regSequence(['A+1', 'A+2', 'A+3', 'A+4', 'A+5']));
  expect(h.main.logger.error).not.toHaveBeenCalled();
  expect(h.main.logger.log).toHaveBeenCalledWith('Total: 5/5 hotkeys registered');

  // Partial failure (stub returns false for two): all five still attempted, logged FAILED, no error, no throw.
  h.events.length = 0;
  h.main.logger.log.mockClear();
  stub.registerResult = accelerator => accelerator !== 'A+2' && accelerator !== 'A+4';
  expect(() => h.main.register()).not.toThrow();
  expect(h.events).toEqual(regSequence(['A+1', 'A+2', 'A+3', 'A+4', 'A+5']));
  expect(h.main.logger.error).not.toHaveBeenCalled();
  const lines = h.main.logger.log.mock.calls.map(call => call[0]);
  expect(lines).toEqual(expect.arrayContaining(['  A+1: SUCCESS', '  A+2: FAILED', '  A+3: SUCCESS', '  A+4: FAILED', '  A+5: SUCCESS', 'Total: 3/5 hotkeys registered']));
  expect([...stub.callbacks.keys()]).toEqual(['A+1', 'A+3', 'A+5']);

  // Total failure: logged as error, thrown inside the source and swallowed by its catch (nonfatal).
  h.events.length = 0;
  h.main.logger.error.mockClear();
  stub.registerResult = () => false;
  expect(() => h.main.register()).not.toThrow();
  expect(h.main.logger.error.mock.calls[0]).toEqual(['No hotkeys registered successfully']);
  expect(h.main.logger.error.mock.calls[1][0]).toBe('Error in registerHotkeys:');
  expect((h.main.logger.error.mock.calls[1][1] as Error).message).toBe('Hotkey registration completely failed');

  // A throwing register stops the sequence at that call; unregisterAll already happened; nonfatal.
  h.events.length = 0;
  h.main.logger.error.mockClear();
  stub.registerResult = () => true;
  stub.registerThrows = accelerator => accelerator === 'A+3';
  expect(() => h.main.register()).not.toThrow();
  expect(h.events).toEqual(regSequence(['A+1', 'A+2', 'A+3']));
  expect(h.main.logger.error.mock.calls[0][0]).toBe('Error in registerHotkeys:');
});

test('toggle visibility always; other four only when visible', () => {
  const h = boot();
  const fire = (accelerator: string) => (stub.callbacks.get(accelerator) as () => void)();
  const [toggle, interactive, next, undo, redo] = MAIN_DEFAULTS;

  // Hidden: toggle works (shows, then manuallyHidden=false); the other four do nothing.
  h.win.visible = false;
  fire(interactive); fire(next); fire(undo); fire(redo);
  expect(h.events).toEqual([]);
  fire(toggle);
  expect(h.events).toEqual([
    'window.setFocusable(true)', 'window.show', 'window.setAlwaysOnTop(true,screen-saver)', 'window.focus', 'webContents.focus',
    'main.setManuallyHidden(false)'
  ]);
  expect(h.win.visible).toBe(true);

  // Visible: the other four act.
  h.events.length = 0;
  fire(interactive); fire(next); fire(undo); fire(redo);
  expect(h.events).toEqual([
    'main.toggleInteractiveMode',
    'webContents.send(complete-next-task)', 'webContents.send(undo-last-task-action)', 'webContents.send(redo-last-task-action)'
  ]);

  // Visible: toggle hides and sets manuallyHidden=true.
  h.events.length = 0;
  fire(toggle);
  expect(h.events).toEqual(['window.hide', 'main.setManuallyHidden(true)']);

  // No window: every callback is inert.
  h.events.length = 0;
  h.main.window = null;
  for (const accelerator of MAIN_DEFAULTS) fire(accelerator);
  expect(h.events).toEqual([]);
});

test('task hotkey dispatch with modal open', async () => {
  const h = boot();
  const tasks: string[] = [];
  // Real overlayController routes the real windowActions->webContents.send channel to the application
  // boundary spies below (stand-ins for taskWorkflowController, which is NOT executed here).
  const overlay = createOverlayController({
    api: h.api as never,
    getSettings: () => h.controller.getSettings(),
    logger: { log() {} },
    onCompleteNextTask: () => { tasks.push('completeNextTask'); },
    onUndoLastTaskAction: () => { tasks.push('undoLastAction'); },
    onRedoLastTaskAction: () => { tasks.push('redoLastAction'); }
  });
  overlay.setupIpcListeners();
  h.open();
  expect(h.modalVisible()).toBe(true);
  h.events.length = 0;

  const [, , next, undo, redo] = MAIN_DEFAULTS;
  for (const accelerator of [next, undo, redo]) (stub.callbacks.get(accelerator) as () => void)();
  expect(h.events).toEqual([
    'webContents.send(complete-next-task)', 'webContents.send(undo-last-task-action)', 'webContents.send(redo-last-task-action)'
  ]);
  expect(tasks).toEqual(['completeNextTask', 'undoLastAction', 'redoLastAction']);
  expect(h.modalVisible()).toBe(true); // the open settings modal does not gate dispatch

  // While recording the stub has no registered chords (main unregistered them) => nothing can dispatch.
  await h.click('recordBtn1');
  expect(stub.callbacks.size).toBe(0);
  expect(tasks).toHaveLength(3);
});
