import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import type {
  Settings,
  Theme
} from '../../../src/shared/types';
import {
  hideSettingsModal,
  readSelectedThemeId,
  readSettingsControls,
  renderSettingsModal,
  setTransparencyControlsEnabled,
  writeHotkeyControls
} from '../../../src/renderer/features/settings/settingsUi';

describe('settings UI', () => {
  beforeEach(() => {
    renderSettingsFixture();
  });

  afterEach(() => {
    document.body.replaceChildren();
  });

  test('renders theme options and all hotkey controls', () => {
    renderSettingsModal(settingsFixture, themeFixtures);

    expect(getElement('settingsModal').classList.contains('is-visible')).toBe(true);
    expect(getElement('settingsModal').getAttribute('aria-hidden')).toBe('false');
    expect(getSelect('themeSelect').value).toBe('echosight');
    expect([...getSelect('themeSelect').options].map(option => option.textContent))
      .toEqual(['Dark', 'Echosight']);
    expect(getInput('transparencySlider').value).toBe('70');
    expect(getElement('transparencyValue').textContent).toBe('70% visible');
    expect(getInput('undoLastActionHotkey').value).toBe('Ctrl+Shift+Z');
    expect(getInput('redoLastActionHotkey').value).toBe('Ctrl+Shift+Y');

    hideSettingsModal();
    expect(getElement('settingsModal').classList.contains('is-visible')).toBe(false);
    expect(getElement('settingsModal').getAttribute('aria-hidden')).toBe('true');
  });

  test('reads updated settings controls including forward hotkey', () => {
    renderSettingsModal(settingsFixture, themeFixtures);

    getInput('transparencySlider').value = '75';
    getSelect('themeSelect').value = 'dark';
    getInput('redoLastActionHotkey').value = 'Alt+Y';

    expect(readSettingsControls(settingsFixture)).toEqual({
      transparency: '75',
      theme: 'dark',
      hotkeys: {
        toggleVisibility: 'Ctrl+Shift+T',
        toggleInteractive: 'Ctrl+Shift+I',
        completeNextTask: 'Ctrl+Shift+N',
        undoLastAction: 'Ctrl+Shift+Z',
        redoLastAction: 'Alt+Y'
      }
    });
    expect(readSelectedThemeId('fallback')).toBe('dark');
  });

  test('formats Electron hotkeys for display', () => {
    writeHotkeyControls({
      toggleVisibility: 'CommandOrControl+Shift+T',
      toggleInteractive: 'CommandOrControl+Shift+I',
      completeNextTask: 'CommandOrControl+Shift+N',
      undoLastAction: 'CommandOrControl+Shift+Z',
      redoLastAction: 'CommandOrControl+Shift+Y'
    });

    expect(getInput('redoLastActionHotkey').value).toBe('Ctrl+Shift+Y');
  });

  test('disables transparency controls for opaque themes', () => {
    setTransparencyControlsEnabled(false, 60);

    expect(getInput('transparencySlider').disabled).toBe(true);
    expect(getInput('transparencySlider').classList.contains('transparency-control-disabled')).toBe(true);
    expect(getElement('transparencyValue').classList.contains('transparency-control-disabled')).toBe(true);
    expect(getElement('transparencyValue').textContent).toBe('Not supported by this theme');

    setTransparencyControlsEnabled(true, 40);

    expect(getInput('transparencySlider').disabled).toBe(false);
    expect(getInput('transparencySlider').classList.contains('transparency-control-disabled')).toBe(false);
    expect(getElement('transparencyValue').classList.contains('transparency-control-disabled')).toBe(false);
    expect(getElement('transparencyValue').textContent).toBe('40% visible');
  });
});

const settingsFixture: Settings = {
  transparency: 70,
  theme: 'echosight',
  hotkeys: {
    toggleVisibility: 'Ctrl+Shift+T',
    toggleInteractive: 'Ctrl+Shift+I',
    completeNextTask: 'Ctrl+Shift+N',
    undoLastAction: 'Ctrl+Shift+Z',
    redoLastAction: 'Ctrl+Shift+Y'
  }
};

const themeFixtures: Theme[] = [
  {
    id: 'dark',
    name: 'Dark'
  },
  {
    id: 'echosight',
    name: 'Echosight',
    description: 'Default fantasy theme'
  }
];

function renderSettingsFixture(): void {
  document.body.innerHTML = `
    <div id="settingsModal"></div>
    <label for="transparencySlider">Transparency</label>
    <input id="transparencySlider" type="range">
    <div id="transparencyValue"></div>
    <select id="themeSelect"></select>
    <input id="toggleVisibilityHotkey">
    <input id="toggleInteractiveHotkey">
    <input id="completeNextTaskHotkey">
    <input id="undoLastActionHotkey">
    <input id="redoLastActionHotkey">
  `;
}

function getElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element ${id}`);
  }
  return element;
}

function getInput(id: string): HTMLInputElement {
  const element = getElement(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`Expected input ${id}`);
  }
  return element;
}

function getSelect(id: string): HTMLSelectElement {
  const element = getElement(id);
  if (!(element instanceof HTMLSelectElement)) {
    throw new Error(`Expected select ${id}`);
  }
  return element;
}
