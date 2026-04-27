import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { HotkeySettings } from '../../../src/shared/types';
import type { HotkeyAction } from '../../../src/renderer/features/settings/settingsDomain';
import { createHotkeyRecorder } from '../../../src/renderer/features/settings/hotkeyRecorder';

describe('hotkey recorder', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    renderHotkeyFixture();
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  test('records forward hotkey combinations into the matching control', () => {
    const recorded: Array<{ action: HotkeyAction; hotkey: string }> = [];
    const recordingStates: boolean[] = [];
    const recorder = createHotkeyRecorder({
      onRecorded: (action, hotkey) => recorded.push({ action, hotkey }),
      onRecordingChanged: recording => recordingStates.push(recording)
    });

    recorder.record('redoLastAction');
    expect(recordingStates).toEqual([true]);
    expect(getInput('redoLastActionHotkey').value).toBe('Press keys now...');
    expect(getInput('redoLastActionHotkey').classList.contains('hotkey-recording-input')).toBe(true);
    expect(getButton('recordBtn5').classList.contains('hotkey-recording-button')).toBe(true);
    expect(getButton('recordBtn5').textContent).toBe('Cancel');
    expect(getButton('recordBtn5').disabled).toBe(false);
    expect(recorder.isRecording()).toBe(true);

    recorder.handleKeydown(new KeyboardEvent('keydown', {
      key: 'y',
      ctrlKey: true,
      shiftKey: true
    }));

    expect(recorded).toEqual([{ action: 'redoLastAction', hotkey: 'Ctrl+Shift+Y' }]);
    expect(getInput('redoLastActionHotkey').value).toBe('Ctrl+Shift+Y');

    vi.runAllTimers();
    expect(getButton('recordBtn5').disabled).toBe(false);
    expect(getInput('redoLastActionHotkey').classList.contains('hotkey-recording-input')).toBe(false);
    expect(getButton('recordBtn5').classList.contains('hotkey-recording-button')).toBe(false);
    expect(getButton('recordBtn5').textContent).toBe('Record');
    expect(recorder.isRecording()).toBe(false);
    expect(recordingStates).toEqual([true, false]);
  });

  test('clicking the active record button cancels and restores the previous value', () => {
    const recordingStates: boolean[] = [];
    const recorder = createHotkeyRecorder({
      onRecorded: () => undefined,
      onRecordingChanged: recording => recordingStates.push(recording)
    });

    recorder.record('redoLastAction');
    recorder.record('redoLastAction');

    expect(getInput('redoLastActionHotkey').value).toBe('Ctrl+Shift+Y');
    expect(getInput('redoLastActionHotkey').classList.contains('hotkey-recording-input')).toBe(false);
    expect(getButton('recordBtn5').textContent).toBe('Record');
    expect(getButton('recordBtn5').disabled).toBe(false);
    expect(recorder.isRecording()).toBe(false);
    expect(recordingStates).toEqual([true, false]);
  });

  test('switching record buttons stops the previous recording state', () => {
    const recordingStates: boolean[] = [];
    const recorder = createHotkeyRecorder({
      onRecorded: () => undefined,
      onRecordingChanged: recording => recordingStates.push(recording)
    });

    recorder.record('undoLastAction');
    recorder.record('redoLastAction');

    expect(getInput('undoLastActionHotkey').classList.contains('hotkey-recording-input')).toBe(false);
    expect(getInput('undoLastActionHotkey').value).toBe('Ctrl+Shift+Z');
    expect(getButton('recordBtn4').disabled).toBe(false);
    expect(getInput('redoLastActionHotkey').classList.contains('hotkey-recording-input')).toBe(true);
    expect(getButton('recordBtn5').textContent).toBe('Cancel');
    expect(getButton('recordBtn5').disabled).toBe(false);
    expect(recordingStates).toEqual([true, false, true]);
  });

  test('rejects hotkeys already assigned to another action', () => {
    const recorded: Array<{ action: HotkeyAction; hotkey: string }> = [];
    const alerts: string[] = [];
    const recorder = createHotkeyRecorder({
      getHotkeys: () => hotkeySettings,
      onRecorded: (action, hotkey) => recorded.push({ action, hotkey }),
      alertUser: message => {
        alerts.push(message);
      }
    });

    recorder.record('redoLastAction');
    recorder.handleKeydown(new KeyboardEvent('keydown', {
      key: 'z',
      ctrlKey: true,
      shiftKey: true
    }));

    expect(recorded).toEqual([]);
    expect(alerts).toEqual([
      '"Ctrl+Shift+Z" is already assigned to Undo Last Task Action. Choose a different shortcut.'
    ]);
    expect(getInput('redoLastActionHotkey').value).toBe('Press keys now...');
    expect(recorder.isRecording()).toBe(true);

    recorder.handleKeydown(new KeyboardEvent('keydown', {
      key: 'F8'
    }));

    expect(recorded).toEqual([{ action: 'redoLastAction', hotkey: 'F8' }]);
    expect(getInput('redoLastActionHotkey').value).toBe('F8');
  });

  test('ignores unknown record actions', () => {
    const recorder = createHotkeyRecorder({
      onRecorded: () => {
        throw new Error('should not record');
      }
    });

    recorder.record('unknown-action');

    expect(recorder.isRecording()).toBe(false);
    expect(getInput('redoLastActionHotkey').value).toBe('Ctrl+Shift+Y');
  });
});

function renderHotkeyFixture(): void {
  document.body.innerHTML = `
    <input id="toggleVisibilityHotkey" value="Ctrl+Shift+T">
    <button id="recordBtn1">Record</button>
    <input id="toggleInteractiveHotkey" value="Ctrl+Shift+I">
    <button id="recordBtn2">Record</button>
    <input id="completeNextTaskHotkey" value="Ctrl+Shift+N">
    <button id="recordBtn3">Record</button>
    <input id="undoLastActionHotkey" value="Ctrl+Shift+Z">
    <button id="recordBtn4">Record</button>
    <input id="redoLastActionHotkey" value="Ctrl+Shift+Y">
    <button id="recordBtn5">Record</button>
  `;
}

const hotkeySettings: HotkeySettings = {
  toggleVisibility: 'Ctrl+Shift+T',
  toggleInteractive: 'Ctrl+Shift+I',
  completeNextTask: 'Ctrl+Shift+N',
  undoLastAction: 'Ctrl+Shift+Z',
  redoLastAction: 'Ctrl+Shift+Y'
};

function getInput(id: string): HTMLInputElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`Missing input ${id}`);
  }
  return element;
}

function getButton(id: string): HTMLButtonElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLButtonElement)) {
    throw new Error(`Missing button ${id}`);
  }
  return element;
}
