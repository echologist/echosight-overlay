import type {
  EchosightApi,
  Settings
} from '../../shared/types';
import { renderInteractiveMode } from './overlayUi';

type LogSink = Pick<Console, 'log'>;

export interface OverlayControllerOptions {
  api: Pick<
    EchosightApi,
    'onCompleteNextTask' | 'onInteractiveModeChanged' | 'onRedoLastTaskAction' | 'onUndoLastTaskAction'
  >;
  getSettings: () => Settings;
  logger?: LogSink;
  onCompleteNextTask: () => void;
  onRedoLastTaskAction: () => void;
  onUndoLastTaskAction: () => void;
}

export interface OverlayController {
  isInteractive: () => boolean;
  refreshInteractiveVisuals: (interactive?: boolean) => void;
  setupIpcListeners: () => void;
}

export function createOverlayController(options: OverlayControllerOptions): OverlayController {
  const logger = options.logger || console;
  let isInteractiveMode = true;

  function refreshInteractiveVisuals(interactive = isInteractiveMode): void {
    const settings = options.getSettings();
    renderInteractiveMode(interactive, settings.theme, settings.hotkeys.toggleInteractive);
  }

  function setupIpcListeners(): void {
    options.api.onInteractiveModeChanged(interactive => {
      logger.log('Interactive mode changed:', interactive);
      isInteractiveMode = interactive;
      refreshInteractiveVisuals(interactive);
    });

    options.api.onCompleteNextTask(() => {
      logger.log('Complete next task triggered');
      options.onCompleteNextTask();
    });

    options.api.onUndoLastTaskAction(() => {
      logger.log('Undo last task action triggered');
      options.onUndoLastTaskAction();
    });

    options.api.onRedoLastTaskAction(() => {
      logger.log('Forward last task action triggered');
      options.onRedoLastTaskAction();
    });
  }

  return {
    isInteractive: () => isInteractiveMode,
    refreshInteractiveVisuals,
    setupIpcListeners
  };
}
