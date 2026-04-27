import type { HotkeySettings } from '../../../shared/types';
import { formatHotkeyForDisplay } from '../../ui/hotkeyDisplay';
import {
  getHotkeyRecordingResult,
  isHotkeyAction,
  type HotkeyAction
} from './settingsDomain';
import {
  beginHotkeyRecording,
  endHotkeyRecording,
  writeRecordedHotkey
} from './settingsUi';
import {
  ignoreAlert,
  type AlertHandler
} from '../../ui/dialogTypes';

export interface HotkeyRecorder {
  handleKeydown: (event: KeyboardEvent) => void;
  isRecording: () => boolean;
  record: (action: unknown) => void;
  stop: () => void;
}

export interface HotkeyRecorderOptions {
  onRecorded: (action: HotkeyAction, hotkey: string) => void;
  alertUser?: AlertHandler;
  getHotkeys?: () => HotkeySettings;
  onRecordingChanged?: (recording: boolean) => void;
}

export function createHotkeyRecorder(options: HotkeyRecorderOptions): HotkeyRecorder {
  const alertUser = options.alertUser || ignoreAlert;
  let recordingHotkey: HotkeyAction | null = null;
  let previousHotkeyDisplayValue: string | null = null;

  function stop(stopOptions: { restorePrevious?: boolean } = {}): void {
    if (!recordingHotkey) {
      return;
    }

    const restorePrevious = stopOptions.restorePrevious ?? true;
    const previousRecordingHotkey = recordingHotkey;
    const restoreValue = restorePrevious ? previousHotkeyDisplayValue ?? '' : undefined;
    recordingHotkey = null;
    previousHotkeyDisplayValue = null;
    endHotkeyRecording(previousRecordingHotkey, restoreValue);
    options.onRecordingChanged?.(false);
  }

  return {
    handleKeydown: (event: KeyboardEvent) => {
      if (!isHotkeyAction(recordingHotkey)) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      const hotkeyAction = recordingHotkey;
      const result = getHotkeyRecordingResult(event);
      if (!result) {
        return;
      }

      if (result.warning) {
        void alertUser(result.warning);
      }

      if (result.hotkey) {
        const duplicateAction = findDuplicateHotkeyAction(
          options.getHotkeys?.(),
          hotkeyAction,
          result.hotkey
        );
        if (duplicateAction) {
          void alertUser(`"${result.hotkey}" is already assigned to ${HOTKEY_ACTION_LABELS[duplicateAction]}. Choose a different shortcut.`);
          return;
        }

        writeRecordedHotkey(hotkeyAction, result.hotkey);
        options.onRecorded(hotkeyAction, result.hotkey);
        setTimeout(() => stop({ restorePrevious: false }), 200);
      }
    },
    isRecording: () => recordingHotkey !== null,
    record: (action: unknown) => {
      if (!isHotkeyAction(action)) {
        return;
      }

      if (recordingHotkey === action) {
        stop({ restorePrevious: true });
        return;
      }

      stop({ restorePrevious: true });
      recordingHotkey = action;
      previousHotkeyDisplayValue = beginHotkeyRecording(action);
      if (previousHotkeyDisplayValue === null) {
        recordingHotkey = null;
        return;
      }

      options.onRecordingChanged?.(true);
    },
    stop
  };
}

const HOTKEY_ACTION_LABELS: Record<HotkeyAction, string> = {
  toggleVisibility: 'Toggle Overlay Visibility',
  toggleInteractive: 'Toggle Interactive Mode',
  completeNextTask: 'Complete Next Task',
  undoLastAction: 'Undo Last Task Action',
  redoLastAction: 'Forward Last Task Action'
};

function findDuplicateHotkeyAction(
  hotkeys: HotkeySettings | undefined,
  currentAction: HotkeyAction,
  hotkey: string
): HotkeyAction | null {
  if (!hotkeys) {
    return null;
  }

  const normalizedHotkey = normalizeHotkeyForComparison(hotkey);
  const duplicate = Object.entries(hotkeys).find(([action, assignedHotkey]) =>
    action !== currentAction &&
    normalizeHotkeyForComparison(assignedHotkey) === normalizedHotkey
  );

  return duplicate && isHotkeyAction(duplicate[0]) ? duplicate[0] : null;
}

function normalizeHotkeyForComparison(hotkey: string): string {
  return formatHotkeyForDisplay(hotkey).trim().toLowerCase();
}
