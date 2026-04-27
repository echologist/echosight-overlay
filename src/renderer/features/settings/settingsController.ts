import type {
  EchosightApi,
  Settings,
  Theme
} from '../../../shared/types';
import { getErrorMessage } from '../../../shared/errors';
import { formatHotkeyForDisplay } from '../../ui/hotkeyDisplay';
import { applyRendererTheme } from '../themes/themeApplicator';
import { createHotkeyRecorder } from './hotkeyRecorder';
import type { HotkeyRecorder } from './hotkeyRecorder';
import {
  createDefaultHotkeys,
  createDefaultSettings,
  normalizeSettings,
  normalizeTransparency
} from './settingsDomain';
import {
  hideSettingsModal,
  readSelectedThemeId,
  readSettingsControls,
  renderSettingsModal,
  renderThemeSelector,
  setTransparencyControlsEnabled,
  setTransparencyDisplay,
  writeHotkeyControls
} from './settingsUi';
import {
  loadSettingsState,
  saveSettingsState
} from './settingsPersistence';
import {
  denyConfirm,
  ignoreAlert,
  type AlertHandler,
  type ConfirmHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log' | 'warn' | 'error'>;

export interface SettingsControllerOptions {
  api: EchosightApi;
  alertUser?: AlertHandler;
  confirmUser?: ConfirmHandler;
  getIsInteractiveMode: () => boolean;
  getThemes: () => Theme[];
  logger?: LogSink;
  onInteractiveRefresh: () => void;
}

export interface SettingsController {
  applySelectedTheme: () => Promise<void>;
  applyTheme: () => Promise<void>;
  closeSettingsModal: () => void;
  getSettings: () => Settings;
  hotkeyRecorder: Pick<HotkeyRecorder, 'handleKeydown' | 'isRecording' | 'record' | 'stop'>;
  loadSettings: () => Promise<void>;
  resetHotkeys: () => Promise<void>;
  saveSettings: () => Promise<void>;
  showSettingsModal: () => void;
  updateTheme: (themeId: string) => Promise<void>;
  updateThemeSelector: () => void;
  updateTransparency: (value: unknown) => void;
}

export function createSettingsController(options: SettingsControllerOptions): SettingsController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  const confirmUser = options.confirmUser || denyConfirm;
  let settings = createDefaultSettings();

  const hotkeyRecorder = createHotkeyRecorder({
    onRecorded: (action, hotkey) => {
      settings = {
        ...settings,
        hotkeys: {
          ...settings.hotkeys,
          [action]: hotkey
        }
      };
    },
    getHotkeys: () => settings.hotkeys,
    onRecordingChanged: recording => options.api.setHotkeyRecording(recording),
    alertUser
  });

  function getSettings(): Settings {
    return settings;
  }

  async function resetHotkeys(): Promise<void> {
    const defaultHotkeys = createDefaultHotkeys();
    const defaultHotkeyLabels = [
      defaultHotkeys.toggleVisibility,
      defaultHotkeys.toggleInteractive,
      defaultHotkeys.completeNextTask,
      defaultHotkeys.undoLastAction,
      defaultHotkeys.redoLastAction
    ].map(formatHotkeyForDisplay);

    if (!await confirmUser(`Reset hotkeys to default values (${defaultHotkeyLabels.join(', ')})?`, {
      title: 'Reset Hotkeys',
      tone: 'danger'
    })) {
      return;
    }

    settings = {
      ...settings,
      hotkeys: defaultHotkeys
    };
    writeHotkeyControls(settings.hotkeys);
    await alertUser('Hotkeys reset to defaults. Click "Save Settings" to apply.');
  }

  function showSettingsModal(): void {
    renderSettingsModal(settings, options.getThemes());
  }

  function updateThemeSelector(): void {
    renderThemeSelector(options.getThemes(), settings.theme);
  }

  function closeSettingsModal(): void {
    hotkeyRecorder.stop();
    hideSettingsModal();
  }

  function updateTransparencyControls(supportsTransparency: boolean): void {
    setTransparencyControlsEnabled(supportsTransparency, settings.transparency);
  }

  function updateTransparency(value: unknown): void {
    const transparency = normalizeTransparency(value, settings.transparency);
    setTransparencyDisplay(transparency);

    settings = {
      ...settings,
      transparency
    };

    document.documentElement.style.setProperty('--user-transparency', String(transparency / 100));
    void applyTheme();
  }

  async function updateTheme(themeId: string): Promise<void> {
    logger.log('Theme changed to:', themeId);
    settings = {
      ...settings,
      theme: themeId
    };
    await applyTheme();

    if (options.getIsInteractiveMode()) {
      options.onInteractiveRefresh();
    }
  }

  async function applySelectedTheme(): Promise<void> {
    const currentTheme = readSelectedThemeId(settings.theme);
    if (!currentTheme) {
      return;
    }

    settings = {
      ...settings,
      theme: currentTheme
    };
    await applyTheme();
  }

  async function applyTheme(): Promise<void> {
    try {
      const theme = await applyRendererTheme({
        themeId: settings.theme || 'echosight',
        transparency: settings.transparency,
        api: options.api,
        logger
      });

      if (theme) {
        updateTransparencyControls(theme.supportsTransparency !== false);
      }
    } catch (error) {
      logger.error('Failed to apply theme:', error);
    }
  }

  async function saveSettings(): Promise<void> {
    try {
      logger.log('Saving settings...');

      if (hotkeyRecorder.isRecording()) {
        hotkeyRecorder.stop();
      }

      settings = normalizeSettings(readSettingsControls(settings), settings);
      await saveSettingsState(options.api, settings, logger);

      closeSettingsModal();
      await alertUser('Settings saved successfully! Hotkey changes are active now.');
    } catch (error) {
      logger.error('Error saving settings:', error);
      await alertUser(`Error saving settings: ${getErrorMessage(error)}. Please try again.`);
    }
  }

  async function loadSettings(): Promise<void> {
    settings = await loadSettingsState(options.api, settings, logger);
  }

  return {
    applySelectedTheme,
    applyTheme,
    closeSettingsModal,
    getSettings,
    hotkeyRecorder,
    loadSettings,
    resetHotkeys,
    saveSettings,
    showSettingsModal,
    updateTheme,
    updateThemeSelector,
    updateTransparency
  };
}
