import type { HotkeySettings, Settings, Theme } from '../../../shared/types';
import type { HotkeyAction } from './settingsDomain';
import { formatHotkeyForDisplay } from '../../ui/hotkeyDisplay';
import { hideModal, showModal } from '../../ui/modalUi';

const HOTKEY_INPUT_IDS: Record<HotkeyAction, string> = {
  toggleVisibility: 'toggleVisibilityHotkey',
  toggleInteractive: 'toggleInteractiveHotkey',
  completeNextTask: 'completeNextTaskHotkey',
  undoLastAction: 'undoLastActionHotkey',
  redoLastAction: 'redoLastActionHotkey'
};

const HOTKEY_RECORD_BUTTON_IDS: Record<HotkeyAction, string> = {
  toggleVisibility: 'recordBtn1',
  toggleInteractive: 'recordBtn2',
  completeNextTask: 'recordBtn3',
  undoLastAction: 'recordBtn4',
  redoLastAction: 'recordBtn5'
};

export interface SettingsControlValues {
  transparency: string;
  theme: string;
  hotkeys: HotkeySettings;
}

export function renderSettingsModal(settings: Settings, themes: Theme[]): void {
  showModal('settingsModal');
  renderThemeSelector(themes, settings.theme);
  writeSettingsControls(settings);
}

export function hideSettingsModal(): void {
  hideModal('settingsModal');
}

export function renderThemeSelector(themes: Theme[], selectedThemeId: string): void {
  const select = getSelectElement('themeSelect');
  if (!select) {
    return;
  }

  select.replaceChildren();
  themes.forEach(theme => {
    const option = document.createElement('option');
    option.value = theme.id;
    option.textContent = theme.name;
    if (theme.description) {
      option.title = theme.description;
    }
    select.appendChild(option);
  });
  select.value = selectedThemeId;
}

export function writeSettingsControls(settings: Settings): void {
  setInputValue('transparencySlider', String(settings.transparency));
  setTextContent('transparencyValue', formatTransparencyLabel(settings.transparency));
  setInputValue('themeSelect', settings.theme);
  writeHotkeyControls(settings.hotkeys);
}

export function readSettingsControls(fallback: Settings): SettingsControlValues {
  return {
    transparency: readInputValue('transparencySlider', String(fallback.transparency)),
    theme: readInputValue('themeSelect', fallback.theme),
    hotkeys: {
      toggleVisibility: readInputValue(HOTKEY_INPUT_IDS.toggleVisibility, fallback.hotkeys.toggleVisibility),
      toggleInteractive: readInputValue(HOTKEY_INPUT_IDS.toggleInteractive, fallback.hotkeys.toggleInteractive),
      completeNextTask: readInputValue(HOTKEY_INPUT_IDS.completeNextTask, fallback.hotkeys.completeNextTask),
      undoLastAction: readInputValue(HOTKEY_INPUT_IDS.undoLastAction, fallback.hotkeys.undoLastAction),
      redoLastAction: readInputValue(HOTKEY_INPUT_IDS.redoLastAction, fallback.hotkeys.redoLastAction)
    }
  };
}

export function readSelectedThemeId(fallback = ''): string {
  return readInputValue('themeSelect', fallback);
}

export function writeHotkeyControls(hotkeys: HotkeySettings): void {
  setInputValue(HOTKEY_INPUT_IDS.toggleVisibility, formatHotkeyForDisplay(hotkeys.toggleVisibility));
  setInputValue(HOTKEY_INPUT_IDS.toggleInteractive, formatHotkeyForDisplay(hotkeys.toggleInteractive));
  setInputValue(HOTKEY_INPUT_IDS.completeNextTask, formatHotkeyForDisplay(hotkeys.completeNextTask));
  setInputValue(HOTKEY_INPUT_IDS.undoLastAction, formatHotkeyForDisplay(hotkeys.undoLastAction));
  setInputValue(HOTKEY_INPUT_IDS.redoLastAction, formatHotkeyForDisplay(hotkeys.redoLastAction));
}

export function setTransparencyDisplay(transparency: number): void {
  setTextContent('transparencyValue', formatTransparencyLabel(transparency));
}

export function setTransparencyControlsEnabled(supportsTransparency: boolean, transparency: number): void {
  const slider = getInputElement('transparencySlider');
  const label = document.querySelector<HTMLElement>('label[for="transparencySlider"]');
  const valueDisplay = document.getElementById('transparencyValue');

  if (slider) {
    slider.disabled = !supportsTransparency;
    slider.classList.toggle('transparency-control-disabled', !supportsTransparency);
  }

  if (label) {
    label.classList.toggle('transparency-control-disabled', !supportsTransparency);
  }

  if (valueDisplay) {
    valueDisplay.classList.toggle('transparency-control-disabled', !supportsTransparency);
    valueDisplay.textContent = supportsTransparency
      ? formatTransparencyLabel(transparency)
      : 'Not supported by this theme';
  }
}

export function beginHotkeyRecording(action: HotkeyAction): string | null {
  const input = getInputElement(HOTKEY_INPUT_IDS[action]);
  const button = getButtonElement(HOTKEY_RECORD_BUTTON_IDS[action]);
  if (!input || !button) {
    return null;
  }

  const previousValue = input.value;
  input.value = 'Press keys now...';
  input.classList.add('hotkey-recording-input');
  button.textContent = 'Cancel';
  button.classList.add('hotkey-recording-button');
  button.disabled = false;
  input.focus();
  return previousValue;
}

export function endHotkeyRecording(
  action: HotkeyAction | null | undefined,
  restoreValue?: string
): void {
  if (!action) {
    return;
  }

  const input = getInputElement(HOTKEY_INPUT_IDS[action]);
  const button = getButtonElement(HOTKEY_RECORD_BUTTON_IDS[action]);

  if (input) {
    if (restoreValue !== undefined) {
      input.value = restoreValue;
    }
    input.classList.remove('hotkey-recording-input');
  }
  if (button) {
    button.textContent = 'Record';
    button.classList.remove('hotkey-recording-button');
    button.disabled = false;
  }
}

export function writeRecordedHotkey(action: HotkeyAction, hotkey: string): void {
  setInputValue(HOTKEY_INPUT_IDS[action], hotkey);
}

function formatTransparencyLabel(transparency: number): string {
  return `${transparency}% visible`;
}

function readInputValue(id: string, fallback = ''): string {
  const input = getInputElement(id) || getSelectElement(id);
  return input?.value ?? fallback;
}

function setInputValue(id: string, value: string): void {
  const input = getInputElement(id) || getSelectElement(id);
  if (input) {
    input.value = value;
  }
}

function setTextContent(id: string, value: string): void {
  const element = document.getElementById(id);
  if (element) {
    element.textContent = value;
  }
}

function getInputElement(id: string): HTMLInputElement | null {
  const element = document.getElementById(id);
  return element instanceof HTMLInputElement ? element : null;
}

function getSelectElement(id: string): HTMLSelectElement | null {
  const element = document.getElementById(id);
  return element instanceof HTMLSelectElement ? element : null;
}

function getButtonElement(id: string): HTMLButtonElement | null {
  const element = document.getElementById(id);
  return element instanceof HTMLButtonElement ? element : null;
}
