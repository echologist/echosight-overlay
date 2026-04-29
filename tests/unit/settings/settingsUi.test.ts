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
    expect(getInput('themeSelect').value).toBe('echosight');
    expect(getElement('selectedThemeName').textContent).toBe('Echosight');
    expect(getElement('selectedThemeId').textContent).toBe('echosight');
    expect(getInput('transparencySlider').value).toBe('70');
    expect(getElement('transparencyValue').textContent).toBe('70% visible');
    expect(getInput('themeSoundsEnabled').checked).toBe(true);
    expect(getInput('themeSoundVolumeSlider').value).toBe('45');
    expect(getElement('themeSoundVolumeValue').textContent).toBe('45% volume');
    expect(getInput('undoLastActionHotkey').value).toBe('Ctrl+Shift+Z');
    expect(getInput('redoLastActionHotkey').value).toBe('Ctrl+Shift+Y');

    hideSettingsModal();
    expect(getElement('settingsModal').classList.contains('is-visible')).toBe(false);
    expect(getElement('settingsModal').getAttribute('aria-hidden')).toBe('true');
  });

  test('reads updated settings controls including forward hotkey', () => {
    renderSettingsModal(settingsFixture, themeFixtures);

    getInput('transparencySlider').value = '75';
    getInput('themeSelect').value = 'dark';
    getInput('themeSoundsEnabled').checked = false;
    getInput('themeSoundVolumeSlider').value = '30';
    getInput('redoLastActionHotkey').value = 'Alt+Y';

    expect(readSettingsControls(settingsFixture)).toEqual({
      transparency: '75',
      theme: 'dark',
      sounds: {
        enabled: false,
        volume: '30'
      },
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
  settingsVersion: 2,
  transparency: 70,
  theme: 'echosight',
  sounds: {
    enabled: true,
    volume: 45
  },
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
    <input id="themeSelect" type="hidden">
    <span id="selectedThemeName"></span>
    <span id="selectedThemeId"></span>
    <input id="themeSoundsEnabled" type="checkbox">
    <input id="themeSoundVolumeSlider" type="range">
    <span id="themeSoundVolumeValue"></span>
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
